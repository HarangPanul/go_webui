// SSH 연결/해제 로직: russh 세션 수립(ssh::client) 후 GTP 프로세스 기동(gtp::process)까지
// 담당한다. commands/ssh.rs는 State/AppHandle을 받아 이 서비스를 구성하고 위임하기만 함.
use std::sync::Arc;

use tauri::AppHandle;

use crate::error::AppError;
use crate::gtp::process::GtpSession;
use crate::ssh::keystore;
use crate::state::AppState;

pub struct ConnectionService<'a> {
    state: &'a AppState,
}

impl<'a> ConnectionService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        ConnectionService { state }
    }

    pub async fn connect(&self, app: AppHandle, profile_id: String) -> Result<(), AppError> {
        let profile = keystore::get(&app, &profile_id)?
            .ok_or_else(|| AppError::ProfileNotFound(profile_id.clone()))?;

        if profile.has_passphrase {
            // PLAN.md 결정사항: passphrase 걸린 key는 1차 버전 미지원, 경고만 표시
            return Err(AppError::PassphraseUnsupported);
        }

        let session = GtpSession::connect(app, profile).await?;

        // 새로 붙은 엔진은 KataGo 자체 기본 덤으로 시작하므로, Settings에서 이미
        // 설정해둔 값이 있으면(또는 아직 아무도 안 바꿨어도 DEFAULT_KOMI로) 접속 직후
        // 바로 반영해둔다 - 그래야 이후 대국 내내 화면에 보이는 덤과 엔진이 실제로 쓰는
        // 덤이 어긋나지 않는다. best-effort: 이 시점에 실패해도 연결 자체는 그대로 둔다
        // (사용자가 Settings에서 값을 바꾸면 set_komi가 다시 시도함).
        let komi = self.state.komi();
        let _ = session.send(&format!("komi {komi}")).await;

        self.state.set_session(Some(Arc::new(session))).await;

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), AppError> {
        let session = self.state.take_session().await;

        if let Some(session) = session {
            session.disconnect().await;
        }

        Ok(())
    }
}
