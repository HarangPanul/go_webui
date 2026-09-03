// 새 서비스 레이어: state 위, commands 아래에서 게임/엔진 동기화/연결 관련 로직을
// 모아둔다. state가 services를 import하지 않으므로 game <-> gtp/state였던 기존 순환
// 의존이 구조적으로 재발할 수 없다.
pub mod connection_service;
pub mod engine_sync;
pub mod game_service;
