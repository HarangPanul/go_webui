// SSH를 완전히 건너뛰고, PATH에 있는 실제 `katago_gtp`(KataGo 바이너리를 감싼 wrapper
// 스크립트)를 로컬 자식 프로세스로 직접 띄워 stdin/stdout으로 GTP 대화를 나누는
// 통합 테스트. russh/AppState/ConnectionService는 전혀 거치지 않는다 - 여기서 검증하려는
// 건 그 레이어들이 아니라, gtp::coords/gtp::parser에 "KataGo의 C++ 소스를 읽고" 박아둔
// 프로토콜 가정(vertex 표기, kata-analyze 스트리밍 프레이밍, ownership 트레일링 필드
// 형식)이 실제 KataGo와 정말 맞는지다.
//
// 로컬에 KataGo가 설치되어 있어야 해서 기본 `cargo test`에는 포함되지 않는다:
//     cargo test --lib gtp::live_katago_tests -- --ignored --nocapture
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::gtp::{coords, parser};

/// 엔진이 응답하지 않을 때(모델 로딩/CUDA 튜닝이 유난히 오래 걸리는 경우 등) 테스트가
/// 무한정 멈춰있지 않도록. KataGo 첫 실행은 튜닝 캐시가 없으면 수십 초씩 걸릴 수 있어
/// 넉넉하게 잡는다.
const READ_TIMEOUT: Duration = Duration::from_secs(120);

const BOARD_SIZE: usize = 19;

/// SSH 없이 `katago_gtp`를 직접 자식 프로세스로 띄운 GTP 세션. gtp/process.rs의
/// spawn_reader/dispatch_line과 동일한 "빈 줄까지 누적, info 줄은 즉시 분리" 규칙을
/// 그대로 재현해 프로덕션 코드가 기대하는 프레이밍이 실제 엔진에서도 성립하는지 확인한다.
struct KataGoProcess {
    // 읽고 쓰는 데는 안 쓰이지만, drop되면 kill_on_drop(true)로 자식 프로세스가 함께
    // 정리되므로 테스트가 끝날 때까지 값 자체는 들고 있어야 한다.
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl KataGoProcess {
    async fn spawn() -> Self {
        let mut child = Command::new("katago_gtp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect(
                "PATH에서 katago_gtp를 찾지 못함 - `which katago_gtp`로 실제 KataGo wrapper가 \
                 설치되어 있는지 확인하세요",
            );
        let stdin = child.stdin.take().expect("child stdin가 없음");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout이 없음"));
        KataGoProcess {
            _child: child,
            stdin,
            stdout,
        }
    }

    /// 명령 한 줄을 보내고, "info " 스트리밍 줄은 건너뛴 채 다음 빈 줄로 끝나는 정규
    /// GTP 응답 블록을 그대로(맨 앞 "="/"?" 포함) 반환한다.
    async fn send(&mut self, line: &str) -> String {
        self.write_line(line).await;
        self.read_response().await
    }

    async fn write_line(&mut self, line: &str) {
        self.stdin
            .write_all(format!("{line}\n").as_bytes())
            .await
            .expect("katago_gtp stdin에 쓰기 실패 - 프로세스가 이미 죽었을 수 있음");
        self.stdin
            .flush()
            .await
            .expect("katago_gtp stdin flush 실패");
    }

    async fn read_response(&mut self) -> String {
        tokio::time::timeout(READ_TIMEOUT, async {
            let mut acc = String::new();
            loop {
                let line = self.read_raw_line().await;
                if parser::is_analysis_line(&line) {
                    continue; // 스트리밍 줄은 정규 응답에 섞이지 않으므로 건너뜀
                }
                if line.is_empty() {
                    return acc;
                }
                if !acc.is_empty() {
                    acc.push('\n');
                }
                acc.push_str(&line);
            }
        })
        .await
        .expect("katago_gtp 응답 시간 초과")
    }

    /// kata-analyze 스트리밍 도중 "info " 줄을 최소 `min_lines`개 모을 때까지 읽는다.
    async fn read_analysis_lines(&mut self, min_lines: usize) -> Vec<String> {
        tokio::time::timeout(READ_TIMEOUT, async {
            let mut lines = Vec::new();
            while lines.len() < min_lines {
                let line = self.read_raw_line().await;
                if parser::is_analysis_line(&line) {
                    lines.push(line);
                }
            }
            lines
        })
        .await
        .expect("kata-analyze 스트리밍 응답 시간 초과")
    }

    async fn read_raw_line(&mut self) -> String {
        let mut line = String::new();
        let n = self
            .stdout
            .read_line(&mut line)
            .await
            .expect("katago_gtp stdout 읽기 실패");
        assert!(n > 0, "katago_gtp가 응답 전에 stdout을 닫음(비정상 종료)");
        line.trim_end_matches(['\n', '\r']).to_string()
    }
}

fn assert_not_rejected(context: &str, resp: &str) {
    assert!(
        !resp.trim_start().starts_with('?'),
        "{context} 명령이 거부됨: {resp}"
    );
}

#[tokio::test]
#[ignore = "로컬 PATH에 실제 katago_gtp(KataGo 바이너리 wrapper)가 있어야 함"]
async fn play_genmove_undo_roundtrip_against_real_katago() {
    let mut kg = KataGoProcess::spawn().await;

    let komi_resp = kg.send("komi 6.5").await;
    assert_not_rejected("komi", &komi_resp);

    // gtp::coords::to_vertex가 만든 vertex 문자열을 실제 KataGo가 그대로 받아들이는지 확인.
    let vertex = coords::to_vertex(3, 3, BOARD_SIZE);
    let play_resp = kg.send(&format!("play B {vertex}")).await;
    assert_not_rejected("play", &play_resp);

    let genmove_resp = kg.send("genmove W").await;
    assert_not_rejected("genmove", &genmove_resp);
    let reply = genmove_resp
        .trim()
        .strip_prefix('=')
        .expect("genmove 응답에 '=' 접두가 없음")
        .trim();
    // engine_sync::request_engine_move_if_needed와 동일한 순서로 검사: resign/pass가
    // 아니면 gtp::coords::from_vertex로 파싱 가능해야 한다.
    if !reply.eq_ignore_ascii_case("resign") && !reply.eq_ignore_ascii_case("pass") {
        assert!(
            coords::from_vertex(reply, BOARD_SIZE).is_some(),
            "genmove가 돌려준 '{reply}'를 좌표로 파싱하지 못함"
        );
    }

    let undo_resp = kg.send("undo").await;
    assert_not_rejected("undo", &undo_resp);

    let quit_resp = kg.send("quit").await;
    assert_not_rejected("quit", &quit_resp);
}

#[tokio::test]
#[ignore = "로컬 PATH에 실제 katago_gtp(KataGo 바이너리 wrapper)가 있어야 함"]
async fn kata_analyze_streams_and_yields_to_next_command_against_real_katago() {
    let mut kg = KataGoProcess::spawn().await;

    let komi_resp = kg.send("komi 6.5").await;
    assert_not_rejected("komi", &komi_resp);

    // start_kata_analyze와 동일한 명령 형태.
    kg.write_line("kata-analyze 20 ownership true").await;

    let analysis_lines = kg.read_analysis_lines(2).await;
    assert!(analysis_lines.len() >= 2, "info 줄이 충분히 오지 않음");
    for line in &analysis_lines {
        let parsed = parser::parse_kata_analyze(line)
            .unwrap_or_else(|| panic!("실제 KataGo의 info 줄을 파싱하지 못함: {line}"));
        assert!(!parsed.candidates.is_empty());
        let ownership = parsed
            .ownership
            .unwrap_or_else(|| panic!("ownership true로 요청했는데 ownership 필드가 없음: {line}"));
        assert_eq!(
            ownership.len(),
            BOARD_SIZE * BOARD_SIZE,
            "ownership 배열 길이가 보드 칸 수와 다름: {line}"
        );
    }

    // 다음 명령을 보내면(사람 착수 미러링을 흉내) 실제 KataGo도 그 즉시 분석을 멈추고,
    // 먼저 원래 kata-analyze 명령 자체를 "="로 완료 처리한 뒤에야 이 새 명령의 진짜
    // 응답을 내놓는다 - gtp/process.rs의 FIFO 대기열(GtpSession::send)이 정확히 이
    // 순서를 전제로 설계되어 있으므로, 실제 엔진에서도 이 순서가 성립하는지 확인.
    kg.write_line("protocol_version").await;
    let kata_analyze_completion = kg.read_response().await;
    assert_not_rejected("kata-analyze(중단 완료)", &kata_analyze_completion);

    let protocol_version_resp = kg.read_response().await;
    assert_eq!(protocol_version_resp.trim(), "= 2");

    let quit_resp = kg.send("quit").await;
    assert_not_rejected("quit", &quit_resp);
}
