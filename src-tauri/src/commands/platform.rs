// 프런트가 플랫폼별로 UI를 갈라야 할 때(지금은 "Local" 프로필 종류를 보여줄지) 쓰는
// 작은 조회용 커맨드 모음.

/// 이 빌드에서 "Local"(이 기기의 온디바이스 KataGo, tauri-plugin-katago-local) 프로필
/// 종류를 실제로 쓸 수 있는지. Android가 아니면 항상 false - ServerProfileForm이 이
/// 값에 따라 SSH/Local 선택 UI 자체를 보여줄지 말지 정한다(Local을 골라도 연결
/// 시점에야 실패하는 것보다, 애초에 고를 수 없게 하는 편이 명확함).
#[tauri::command]
#[specta::specta]
pub fn supports_local_engine() -> bool {
    cfg!(target_os = "android")
}
