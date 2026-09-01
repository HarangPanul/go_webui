// GTP 명령 전송 커맨드 (착수, pass, kata-analyze 시작 등)
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn send_gtp_command(
    _state: State<'_, AppState>,
    _command: String,
) -> Result<String, String> {
    // TODO: Write half로 GTP 명령 전송, 응답은 Read 파서가 별도 emit
    Ok(String::new())
}
