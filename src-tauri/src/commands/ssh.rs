// SSH 연결/해제 커맨드. russh 세션 수립 후 `katago gtp` 프로세스 기동까지 담당
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn connect_ssh(_state: State<'_, AppState>, _profile_id: String) -> Result<(), String> {
    // TODO: russh_keys로 in-memory key 인증, 세션 수립 후 `katago gtp` 실행
    // 무응답 시 10초 간격 재연결 재시도 로직도 이 경로에 연결
    Ok(())
}

#[tauri::command]
pub async fn disconnect_ssh(_state: State<'_, AppState>) -> Result<(), String> {
    // TODO: GTP 프로세스 종료 및 세션 정리
    Ok(())
}
