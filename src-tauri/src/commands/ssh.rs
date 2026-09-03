// SSH 연결/해제 커맨드. 실제 연결 수립/세션 관리 로직은 전부
// crate::services::connection_service::ConnectionService가 담당한다 - 여기서는
// State/AppHandle을 받아 위임하기만 한다. list_local_ssh_keys/load_local_ssh_key는
// AppState/세션과 무관하게 로컬 파일시스템만 보는 유틸이라 그대로 둔다.
use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::services::connection_service::ConnectionService;
use crate::ssh::local_keys::{self, LocalSshKeyInfo};
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn connect_ssh(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<(), AppError> {
    ConnectionService::new(&state).connect(app, profile_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn disconnect_ssh(state: State<'_, AppState>) -> Result<(), AppError> {
    ConnectionService::new(&state).disconnect().await
}

/// 데스크탑에서 `~/.ssh`에 있는 key들을 감지해 프로필 폼에서 고를 수 있는 목록으로
/// 돌려준다. 모바일에서는 항상 에러 - 프론트는 이걸 "이 플랫폼은 미지원"으로 받아
/// 감지 UI 자체를 숨기면 됨.
#[tauri::command]
#[specta::specta]
pub async fn list_local_ssh_keys() -> Result<Vec<LocalSshKeyInfo>, AppError> {
    local_keys::list()
}

/// `list_local_ssh_keys`가 돌려준 경로 중 하나를 골랐을 때 실제 key 원문을 읽어온다.
#[tauri::command]
#[specta::specta]
pub async fn load_local_ssh_key(path: String) -> Result<String, AppError> {
    local_keys::load(&path)
}
