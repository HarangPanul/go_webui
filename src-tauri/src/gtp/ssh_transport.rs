// GtpTransport를 실제 SSH 채널(원격 `katago gtp` 프로세스)로 구현한다. 채널 프레이밍
// (ChannelMessage::Data/ExtendedData/ExitStatus/Eof/Close을 줄 단위 TransportEvent로
// 변환하는 것)과 재연결 정책(고정 간격 재시도)은 전부 SSH 고유의 세부사항이라 여기
// 갇혀 있고, gtp/process.rs(GtpSession)는 이 파일의 존재를 몰라도 된다.
use std::time::Duration;

use tokio::sync::{mpsc, Mutex as AsyncMutex};

use crate::error::AppError;
use crate::gtp::transport::{
    GtpTransport, OpenTransport, ReconnectPolicy, TransportEvent, TransportHandle,
};
use crate::models::server_profile::ServerProfile;
use crate::ssh::client::{Channel, ChannelMessage, ClientHandler, SshSession};

/// 재연결 시도 간격.
const RECONNECT_INTERVAL: Duration = Duration::from_secs(10);

/// stderr 진단 버퍼 상한. katago 시작 로그가 길 수 있어(모델 로딩 등) 전부 붙잡아두지
/// 않고 앞부분만 보관 - 어차피 원인 파악용 요약이면 충분함.
const STDERR_DIAG_LIMIT: usize = 2000;

pub struct SshTransport {
    profile: ServerProfile,
}

impl SshTransport {
    pub fn new(profile: ServerProfile) -> Self {
        SshTransport { profile }
    }
}

#[async_trait::async_trait]
impl GtpTransport for SshTransport {
    fn profile_id(&self) -> &str {
        &self.profile.id
    }

    async fn open(&self) -> Result<OpenTransport, AppError> {
        let ssh = SshSession::connect(
            &self.profile.host,
            self.profile.port,
            &self.profile.username,
            &self.profile.private_key,
            &self.profile.engine_command,
        )
        .await?;

        let (tx, rx) = mpsc::unbounded_channel();
        spawn_channel_pump(ssh.channel, tx);

        Ok(OpenTransport {
            writer: ssh.writer,
            lines: rx,
            handle: Box::new(SshTransportHandle {
                handle: AsyncMutex::new(Some(ssh.handle)),
            }),
        })
    }

    fn reconnect_policy(&self) -> ReconnectPolicy {
        ReconnectPolicy::Retry(RECONNECT_INTERVAL)
    }
}

struct SshTransportHandle {
    handle: AsyncMutex<Option<russh::client::Handle<ClientHandler>>>,
}

#[async_trait::async_trait]
impl TransportHandle for SshTransportHandle {
    async fn close(&self) {
        if let Some(handle) = self.handle.lock().await.take() {
            let _ = handle
                .disconnect(russh::Disconnect::ByApplication, "", "English")
                .await;
        }
    }
}

/// SSH 채널의 raw ChannelMessage 스트림을 읽어 줄 단위 TransportEvent로 변환해 mpsc로
/// 흘려보낸다. GTP 응답은 stdout(Data)에서만 오고, stderr(ExtendedData)는 GTP
/// 프로토콜과 무관한 진단 로그라 별도로 모았다가 연결이 끊겼을 때 이유 설명에만 쓴다.
fn spawn_channel_pump(mut channel: Channel, tx: mpsc::UnboundedSender<TransportEvent>) {
    tokio::spawn(async move {
        let mut line_buffer = String::new();
        let mut stderr_diag = String::new();
        let mut exit_status: Option<u32> = None;

        loop {
            let Some(msg) = channel.wait().await else {
                break;
            };
            match &msg {
                ChannelMessage::Data { data } => {
                    line_buffer.push_str(&String::from_utf8_lossy(data));
                    while let Some(pos) = line_buffer.find('\n') {
                        let line = line_buffer[..pos].trim_end_matches('\r').to_string();
                        line_buffer.drain(..=pos);
                        if tx.send(TransportEvent::Line(line)).is_err() {
                            return; // 받는 쪽(GtpSession)이 이미 사라짐
                        }
                    }
                }
                ChannelMessage::ExtendedData { data, .. } => {
                    if stderr_diag.len() < STDERR_DIAG_LIMIT {
                        stderr_diag.push_str(&String::from_utf8_lossy(data));
                    }
                }
                ChannelMessage::ExitStatus { exit_status: code } => {
                    exit_status = Some(*code);
                }
                ChannelMessage::Eof | ChannelMessage::Close => break,
                _ => {}
            }
        }

        let reason = disconnect_reason(exit_status, &stderr_diag);
        let _ = tx.send(TransportEvent::Closed { reason });
    });
}

/// 채널이 끊긴 이유를 사람이 읽을 수 있는 문자열로 요약. exit status/stderr가 있으면
/// (원격 명령 자체가 실패한 경우 - 잘못된 engine_command 등) 그걸 우선 보여줘야
/// 사용자가 "네트워크 문제로 재연결 중"과 "명령 자체가 잘못됨"을 구분할 수 있다.
fn disconnect_reason(exit_status: Option<u32>, stderr_diag: &str) -> String {
    let trimmed = stderr_diag.trim();
    match (exit_status, trimmed.is_empty()) {
        (Some(code), false) => format!("원격 명령이 종료됨 (exit code {code}): {trimmed}"),
        (Some(code), true) => format!("원격 명령이 종료됨 (exit code {code})"),
        (None, false) => format!("연결이 끊겼습니다: {trimmed}"),
        (None, true) => "연결이 끊겼습니다".to_string(),
    }
}
