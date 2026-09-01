// GTP 응답 / kata-analyze 스트리밍 출력의 line-by-line 파서.
// "info move ... visits ... winrate ... pv ..." 형식을 파싱해서
// Tauri event("kata-analyze")로 emit할 수 있는 구조체로 변환한다.
//
// 프론트 `src/lib/types/gtp.ts`의 KataAnalyzeResult/KataAnalyzeMove와 1:1 대응.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KataAnalyzeMove {
    pub r#move: String,
    pub visits: u32,
    pub winrate: f64,
    pub score_lead: f64,
    pub pv: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KataAnalyzeResult {
    pub move_number: u32,
    pub candidates: Vec<KataAnalyzeMove>,
}

/// kata-analyze 스트리밍 라인인지 판별. GTP 정규 응답은 항상 `=`/`?`로 시작하므로
/// `info `로 시작하는 줄은 스트리밍 출력으로만 나타난다.
pub fn is_analysis_line(line: &str) -> bool {
    line.starts_with("info ")
}

/// `info move Q16 visits 123 winrate 0.5432 scoreLead 1.23 ... pv Q16 D4 ... info move
/// D4 ...` 형태로 후보 수들을 이어붙인 한 줄을 후보 목록으로 파싱.
///
/// `moveNumber`는 원본 라인에 없는 정보라 이번 Phase에서는 항상 0으로 채운다 — 로컬
/// 게임 트리의 착수 수와 맞물리는 건 board 자동 mirroring을 붙일 때(Phase 3) 채워 넣을
/// 지점.
pub fn parse_kata_analyze(line: &str) -> Option<KataAnalyzeResult> {
    let candidates: Vec<KataAnalyzeMove> = split_move_blocks(line)
        .iter()
        .filter_map(|block| parse_move_block(block))
        .collect();

    if candidates.is_empty() {
        return None;
    }

    Some(KataAnalyzeResult {
        move_number: 0,
        candidates,
    })
}

/// "info move A ... info move B ..." 한 줄을 "move A ..." / "move B ..." 블록들로 분리.
fn split_move_blocks(line: &str) -> Vec<String> {
    line.split("info ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// "move A visits N winrate F scoreLead F ... pv A B C" 한 블록을 파싱. 알 수 없는
/// key(ownership/scoreStdev/prior/lcb/order 등, KataGo 버전별로 다를 수 있음)는 값 하나만
/// 건너뛰고 무시 — 파싱 자체가 깨지지 않도록 관대하게 처리.
fn parse_move_block(block: &str) -> Option<KataAnalyzeMove> {
    let tokens: Vec<&str> = block.split_whitespace().collect();
    let mut idx = 0;

    let mut mv: Option<String> = None;
    let mut visits = 0u32;
    let mut winrate = 0.0f64;
    let mut score_lead = 0.0f64;
    let mut pv: Vec<String> = Vec::new();

    while idx < tokens.len() {
        match tokens[idx] {
            "move" if idx + 1 < tokens.len() => {
                mv = Some(tokens[idx + 1].to_string());
                idx += 2;
            }
            "visits" if idx + 1 < tokens.len() => {
                visits = tokens[idx + 1].parse().unwrap_or(0);
                idx += 2;
            }
            "winrate" if idx + 1 < tokens.len() => {
                winrate = tokens[idx + 1].parse().unwrap_or(0.0);
                idx += 2;
            }
            "scoreLead" if idx + 1 < tokens.len() => {
                score_lead = tokens[idx + 1].parse().unwrap_or(0.0);
                idx += 2;
            }
            "pv" => {
                // pv는 이 블록 끝까지 이어지는 좌표 목록 — 관대하게 나머지 전부 소비.
                pv = tokens[idx + 1..].iter().map(|s| s.to_string()).collect();
                break;
            }
            _ => {
                idx += 2;
            }
        }
    }

    Some(KataAnalyzeMove {
        r#move: mv?,
        visits,
        winrate,
        score_lead,
        pv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_candidate() {
        let line = "info move Q16 visits 123 winrate 0.5432 scoreLead 1.23 pv Q16 D4 Q3";
        let result = parse_kata_analyze(line).expect("should parse");
        assert_eq!(result.candidates.len(), 1);
        let c = &result.candidates[0];
        assert_eq!(c.r#move, "Q16");
        assert_eq!(c.visits, 123);
        assert!((c.winrate - 0.5432).abs() < 1e-9);
        assert!((c.score_lead - 1.23).abs() < 1e-9);
        assert_eq!(c.pv, vec!["Q16", "D4", "Q3"]);
    }

    #[test]
    fn parses_multiple_candidates_and_ignores_unknown_fields() {
        let line = "info move Q16 visits 123 winrate 0.54 scoreLead 1.2 scoreStdev 4.5 prior 0.02 lcb 0.51 order 0 pv Q16 D4 info move D4 visits 90 winrate 0.49 scoreLead 0.9 pv D4 Q16";
        let result = parse_kata_analyze(line).expect("should parse");
        assert_eq!(result.candidates.len(), 2);
        assert_eq!(result.candidates[0].r#move, "Q16");
        assert_eq!(result.candidates[1].r#move, "D4");
        assert_eq!(result.candidates[1].pv, vec!["D4", "Q16"]);
    }

    #[test]
    fn non_analysis_line_is_not_detected() {
        assert!(!is_analysis_line("= KataGo 1.13.0"));
        assert!(is_analysis_line(
            "info move Q16 visits 1 winrate 0.5 pv Q16"
        ));
    }
}
