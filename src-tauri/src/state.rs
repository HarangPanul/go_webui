// Tauri managed state: 활성 SSH/GTP 세션, 게임 트리 등을 보관
use std::sync::{Arc, Mutex};

use tokio::sync::Mutex as AsyncMutex;

use crate::game::GameTree;
use crate::gtp::process::GtpSession;

pub struct AppState {
    // 커맨드가 async라 lock을 GtpSession::send().await 너머로 들고 갈 일이 생기므로
    // std Mutex가 아니라 tokio Mutex 사용. 단, 실제로 lock을 쥔 채로 send()까지
    // await하지 않도록 주의 — Arc만 clone해서 즉시 lock을 해제한 뒤 lock 밖에서
    // await할 것 (그렇지 않으면 다른 모든 SSH/GTP 커맨드가 그동안 block됨).
    pub gtp_session: AsyncMutex<Option<Arc<GtpSession>>>,
    pub game: Mutex<GameTree>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            gtp_session: AsyncMutex::new(None),
            game: Mutex::new(GameTree::default()),
        }
    }
}
