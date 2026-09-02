// SSH 연결/해제 커맨드. russh 세션 수립 후 `katago gtp` 프로세스 기동까지 담당
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::gtp::process::GtpSession;
use crate::ssh::keystore;
use crate::ssh::local_keys::{self, LocalSshKeyInfo};
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

    // 새로 붙은 엔진은 KataGo 자체 기본 덤으로 시작하므로, Settings에서 이미
    // 설정해둔 값이 있으면(또는 아직 아무도 안 바꿨어도 DEFAULT_KOMI로) 접속 직후
    // 바로 반영해둔다 - 그래야 이후 대국 내내 화면에 보이는 덤과 엔진이 실제로 쓰는
    // 덤이 어긋나지 않는다. best-effort: 이 시점에 실패해도 연결 자체는 그대로 둔다
    // (사용자가 Settings에서 값을 바꾸면 set_komi가 다시 시도함).
    let komi = *state.komi.lock().unwrap();
    let _ = session.send(&format!("komi {komi}")).await;

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

/// 데스크탑에서 `~/.ssh`에 있는 key들을 감지해 프로필 폼에서 고를 수 있는 목록으로
/// 돌려준다. 모바일에서는 항상 에러 - 프론트는 이걸 "이 플랫폼은 미지원"으로 받아
/// 감지 UI 자체를 숨기면 됨.
#[tauri::command]
pub async fn list_local_ssh_keys() -> Result<Vec<LocalSshKeyInfo>, String> {
    local_keys::list()
}

/// `list_local_ssh_keys`가 돌려준 경로 중 하나를 골랐을 때 실제 key 원문을 읽어온다.
#[tauri::command]
pub async fn load_local_ssh_key(path: String) -> Result<String, String> {
    local_keys::load(&path)
}
