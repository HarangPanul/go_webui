// Tauri managed state: 동시에 연결된 여러 SSH/GTP 세션, 흑/백 엔진 배정, 게임 트리
// 등을 보관
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AsyncMutex;

use crate::game::{BoardSnapshot, Color, GameTree};
use crate::gtp::process::GtpSession;

/// 메인 화면에서 사용자가 지정한 "흑/백을 각각 어느 프로필(서버)이 자동으로 둘지".
/// 값은 서버 프로필의 id - 그 프로필이 실제로 연결되어 있어야 동작하고(연결 안 됐거나
/// 연결이 끊기면 그 색은 다시 사람이 둠), 흑/백에 같은 프로필을 배정해도 세션은
/// state.sessions에 하나만 존재하므로 자기 자신과 대국하듯 동작한다(기존과 동일).
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EngineAssignment {
    pub black: Option<String>,
    pub white: Option<String>,
}

impl EngineAssignment {
    fn profile_for(&self, color: Color) -> Option<&str> {
        match color {
            Color::Black => self.black.as_deref(),
            Color::White => self.white.as_deref(),
        }
    }
}

// 덤(komi) 기본값 - 사용자가 아직 값을 바꾼 적 없을 때(앱을 처음 실행했을 때)
// Settings에 보여줄 초기값. 6.5는 일본식 규칙에서 가장 흔히 쓰이는 값.
pub const DEFAULT_KOMI: f64 = 6.5;

pub struct AppState {
    // 커맨드가 async라 lock을 GtpSession::send().await 너머로 들고 갈 일이 생기므로
    // std Mutex가 아니라 tokio Mutex 사용. 단, 실제로 lock을 쥔 채로 send()까지
    // await하지 않도록 주의 — Arc만 clone해서 즉시 lock을 해제한 뒤 lock 밖에서
    // await할 것 (그렇지 않으면 다른 모든 SSH/GTP 커맨드가 그동안 block됨). 필드를
    // private으로 감추고 아래 accessor로만 접근하게 해서 이 규칙을 구조적으로 강제한다.
    //
    // 프로필 id를 key로 여러 세션을 동시에 들고 있을 수 있다(흑/백을 서로 다른
    // 서버가 두게 하려면 최소 두 개가 동시에 연결되어 있어야 함). 같은 프로필을
    // 두 번 연결하려 하면(connection_service::connect) 이 맵에 이미 있는 항목을
    // 그대로 두고 새 SSH 연결을 만들지 않는다 - 흑/백 양쪽에 같은 프로필을 배정해도
    // katago 프로세스가 하나만 뜨는 이유.
    sessions: AsyncMutex<HashMap<String, Arc<GtpSession>>>,
    game: Mutex<GameTree>,
    // commands/game.rs가 매 착수 후 이 값을 보고 자동으로 genmove를 요청할지(어느
    // 색을, 어느 세션에) 판단.
    engine_assignment: Mutex<EngineAssignment>,
    // Settings에서 설정한 덤. connect_ssh가 세션을 새로 맺을 때, 그리고
    // commands::gtp::set_komi가 값이 바뀔 때마다 연결된 모든 엔진에 `komi <값>`으로
    // 즉시 반영한다 - 로컬 게임 트리 자체는 덤을 집 계산에 쓰지 않으므로(그건
    // KataGo에 위임) 여기 보관된 값은 오직 엔진에 보낼 값을 기억해두는 용도.
    komi: Mutex<f64>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            sessions: AsyncMutex::new(HashMap::new()),
            game: Mutex::new(GameTree::default()),
            engine_assignment: Mutex::new(EngineAssignment::default()),
            komi: Mutex::new(DEFAULT_KOMI),
        }
    }
}

impl AppState {
    /// GameTree 락을 쥐고 클로저를 실행한 뒤 클로저가 끝나는 즉시 락을 해제한다.
    /// `game` 필드에 직접 접근하는 길을 막아, 락을 쥔 채로 `.await`하지 않는다는
    /// 규칙이 (기존처럼 주석 컨벤션이 아니라) 컴파일 타임에 강제되게 한다.
    pub fn with_game<R>(&self, f: impl FnOnce(&mut GameTree) -> R) -> R {
        f(&mut self.game.lock().unwrap())
    }

    pub fn game_snapshot(&self) -> BoardSnapshot {
        self.game.lock().unwrap().snapshot()
    }

    /// 주어진 프로필의 세션이 연결되어 있으면 그 `Arc`를 clone해 즉시 반환한다 - 락은
    /// 이 함수 안에서만 쥐고 바로 해제되므로, 호출자가 반환값으로 실제 통신
    /// (`send().await` 등)을 하는 동안에는 이미 락 밖이라 다른 SSH/GTP 커맨드를
    /// block하지 않는다.
    pub async fn session_for_profile(&self, profile_id: &str) -> Option<Arc<GtpSession>> {
        self.sessions.lock().await.get(profile_id).cloned()
    }

    pub async fn is_connected(&self, profile_id: &str) -> bool {
        self.sessions.lock().await.contains_key(profile_id)
    }

    /// 지금 연결된 모든 세션 - 착수/pass/undo를 "연결된 세션 전부"에 미러링해야
    /// 하는 engine_sync가 사용(흑/백에 서로 다른 서버가 배정돼 있어도 각자 정확한
    /// 보드 상태를 유지해야 하므로).
    pub async fn all_sessions(&self) -> Vec<Arc<GtpSession>> {
        self.sessions.lock().await.values().cloned().collect()
    }

    /// 연결된 세션이 하나도 없으면 None. kata-analyze처럼 "누가 됐든 연결된 엔진
    /// 아무거나 하나"면 충분한 경우(지금 차례 색에 배정된 세션이 없을 때의 폴백)에 사용.
    pub async fn any_session(&self) -> Option<Arc<GtpSession>> {
        self.sessions.lock().await.values().next().cloned()
    }

    pub async fn insert_session(&self, profile_id: String, session: Arc<GtpSession>) {
        self.sessions.lock().await.insert(profile_id, session);
    }

    /// 세션 자리를 비우면서 그 전에 들어있던 세션을 반환한다(`disconnect_ssh` 전용).
    pub async fn remove_session(&self, profile_id: &str) -> Option<Arc<GtpSession>> {
        self.sessions.lock().await.remove(profile_id)
    }

    /// 지금 차례 색에 배정된 프로필의 세션 - 배정 자체가 없거나, 배정된 프로필이
    /// 더 이상 연결되어 있지 않으면(예: 연결이 끊겼는데 배정 정리가 아직 안 됐을
    /// 극히 짧은 순간) None.
    pub async fn session_for_color(&self, color: Color) -> Option<Arc<GtpSession>> {
        let profile_id = {
            let assignment = self.engine_assignment.lock().unwrap();
            assignment.profile_for(color).map(|id| id.to_string())
        };
        match profile_id {
            Some(id) => self.session_for_profile(&id).await,
            None => None,
        }
    }

    pub fn engine_assignment(&self) -> EngineAssignment {
        self.engine_assignment.lock().unwrap().clone()
    }

    pub fn set_engine_assignment(&self, color: Color, profile_id: Option<String>) {
        let mut assignment = self.engine_assignment.lock().unwrap();
        match color {
            Color::Black => assignment.black = profile_id,
            Color::White => assignment.white = profile_id,
        }
    }

    /// 프로필 연결이 끊겼을 때(disconnect_ssh) 호출 - 그 프로필이 흑/백 어느 쪽에
    /// 배정돼 있었든 풀어준다. 그러지 않으면 연결이 끊긴 프로필이 계속 배정된 채로
    /// 남아 그 색이 아무도 안 두는 상태(session_for_color가 항상 None)로 조용히
    /// 멈춰버린다.
    pub fn clear_engine_assignment_for(&self, profile_id: &str) {
        let mut assignment = self.engine_assignment.lock().unwrap();
        if assignment.black.as_deref() == Some(profile_id) {
            assignment.black = None;
        }
        if assignment.white.as_deref() == Some(profile_id) {
            assignment.white = None;
        }
    }

    pub fn komi(&self) -> f64 {
        *self.komi.lock().unwrap()
    }

    pub fn set_komi_value(&self, komi: f64) {
        *self.komi.lock().unwrap() = komi;
    }
}
