// Tauri managed state: 활성 SSH/GTP 세션, 게임 트리 등을 보관
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AsyncMutex;

use crate::game::{Color, GameTree};
use crate::gtp::process::GtpSession;

/// 메인 화면에서 사용자가 지정한 "KataGo가 자동으로 둘 색". 흑/백이 서로 독립적인
/// on/off 스위치라, 둘 다 켜면 KataGo가 자기 자신과 대국(self-play)하듯 양쪽을 모두
/// 계속 두고, 둘 다 끄면 자동 착수 없이 사람이 양쪽을 다 둔다(기존 동작과 동일).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineColors {
    pub black: bool,
    pub white: bool,
}

impl EngineColors {
    pub fn enabled(&self, color: Color) -> bool {
        match color {
            Color::Black => self.black,
            Color::White => self.white,
        }
    }
}

// 덤(komi) 기본값 - 이전에는 어디에도 설정된 적 없어 KataGo 자체 기본값을 그대로
// 썼는데, 이제 Settings에서 조절 가능해지므로 앱이 처음 실행됐을 때(아직 사용자가
// 값을 바꾼 적 없을 때) 보여줄 값이 필요해서 명시적으로 둠. 6.5는 일본식 규칙에서
// 가장 흔히 쓰이는 값.
pub const DEFAULT_KOMI: f64 = 6.5;

pub struct AppState {
    // 커맨드가 async라 lock을 GtpSession::send().await 너머로 들고 갈 일이 생기므로
    // std Mutex가 아니라 tokio Mutex 사용. 단, 실제로 lock을 쥔 채로 send()까지
    // await하지 않도록 주의 — Arc만 clone해서 즉시 lock을 해제한 뒤 lock 밖에서
    // await할 것 (그렇지 않으면 다른 모든 SSH/GTP 커맨드가 그동안 block됨).
    pub gtp_session: AsyncMutex<Option<Arc<GtpSession>>>,
    pub game: Mutex<GameTree>,
    // commands/game.rs::confirm_move가 매 착수 후 이 값을 보고 자동으로 genmove를
    // 요청할지(어느 색까지) 판단.
    pub engine_colors: Mutex<EngineColors>,
    // Settings에서 설정한 덤. connect_ssh가 세션을 새로 맺을 때, 그리고
    // commands::gtp::set_komi가 값이 바뀔 때마다 연결된 엔진에 `komi <값>`으로
    // 즉시 반영한다 - 로컬 게임 트리 자체는 덤을 집 계산에 쓰지 않으므로(그건
    // KataGo에 위임) 여기 보관된 값은 오직 엔진에 보낼 값을 기억해두는 용도.
    pub komi: Mutex<f64>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            gtp_session: AsyncMutex::new(None),
            game: Mutex::new(GameTree::default()),
            engine_colors: Mutex::new(EngineColors::default()),
            komi: Mutex::new(DEFAULT_KOMI),
        }
    }
}
