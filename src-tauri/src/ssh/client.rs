// russh 기반 SSH 세션 관리: 접속 + in-memory key 인증 + 채널 open 후 엔진 명령 exec까지만
// 담당한다. GTP 프로토콜 자체의 라인 프레이밍/명령 큐잉/kata-analyze 스트리밍 분리는
// 이 레이어의 책임이 아니라 gtp/process.rs가 담당한다.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle};
use russh::ChannelMsg;
use tokio::io::AsyncWrite;

use crate::error::AppError;

/// 호스트가 응답하지 않을 때(방화벽이 SYN을 조용히 버리는 경우 등) "연결 중" 상태로
/// 무한정 멈춰있지 않도록 접속 자체에 상한을 둠.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// SSH 프로토콜 레벨 keepalive 주기. 이만큼 서버로부터 아무것도 못 받으면 russh가
/// 알아서 keepalive 요청을 보내고, keepalive_max(기본 3)번 연속으로 응답이 없으면
/// 연결을 끊어버린다(client::Config 기본값은 keepalive_interval=None, 즉 이 기능
/// 자체가 꺼져 있음).
///
/// 이게 없으면(고치기 전 상태) 흑/백이 서로 다른 서버에서 자동으로 여러 수를 이어
/// 두는 동안(request_engine_move_if_needed) 네트워크가 (모바일 환경에서 흔한 유휴
/// NAT 타임아웃, 기지국 전환 등으로) 아무 신호 없이 조용히 끊겨도 그걸 감지할 방법이
/// 전혀 없었다 - TCP가 FIN/RST 없이 그냥 죽으면 channel.wait()도, 그 결과를 기다리는
/// GtpSession::send()의 genmove 응답 대기도 영원히 멈춰버리고(둘 다 자체 타임아웃이
/// 없음), 대국이 멈춘 이유조차 알 수 없었다(연결 상태는 여전히 "connected"로 보임).
/// keepalive를 켜두면 이런 "조용한 단절"도 결국 진짜 채널 종료로 이어져 이미 있던
/// reconnect_loop(gtp/process.rs)이 정상적으로 재연결을 재시도하게 된다.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

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
    ) -> Result<Self, AppError> {
        let key_pair = russh_keys::decode_secret_key(private_key, None)
            .map_err(|e| AppError::SshAuthFailed(format!("SSH key 디코딩 실패: {e}")))?;

        let config = Arc::new(client::Config {
            keepalive_interval: Some(KEEPALIVE_INTERVAL),
            ..Default::default()
        });
        let mut handle = tokio::time::timeout(
            CONNECT_TIMEOUT,
            client::connect(config, (host, port), ClientHandler),
        )
        .await
        .map_err(|_| AppError::SshConnectFailed(format!("SSH 연결 시간 초과({host}:{port})")))?
        .map_err(|e| AppError::SshConnectFailed(format!("SSH 연결 실패({host}:{port}): {e}")))?;

        let authenticated = handle
            .authenticate_publickey(username, Arc::new(key_pair))
            .await
            .map_err(|e| AppError::SshAuthFailed(format!("SSH 인증 실패: {e}")))?;
        if !authenticated {
            return Err(AppError::SshAuthFailed(
                "SSH 인증 거부됨 (key 또는 username을 확인하세요)".to_string(),
            ));
        }

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| AppError::SshConnectFailed(format!("SSH 채널 open 실패: {e}")))?;
        // SSH "exec" 요청은 로그인 셸이 아니라서 ~/.profile, ~/.bash_profile 같은 로그인
        // 셸 설정 파일을 읽지 않는다 - `~/.local/bin`처럼 PATH를 거기서 추가하는 경우
        // 명령을 못 찾게 된다. `-l`로 로그인 셸을 흉내내되, 그것만으로는 부족한 경우가
        // 흔하다(예: PATH 추가가 ~/.bashrc에 있는데 거기는 "대화형 셸 아니면 return"
        // 가드가 맨 위에 있어 절대 안 읽힘) - 그래서 흔히 쓰이는 사용자 로컬 bin
        // 경로들을 아예 명시적으로 PATH 맨 앞에 붙여 확실하게 만든다.
        let script = format!(r#"export PATH="$HOME/.local/bin:$HOME/bin:$PATH"; {engine_command}"#);
        let login_shell_command = format!("sh -lc {}", shell_quote(&script));
        channel
            .exec(true, login_shell_command.as_str())
            .await
            .map_err(|e| AppError::SshConnectFailed(format!("엔진 명령 실행 실패({engine_command}): {e}")))?;

        let writer: Pin<Box<dyn AsyncWrite + Send>> = Box::pin(channel.make_writer());

        Ok(SshSession {
            handle,
            channel,
            writer,
        })
    }
}

/// sh 명령줄에 안전하게 끼워 넣기 위한 단일 인용(single-quote) 이스케이프.
/// 작은따옴표 안에서는 어떤 문자도 특수 취급되지 않으므로, 원문 안의 `'`만
/// `'\''`(따옴표 닫고 이스케이프된 따옴표 하나 넣고 다시 따옴표 열기)로 바꿔주면 된다.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::shell_quote;

    #[test]
    fn quotes_plain_command() {
        assert_eq!(shell_quote("katago gtp"), "'katago gtp'");
    }

    #[test]
    fn escapes_embedded_single_quote() {
        // 이스케이프 후 sh가 그대로 이어붙여 원문을 복원할 수 있어야 함:
        // 'it'\''s' -> it's
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }
}

// gtp/process.rs에서 타입을 짧게 참조하기 위해 재노출
pub use russh::client::Msg;
pub type Channel = russh::Channel<Msg>;
pub type ChannelMessage = ChannelMsg;
