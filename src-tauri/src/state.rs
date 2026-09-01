// Tauri managed state: 활성 SSH 세션, GTP 프로세스 핸들, 게임 트리 등을 보관
use std::sync::Mutex;

use crate::game::GameTree;
use crate::gtp::process::GtpSession;

#[derive(Default)]
pub struct AppState {
    pub gtp_session: Mutex<Option<GtpSession>>,
    pub game: Mutex<GameTree>,
}
