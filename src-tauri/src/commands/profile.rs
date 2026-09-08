// 서버 프로필 CRUD/전환 커맨드. 실제 저장은 ssh::keystore에 위임
use tauri::AppHandle;

use crate::error::AppError;
use crate::models::server_profile::{SaveProfileInput, ServerProfileInfo};
use crate::ssh::keystore;

#[tauri::command]
#[specta::specta]
pub async fn list_profiles(app: AppHandle) -> Result<Vec<ServerProfileInfo>, AppError> {
    let profiles = keystore::list(&app)?;
    Ok(profiles.iter().map(ServerProfileInfo::from).collect())
}

#[tauri::command]
#[specta::specta]
pub async fn save_profile(
    app: AppHandle,
    input: SaveProfileInput,
) -> Result<ServerProfileInfo, AppError> {
    let profile = keystore::save(&app, input)?;
    Ok(ServerProfileInfo::from(&profile))
}

#[tauri::command]
#[specta::specta]
pub async fn delete_profile(app: AppHandle, id: String) -> Result<(), AppError> {
    keystore::delete(&app, &id)
}

#[tauri::command]
#[specta::specta]
pub async fn switch_profile(app: AppHandle, profile_id: String) -> Result<(), AppError> {
    keystore::set_active(&app, &profile_id)
}

/// 저장된 SSH host key 신뢰(TOFU 지문)를 초기화한다. 서버를 재설치해 host key가
/// 정말로 바뀐 경우 이걸 눌러야 다음 연결이 HostKeyMismatch로 거부되지 않고 새
/// 지문을 다시 TOFU로 신뢰한다 - ssh/client.rs::ClientHandler, gtp/ssh_transport.rs
/// 참고.
#[tauri::command]
#[specta::specta]
pub async fn forget_host_key_fingerprint(app: AppHandle, id: String) -> Result<(), AppError> {
    keystore::set_host_key_fingerprint(&app, &id, None)
}
