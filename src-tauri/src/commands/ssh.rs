// SSH 연결/해제 커맨드. russh 세션 수립 후 `katago gtp` 프로세스 기동까지 담당
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::gtp::process::GtpSession;
use crate::ssh::keystore;
use crate::state::AppState;

#[tauri::command]
pub async fn connect_ssh(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<(), String> {
    let profile = keystore::get(&app, &profile_id)?
        .ok_or_else(|| format!("존재하지 않는 프로필: {profile_id}"))?;

    if profile.has_passphrase {
        // PLAN.md 결정사항: passphrase 걸린 key는 1차 버전 미지원, 경고만 표시
        return Err(
            "passphrase가 걸린 key는 아직 지원하지 않습니다. passphrase 없는 key를 사용해주세요."
                .to_string(),
        );
    }

    let session = GtpSession::connect(app, profile).await?;

    let mut guard = state.gtp_session.lock().await;
    *guard = Some(Arc::new(session));

    Ok(())
}

#[tauri::command]
pub async fn disconnect_ssh(state: State<'_, AppState>) -> Result<(), String> {
    let session = {
        let mut guard = state.gtp_session.lock().await;
        guard.take()
    };

    if let Some(session) = session {
        session.disconnect().await;
    }

    Ok(())
}
