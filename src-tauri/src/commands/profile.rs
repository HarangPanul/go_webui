// 서버 프로필 CRUD/전환 커맨드. 실제 저장은 ssh::keystore에 위임
use crate::models::server_profile::ServerProfile;

#[tauri::command]
pub async fn list_profiles() -> Result<Vec<ServerProfile>, String> {
    // TODO: sandbox storage에서 프로필 목록 로드
    Ok(vec![])
}

#[tauri::command]
pub async fn save_profile(_profile: ServerProfile) -> Result<(), String> {
    // TODO: sandbox storage에 프로필 저장 (Key 텍스트 포함)
    Ok(())
}

#[tauri::command]
pub async fn switch_profile(_profile_id: String) -> Result<(), String> {
    // TODO: 활성 프로필 전환
    Ok(())
}
