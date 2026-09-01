// russh 기반 SSH 세션 관리: 접속 + in-memory key 인증 + 채널 open 후 엔진 명령 exec까지만
// 담당한다. GTP 프로토콜 자체의 라인 프레이밍/명령 큐잉/kata-analyze 스트리밍 분리는
// 이 레이어의 책임이 아니라 gtp/process.rs가 담당한다.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle};
use russh::ChannelMsg;
use tokio::io::AsyncWrite;

/// 호스트가 응답하지 않을 때(방화벽이 SYN을 조용히 버리는 경우 등) "연결 중" 상태로
/// 무한정 멈춰있지 않도록 접속 자체에 상한을 둠.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// russh client::Handler 구현체. 지금은 인증 배너/채널 이벤트를 별도로 처리할 필요가
/// 없어 상태 없는 unit struct.
pub struct ClientHandler;

#[async_trait::async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO(보안 하드닝): host key 검증(known_hosts / TOFU)이 아직 없어 모든 서버 키를
        // 무조건 수락한다. MITM 방지가 필요해지면 이 지점에서 프로필별로 저장된 지문과
        // 비교하도록 구현.
        Ok(true)
    }
}

/// 연결된 SSH 세션 + 엔진 프로세스가 exec된 채널의 I/O 핸들.
/// `handle`은 연결이 살아있는 동안 계속 들고 있어야 하고(drop되면 세션 종료),
/// `channel`은 읽기(`wait()`) 전용으로, `writer`는 쓰기 전용으로 분리해서 넘긴다.
pub struct SshSession {
    pub handle: Handle<ClientHandler>,
    pub channel: russh::Channel<client::Msg>,
    pub writer: Pin<Box<dyn AsyncWrite + Send>>,
}

impl SshSession {
    pub async fn connect(
        host: &str,
        port: u16,
        username: &str,
        private_key: &str,
        engine_command: &str,
    ) -> Result<Self, String> {
        let key_pair = russh_keys::decode_secret_key(private_key, None)
            .map_err(|e| format!("SSH key 디코딩 실패: {e}"))?;

        let config = Arc::new(client::Config::default());
        let mut handle = tokio::time::timeout(
            CONNECT_TIMEOUT,
            client::connect(config, (host, port), ClientHandler),
        )
        .await
        .map_err(|_| format!("SSH 연결 시간 초과({host}:{port})"))?
        .map_err(|e| format!("SSH 연결 실패({host}:{port}): {e}"))?;

        let authenticated = handle
            .authenticate_publickey(username, Arc::new(key_pair))
            .await
            .map_err(|e| format!("SSH 인증 실패: {e}"))?;
        if !authenticated {
            return Err("SSH 인증 거부됨 (key 또는 username을 확인하세요)".to_string());
        }

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| format!("SSH 채널 open 실패: {e}"))?;
        channel
            .exec(true, engine_command)
            .await
            .map_err(|e| format!("엔진 명령 실행 실패({engine_command}): {e}"))?;

        let writer: Pin<Box<dyn AsyncWrite + Send>> = Box::pin(channel.make_writer());

        Ok(SshSession {
            handle,
            channel,
            writer,
        })
    }
}

// gtp/process.rs에서 타입을 짧게 참조하기 위해 재노출
pub use russh::client::Msg;
pub type Channel = russh::Channel<Msg>;
pub type ChannelMessage = ChannelMsg;
