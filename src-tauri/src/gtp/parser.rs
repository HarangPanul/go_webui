// GTP 응답 / kata-analyze 스트리밍 출력의 line-by-line 파서.
// "info move ... visits ... winrate ... pv ..." 형식을 파싱해서
// Tauri event("kata-analyze")로 emit할 수 있는 구조체로 변환한다.
//
// 프론트 `src/lib/types/gtp.ts`의 KataAnalyzeResult/KataAnalyzeMove와 1:1 대응.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KataAnalyzeMove {
    pub r#move: String,
    pub visits: u32,
    pub winrate: f64,
    pub score_lead: f64,
    pub pv: Vec<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KataAnalyzeResult {
    pub move_number: u32,
    pub candidates: Vec<KataAnalyzeMove>,
    /// `kata-analyze ... ownership true`로 요청했을 때만 채워짐(그 외엔 None).
    /// 보드 전체 지점을 한 줄로 이어붙인 값(row-major, KataGo가 showboard를 출력하는
    /// 순서와 동일 - 즉 위쪽 줄부터, 각 줄은 왼쪽부터). 값은 [-1, 1] 범위로, 이 줄의
    /// winrate/scoreLead와 같은 기준(분석 요청 당시 둘 차례였던 색)으로 1에 가까울수록
    /// 그 색 소유, -1에 가까울수록 상대 소유를 의미.
    pub ownership: Option<Vec<f64>>,
}

/// kata-analyze 스트리밍 라인인지 판별. GTP 정규 응답은 항상 `=`/`?`로 시작하므로
/// `info `로 시작하는 줄은 스트리밍 출력으로만 나타난다.
pub fn is_analysis_line(line: &str) -> bool {
    line.starts_with("info ")
}

/// `info move Q16 visits 123 winrate 0.5432 scoreLead 1.23 ... pv Q16 D4 ... info move
/// D4 ...` 형태로 후보 수들을 이어붙인 한 줄을 후보 목록으로 파싱.
///
/// `moveNumber`는 원본 GTP 응답에 없는 정보라 항상 0으로 채운다(현재 프런트가 이
/// 필드를 쓰지 않음 - 로컬 게임 트리의 착수 수와 맞물리려면 별도로 채워 넣어야 함).
pub fn parse_kata_analyze(line: &str) -> Option<KataAnalyzeResult> {
    let mut candidates: Vec<KataAnalyzeMove> = Vec::new();
    // 마지막 move 블록에서만 의미 있음 - "ownership ..." 같은 줄 전체 단위 트레일링
    // 필드는 맨 마지막 pv 뒤에만 붙어 나오므로(parse_move_block 참고).
    let mut trailing: Vec<String> = Vec::new();

    for block in split_move_blocks(line) {
        if let Some((mv, leftover)) = parse_move_block(&block) {
            candidates.push(mv);
            trailing = leftover;
        }
    }

    if candidates.is_empty() {
        return None;
    }

    Some(KataAnalyzeResult {
        move_number: 0,
        candidates,
        ownership: parse_ownership(&trailing),
    })
}

/// "ownership <o0> <o1> ..." 트레일링 필드 파싱. "ownership" 키워드를 찾아 그 뒤
/// 숫자로 파싱되는 토큰들을 끝까지(혹은 파싱 실패 지점까지) 모은다.
fn parse_ownership(trailing: &[String]) -> Option<Vec<f64>> {
    let pos = trailing.iter().position(|t| t == "ownership")?;
    let values: Vec<f64> = trailing[pos + 1..]
        .iter()
        .map_while(|t| t.parse::<f64>().ok())
        .collect();
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

/// GTP vertex처럼 보이는 토큰인지 판별(pv 좌표 목록을 그 뒤에 이어붙는 다른
/// 필드(ownership 등)와 구분하는 데 사용). "pass"/"resign" 또는 "글자+숫자"
/// (예: "Q16") 형태만 인정 - ownership 값 같은 부동소수점 토큰("-0.3" 등)은
/// 숫자로 시작하므로 걸러진다.
fn looks_like_vertex(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    if lower == "pass" || lower == "resign" {
        return true;
    }
    let mut chars = token.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    let rest = chars.as_str();
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// "info move A ... info move B ..." 한 줄을 "move A ..." / "move B ..." 블록들로 분리.
fn split_move_blocks(line: &str) -> Vec<String> {
    line.split("info ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// 이 블록 파서가 이름으로 알아보는 key들 - 알 수 없는 key를 건너뛸 때 "다음
/// 알려진 key가 나올 때까지"의 경계로도 쓰인다([parse_move_block] 참고).
const KNOWN_BLOCK_KEYS: &[&str] = &["move", "visits", "winrate", "scoreLead", "pv"];

/// "move A visits N winrate F scoreLead F ... pv A B C" 한 블록을 파싱. 알 수 없는
/// key(scoreStdev/prior/lcb/order 등, KataGo 버전별로 다를 수 있음)는 무시하고
/// 건너뛴다 — 파싱 자체가 깨지지 않도록 관대하게 처리.
///
/// 건너뛸 때 "값 토큰 정확히 하나"를 가정하지 않는다 - 대신 다음으로 [KNOWN_BLOCK_KEYS]에
/// 속한 토큰이 나올 때까지 계속 건너뛴다. 지금 우리가 실제로 받는 필드(scoreStdev/prior/
/// lcb/order 등)는 전부 값 하나짜리지만, 그걸 하드코딩해서 가정하면 나중에 값이 없는
/// 플래그성 필드나 여러 토큰짜리 리스트 필드(KataGo의 pvVisits/pvEdgeVisits 등, 지금은
/// 요청하지 않지만 옵션이 늘어나면 나타날 수 있음)가 섞였을 때 그 뒤 필드 전부가
/// 밀려서 잘못 파싱된다 - "다음 알려진 key까지 건너뛰기"는 값 토큰이 0개든 여러 개든
/// 항상 안전하다.
///
/// pv 뒤에 좌표로 보이지 않는 토큰이 남아있으면(마지막 move 블록에만 발생 -
/// "ownership true"로 요청했을 때 줄 맨 끝에 붙는 "ownership <값...>" 같은 줄
/// 전체 단위 필드) 소비하지 않고 그대로 두 번째 반환값(leftover)으로 돌려준다 -
/// 호출자(parse_kata_analyze)가 그걸로 ownership을 파싱함.
fn parse_move_block(block: &str) -> Option<(KataAnalyzeMove, Vec<String>)> {
    let tokens: Vec<&str> = block.split_whitespace().collect();
    let mut idx = 0;

    let mut mv: Option<String> = None;
    let mut visits = 0u32;
    let mut winrate = 0.0f64;
    let mut score_lead = 0.0f64;
    let mut pv: Vec<String> = Vec::new();
    let mut leftover: Vec<String> = Vec::new();

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
                let mut i = idx + 1;
                while i < tokens.len() && looks_like_vertex(tokens[i]) {
                    pv.push(tokens[i].to_string());
                    i += 1;
                }
                leftover = tokens[i..].iter().map(|s| s.to_string()).collect();
                idx = tokens.len();
            }
            _ => {
                idx += 1;
                while idx < tokens.len() && !KNOWN_BLOCK_KEYS.contains(&tokens[idx]) {
                    idx += 1;
                }
            }
        }
    }

    Some((
        KataAnalyzeMove {
            r#move: mv?,
            visits,
            winrate,
            score_lead,
            pv,
        },
        leftover,
    ))
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

    #[test]
    fn without_ownership_option_field_is_none() {
        let line = "info move Q16 visits 123 winrate 0.54 scoreLead 1.2 pv Q16 D4";
        let result = parse_kata_analyze(line).expect("should parse");
        assert_eq!(result.ownership, None);
    }

    #[test]
    fn unknown_field_with_no_value_token_does_not_swallow_following_fields() {
        // "isDuringSearch"류 플래그성 필드를 값 없이 흉내(실제로는 항상 값이 붙지만,
        // 파서가 "값 하나" 가정 없이도 다음 알려진 key까지 안전하게 건너뛰는지 확인).
        let line = "info move Q16 visits 123 winrate 0.54 scoreLead 1.2 isDuringSearch pv Q16 D4";
        let result = parse_kata_analyze(line).expect("should parse");
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].pv, vec!["Q16", "D4"]);
    }

    #[test]
    fn unknown_field_with_multiple_value_tokens_does_not_swallow_following_fields() {
        // pvVisits류 리스트 필드를 흉내(여러 토큰짜리 값) - "값 하나"만 건너뛰면 남은
        // 토큰들이 다음 필드(winrate)의 key/value로 잘못 소비된다.
        let line =
            "info move Q16 visits 123 pvVisits 10 8 6 4 winrate 0.54 scoreLead 1.2 pv Q16 D4";
        let result = parse_kata_analyze(line).expect("should parse");
        let c = &result.candidates[0];
        assert_eq!(c.visits, 123);
        assert!((c.winrate - 0.54).abs() < 1e-9);
        assert!((c.score_lead - 1.2).abs() < 1e-9);
        assert_eq!(c.pv, vec!["Q16", "D4"]);
    }

    #[test]
    fn parses_trailing_ownership_field_on_last_block_only() {
        // ownership은 줄 전체 단위 필드라 맨 마지막 move 블록의 pv 뒤에만 붙어 나옴 -
        // 그 앞쪽 블록들의 파싱/pv 목록은 전혀 영향받지 않아야 함.
        let line = "info move Q16 visits 123 winrate 0.54 scoreLead 1.2 pv Q16 D4 info move D4 visits 90 winrate 0.49 scoreLead 0.9 pv D4 Q16 ownership 0.1 -0.2 0.05 -1 1";
        let result = parse_kata_analyze(line).expect("should parse");
        assert_eq!(result.candidates.len(), 2);
        assert_eq!(result.candidates[0].pv, vec!["Q16", "D4"]);
        assert_eq!(result.candidates[1].pv, vec!["D4", "Q16"]);
        assert_eq!(result.ownership, Some(vec![0.1, -0.2, 0.05, -1.0, 1.0]));
    }
}
