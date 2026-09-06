pub mod android_transport;
pub mod coords;
// SSH 없이 로컬 katago_gtp 바이너리를 직접 자식 프로세스로 띄워 검증하는 통합 테스트.
// 실제 엔진이 설치되어 있어야 하므로 #[ignore]로 기본 `cargo test`에서는 건너뛴다.
#[cfg(test)]
mod live_katago_tests;
pub mod parser;
pub mod process;
pub mod ssh_transport;
pub mod transport;
