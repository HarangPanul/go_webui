// SSH 연결/해제 로직: russh 세션 수립(ssh::client) 후 GTP 프로세스 기동(gtp::process)까지
// 담당한다. commands/ssh.rs는 State/AppHandle을 받아 이 서비스를 구성하고 위임하기만 함.
//
// 여러 프로필을 동시에 연결해둘 수 있다(state.sessions - 프로필 id를 key로 하는 맵).
// "이 프로필을 연결한다"와 "이 프로필이 흑/백 중 뭘 둔다"는 완전히 분리된 개념 -
// 여기서는 전자만 다루고, 후자(engine_assignment)는 메인 화면에서 별도로 정한다
// (commands::gtp::set_engine_assignment 참고).
use std::sync::Arc;

use tauri::AppHandle;

use crate::error::AppError;
use crate::gtp::android_transport::AndroidLocalTransport;
use crate::gtp::process::GtpSession;
use crate::gtp::ssh_transport::SshTransport;
use crate::gtp::transport::GtpTransport;
use crate::models::server_profile::ProfileKind;
use crate::services::engine_sync;
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
        // 이미 연결되어 있으면 그대로 둔다 - 흑/백에 같은 프로필을 배정해도(메인
        // 화면에서 각각 "연결"을 누르는 게 아니라 이미 연결된 프로필 중 고르는
        // 방식이라 실제로는 잘 안 생기지만) 중복 SSH 연결/katago 프로세스가 뜨지
        // 않게 하는 안전장치.
        if self.state.is_connected(&profile_id).await {
            return Ok(());
        }

        let profile = keystore::get(&app, &profile_id)?
            .ok_or_else(|| AppError::ProfileNotFound(profile_id.clone()))?;

        if profile.has_passphrase {
            // PLAN.md 결정사항: passphrase 걸린 key는 1차 버전 미지원, 경고만 표시
            return Err(AppError::PassphraseUnsupported);
        }

        let id = profile.id.clone();
        let transport: Arc<dyn GtpTransport> = match profile.kind {
            ProfileKind::Ssh => Arc::new(SshTransport::new(profile)),
            ProfileKind::Local => Arc::new(AndroidLocalTransport::new(app.clone(), id.clone())),
        };
        let session = GtpSession::connect(app, transport).await?;

        // 새로 뜬 katago 프로세스는 덤도 KataGo 자체 기본값이고 보드도 항상 텅 비어
        // 있으므로, 지금까지의 실제 대국 상태(덤 + 이미 진행된 수순 전부)를 그대로
        // 맞춰준다 - 대국이 시작되기 전에 미리 연결해두는 흔한 경우엔 수순이
        // 비어있어 사실상 덤만 반영하는 것과 같지만, 이미 몇 수 진행된 뒤에 엔진을
        // 새로 연결하는 경우엔 이게 없으면 그 세션이 로컬 보드와 완전히 다른(빈)
        // 보드를 기준으로 genmove/kata-analyze를 하게 된다. best-effort: 이 시점에
        // 실패해도 연결 자체는 그대로 둔다.
        engine_sync::resync_session_to_history(self.state, &session).await;

        self.state.insert_session(id, Arc::new(session)).await;

        Ok(())
    }

    pub async fn disconnect(&self, profile_id: String) -> Result<(), AppError> {
        let session = self.state.remove_session(&profile_id).await;
        // 이 프로필이 흑/백 어느 쪽에 배정돼 있었든 풀어준다 - 연결이 끊긴 프로필이
        // 계속 배정된 채로 남아 그 색이 조용히 아무도 안 두는 상태가 되지 않도록.
        self.state.clear_engine_assignment_for(&profile_id);

        if let Some(session) = session {
            session.disconnect().await;
        }

        Ok(())
    }
}
