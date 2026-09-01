// 서버 프로필 CRUD/전환 커맨드. 실제 저장은 ssh::keystore에 위임
use tauri::AppHandle;

use crate::models::server_profile::{SaveProfileInput, ServerProfileInfo};
use crate::ssh::keystore;

#[tauri::command]
pub async fn list_profiles(app: AppHandle) -> Result<Vec<ServerProfileInfo>, String> {
    let profiles = keystore::list(&app)?;
    Ok(profiles.iter().map(ServerProfileInfo::from).collect())
}

#[tauri::command]
pub async fn save_profile(
    app: AppHandle,
    input: SaveProfileInput,
) -> Result<ServerProfileInfo, String> {
    let profile = keystore::save(&app, input)?;
    Ok(ServerProfileInfo::from(&profile))
}

#[tauri::command]
pub async fn delete_profile(app: AppHandle, id: String) -> Result<(), String> {
    keystore::delete(&app, &id)
}

#[tauri::command]
pub async fn switch_profile(app: AppHandle, profile_id: String) -> Result<(), String> {
    keystore::set_active(&app, &profile_id)
}
