// GTP 명령 전송 커맨드 (착수, pass, kata-analyze 시작 등)
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn send_gtp_command(
    state: State<'_, AppState>,
    command: String,
) -> Result<String, String> {
    // 락을 쥔 채로 세션의 send().await(원격 응답 대기)까지 들고 가면 그동안 다른 모든
    // SSH/GTP 커맨드가 block되므로, Arc만 clone해서 즉시 락을 해제한 뒤 락 밖에서 보낸다.
    let session = {
        let guard = state.gtp_session.lock().await;
        guard.clone()
    };

    let session = session.ok_or_else(|| "SSH에 연결되어 있지 않습니다".to_string())?;
    session.send(&command).await
}
