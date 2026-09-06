// Android 온디바이스 KataGo 엔진(tauri-plugin-katago-local)을 GtpTransport로 감싼다.
// SSH와 달리 "연결"이라는 개념이 없다 - 항상 같은 프로세스 안의 JNI 호출이라, 네트워크
// 재연결에 대응하는 개념 자체가 없다(reconnect_policy는 트레잇 기본값 None을 그대로
// 씀). 매 GTP 명령은 플러그인의 gtpLine 커맨드 하나로 왕복시킨다.
//
// run_mobile_plugin은 동기/블로킹 호출이라(내부적으로 std::sync::mpsc::channel의
// recv()로 JNI 콜백을 기다림) tokio 런타임 워커 스레드를 막지 않도록 매번
// spawn_blocking으로 감싼다.
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tauri::AppHandle;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc;

use crate::error::AppError;
use crate::gtp::transport::{GtpTransport, OpenTransport, TransportEvent, TransportHandle};
use tauri_plugin_katago_local::{GtpLineRequest, KatagoLocalExt};

pub struct AndroidLocalTransport {
    app: AppHandle,
    profile_id: String,
}

impl AndroidLocalTransport {
    pub fn new(app: AppHandle, profile_id: String) -> Self {
        AndroidLocalTransport { app, profile_id }
    }
}

#[async_trait::async_trait]
impl GtpTransport for AndroidLocalTransport {
    fn profile_id(&self) -> &str {
        &self.profile_id
    }

    async fn open(&self) -> Result<OpenTransport, AppError> {
        // 실제로 "여는" 작업은 없음(항상 같은 프로세스) - 대신 한 번 명령을 보내봐서
        // 이 플랫폼이 애초에 지원되지 않으면(desktop.rs의 UnsupportedPlatform) 연결
        // 시점에 바로 에러로 드러나게 한다. 그러지 않으면 "연결됨"으로 보였다가 첫
        // GTP 명령에서야 실패해 원인을 알기 어렵다.
        let app = self.app.clone();
        tokio::task::spawn_blocking(move || {
            app.katago_local().gtp_line(GtpLineRequest {
                line: "name".to_string(),
            })
        })
        .await
        .map_err(|e| AppError::GtpSendFailed(format!("로컬 엔진 호출 실패: {e}")))?
        .map_err(|e| AppError::GtpSendFailed(format!("로컬 엔진을 사용할 수 없습니다: {e}")))?;

        let (tx, rx) = mpsc::unbounded_channel();
        let writer = LocalWriter {
            app: self.app.clone(),
            buf: Vec::new(),
            tx,
        };

        Ok(OpenTransport {
            writer: Box::pin(writer),
            lines: rx,
            handle: Box::new(NoopTransportHandle),
        })
    }

    /// max_visits는 이 로컬 엔진(GtpShell.kt)만 아는 확장 명령이라 engine_sync 같은
    /// 범용 코드는 이 이름 자체를 몰라야 한다 - 값의 실제 저장소도 AppState가 아니라
    /// 이 플러그인 자신(KatagoLocal::max_visits, commands/local_engine.rs가 프런트에
    /// 노출)이라 여기서 바로 읽는다.
    fn extra_resync_commands(&self) -> Vec<String> {
        let max_visits = self.app.katago_local().max_visits();
        vec![format!("max_visits {max_visits}")]
    }
}

/// disconnect() 시 정리할 실제 연결이 없음 - 로컬 엔진은 끊어야 할 소켓/프로세스가
/// 없는 같은 프로세스 안의 호출이라 아무 것도 하지 않는다.
struct NoopTransportHandle;

#[async_trait::async_trait]
impl TransportHandle for NoopTransportHandle {
    async fn close(&self) {}
}

/// GTP 명령을 한 줄씩 받아 gtpLine 플러그인 커맨드로 왕복시키고, 응답을
/// TransportEvent::Line으로 흘려보내는 AsyncWrite. GtpSession::send()는 매 호출마다
/// 이전 명령의 응답(oneshot)을 기다린 뒤에야 다음 줄을 쓰므로, 동시에 두 줄 이상이
/// 이 writer에 들어올 일이 없다 - 그래서 줄마다 독립된 tokio::spawn을 띄워도 FIFO
/// 순서가 자연히 유지된다(다음 줄이 아직 안 왔다는 것 자체가 이전 응답이 이미
/// 끝났다는 뜻).
struct LocalWriter {
    app: AppHandle,
    buf: Vec<u8>,
    tx: mpsc::UnboundedSender<TransportEvent>,
}

impl AsyncWrite for LocalWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        this.buf.extend_from_slice(buf);

        while let Some(pos) = this.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = this.buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line)
                .trim_end_matches(['\n', '\r'])
                .to_string();

            let app = this.app.clone();
            let tx = this.tx.clone();
            tokio::spawn(async move {
                let result =
                    tokio::task::spawn_blocking(move || app.katago_local().gtp_line(GtpLineRequest { line }))
                        .await;

                let response = match result {
                    Ok(Ok(resp)) => resp.response,
                    Ok(Err(e)) => format!("? {e}"),
                    Err(join_err) => format!("? 내부 오류: {join_err}"),
                };

                for line in response.lines() {
                    let _ = tx.send(TransportEvent::Line(line.to_string()));
                }
                // GTP 응답 블록은 빈 줄로 끝나야 GtpSession::dispatch_line이 이 응답을
                // 완결된 것으로 보고 대기열 맨 앞을 resolve한다.
                let _ = tx.send(TransportEvent::Line(String::new()));
            });
        }

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
