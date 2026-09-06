// 로컬 온디바이스 엔진(tauri-plugin-katago-local)에만 의미 있는 설정 - genmove가 쓸
// Mcts 시뮬레이션 수(max_visits). 원격 SSH KataGo는 이 GTP 확장 명령 자체를 모르므로
// (best-effort로 "? unknown command" 무시됨) 사실상 무해하지만, 이 값을 실제로 쓰는
// 건 로컬 엔진뿐이다.
//
// 값의 진짜 저장소는 이 파일이 아니라 그 플러그인 자신(KatagoLocal::max_visits) -
// 여기 두 커맨드는 프런트(get/set_max_visits)와 그 저장소 사이의 얇은 다리일 뿐이다.
// AppState(SSH 세션 등 범용 상태)는 이 값의 존재를 전혀 모른다 - "로컬 엔진에만
// 의미 있는 기능은 관련 코드가 프로젝트 전체에 최대한 적은 연결고리만 갖도록
// 한정한다"는 원칙에 따른 배치. (재)연결 시 이미 연결된 세션에도 이 값을 자동으로
// 반영하는 것도 여기서 하지 않고 gtp::transport::GtpTransport::extra_resync_commands
// 훅으로 옮겨, engine_sync(범용 재연결 로직)가 "max_visits"라는 이름 자체를 몰라도
// 되게 했다.
use tauri::{AppHandle, State};
use tauri_plugin_katago_local::KatagoLocalExt;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn get_max_visits(app: AppHandle) -> u32 {
    app.katago_local().max_visits()
}

/// set_komi(commands/gtp.rs)와 완전히 같은 패턴 - 저장과 동시에, 지금 연결된 세션이
/// 있으면(여러 개일 수 있음) 바로 반영한다. 새로 연결되는 세션은
/// AndroidLocalTransport::extra_resync_commands가 알아서 이 값을 읽어 보낸다.
#[tauri::command]
#[specta::specta]
pub async fn set_max_visits(
    max_visits: u32,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    app.katago_local().set_max_visits(max_visits);

    for session in state.all_sessions().await {
        let _ = session.send(&format!("max_visits {max_visits}")).await;
    }

    Ok(())
}
