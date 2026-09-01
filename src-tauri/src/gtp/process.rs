// 원격 서버에서 실행 중인 `katago gtp` 프로세스의 SSH 채널 I/O 핸들.
//
// - 명령 큐: GTP는 보내는 순서대로 응답이 오는 FIFO 프로토콜이므로, send()마다 oneshot을
//   대기열 뒤에 push하고 reader task가 응답을 완성할 때마다 대기열 맨 앞을 resolve.
// - GTP 응답은 `=`/`?`로 시작해 빈 줄로 끝날 때까지 여러 줄일 수 있음 -> 빈 줄을 만날
//   때까지 누적.
// - kata-analyze 스트리밍(`info ` 라인)은 일반 응답 큐에 섞이지 않도록 즉시 분리해서
//   파싱 후 이벤트로 emit.
// - 무응답/연결 끊김 감지 시 10초 간격으로 재연결 재시도, 명시적 disconnect()는
//   Notify로 즉시 취소.

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::{oneshot, Mutex as AsyncMutex, Notify};

use crate::gtp::parser;
use crate::models::server_profile::ServerProfile;
use crate::ssh::client::{Channel, ChannelMessage, ClientHandler, SshSession};

const CONNECTION_STATUS_EVENT: &str = "connection-status";
const KATA_ANALYZE_EVENT: &str = "kata-analyze";

#[derive(Serialize, Clone)]
struct StatusPayload {
    status: &'static str,
    message: Option<String>,
}

fn emit_status(app: &AppHandle, status: &'static str, message: Option<String>) {
    let _ = app.emit(CONNECTION_STATUS_EVENT, StatusPayload { status, message });
}

type PendingQueue = StdMutex<VecDeque<oneshot::Sender<Result<String, String>>>>;

struct Inner {
    app: AppHandle,
    profile: ServerProfile,
    writer: AsyncMutex<Pin<Box<dyn AsyncWrite + Send>>>,
    // 연결을 살아있게 유지하는 동시에, disconnect() 시 실제로 SSH 세션을 끊기 위해
    // 필요 (reader task는 channel만 소유하고 handle은 여기서 관리).
    ssh_handle: AsyncMutex<Option<russh::client::Handle<ClientHandler>>>,
    pending: PendingQueue,
    reconnect_cancel: Notify,
    stopped: AtomicBool,
}

fn fail_all_pending(inner: &Inner, message: &str) {
    let mut pending = inner.pending.lock().unwrap();
    while let Some(tx) = pending.pop_front() {
        let _ = tx.send(Err(message.to_string()));
    }
}

pub struct GtpSession {
    inner: Arc<Inner>,
}

impl GtpSession {
    pub async fn connect(app: AppHandle, profile: ServerProfile) -> Result<Self, String> {
        emit_status(&app, "connecting", None);

        let ssh = SshSession::connect(
            &profile.host,
            profile.port,
            &profile.username,
            &profile.private_key,
            &profile.engine_command,
        )
        .await
        .map_err(|e| {
            emit_status(&app, "error", Some(e.clone()));
            e
        })?;

        let inner = Arc::new(Inner {
            app: app.clone(),
            profile,
            writer: AsyncMutex::new(ssh.writer),
            ssh_handle: AsyncMutex::new(Some(ssh.handle)),
            pending: StdMutex::new(VecDeque::new()),
            reconnect_cancel: Notify::new(),
            stopped: AtomicBool::new(false),
        });

        spawn_reader(inner.clone(), ssh.channel);
        emit_status(&app, "connected", None);

        Ok(GtpSession { inner })
    }

    /// GTP 명령을 보내고 응답 전체(여러 줄일 수 있음, 빈 줄 이전까지)를 받아온다.
    pub async fn send(&self, command: &str) -> Result<String, String> {
        let (tx, rx) = oneshot::channel();

        {
            // writer 락을 쥔 채로 대기열 push와 실제 전송을 같은 critical section
            // 안에서 처리해야 한다 - 그래야 send()가 동시에 여러 번 호출돼도
            // "대기열에 쌓인 순서 == 실제로 와이어에 나간 순서"가 보장됨(GTP는 FIFO라
            // 순서가 어긋나면 엉뚱한 응답이 엉뚱한 호출자에게 감).
            let mut writer = self.inner.writer.lock().await;
            self.inner.pending.lock().unwrap().push_back(tx);

            let line = format!("{command}\n");
            if let Err(e) = writer.write_all(line.as_bytes()).await {
                // 전송 자체가 실패했으니 방금 push한 항목(반드시 대기열 맨 뒤에 있음 -
                // 이 writer 락을 쥐고 있는 동안 다른 send()는 push할 수 없으므로)을 제거해
                // 다음 응답이 엉뚱한 호출자에게 매칭되지 않게 함.
                self.inner.pending.lock().unwrap().pop_back();
                return Err(format!("GTP 명령 전송 실패: {e}"));
            }
            if let Err(e) = writer.flush().await {
                return Err(format!("GTP 명령 flush 실패: {e}"));
            }
        }

        rx.await
            .map_err(|_| "연결이 끊겨 응답을 받지 못했습니다".to_string())?
    }

    pub async fn disconnect(&self) {
        self.inner.stopped.store(true, Ordering::SeqCst);
        self.inner.reconnect_cancel.notify_one();
        fail_all_pending(&self.inner, "연결이 종료되었습니다");

        // 실제로 SSH 세션을 끊어야 원격 katago 프로세스도 정리되고 reader task도
        // channel EOF/Close를 받아 루프를 빠져나감.
        let handle = self.inner.ssh_handle.lock().await.take();
        if let Some(handle) = handle {
            let _ = handle
                .disconnect(russh::Disconnect::ByApplication, "", "English")
                .await;
        }

        emit_status(&self.inner.app, "disconnected", None);
    }
}

/// stderr 진단 버퍼 상한. katago 시작 로그가 길 수 있어(모델 로딩 등) 전부
/// 붙잡아두지 않고 앞부분만 보관 - 어차피 원인 파악용 요약이면 충분함.
const STDERR_DIAG_LIMIT: usize = 2000;

fn spawn_reader(inner: Arc<Inner>, mut channel: Channel) {
    tokio::spawn(async move {
        let mut line_buffer = String::new();
        let mut response_acc = String::new();
        // 원격 프로세스의 stderr(ExtendedData)는 GTP 프로토콜과 무관한 로그이므로
        // stdout 파싱 버퍼(line_buffer)에 섞지 않고 진단용으로만 별도 보관한다.
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
                        dispatch_line(&inner, &mut response_acc, &line);
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
        fail_all_pending(&inner, &reason);

        if inner.stopped.load(Ordering::SeqCst) {
            return;
        }

        emit_status(&inner.app, "reconnecting", Some(reason));
        reconnect_loop(inner).await;
    });
}

/// 채널이 끊긴 이유를 사람이 읽을 수 있는 문자열로 요약. exit status/stderr가
/// 있으면(원격 명령 자체가 실패한 경우 - 잘못된 engine_command 등) 그걸 우선
/// 보여줘야 사용자가 "네트워크 문제로 재연결 중"과 "명령 자체가 잘못됨"을 구분할 수
/// 있다.
fn disconnect_reason(exit_status: Option<u32>, stderr_diag: &str) -> String {
    let trimmed = stderr_diag.trim();
    match (exit_status, trimmed.is_empty()) {
        (Some(code), false) => format!("원격 명령이 종료됨 (exit code {code}): {trimmed}"),
        (Some(code), true) => format!("원격 명령이 종료됨 (exit code {code})"),
        (None, false) => format!("연결이 끊겼습니다: {trimmed}"),
        (None, true) => "연결이 끊겼습니다".to_string(),
    }
}

/// 한 줄을 처리: kata-analyze 스트리밍 라인이면 즉시 파싱+emit, 아니면 GTP 응답
/// 누적 버퍼에 쌓다가 빈 줄에서 대기열 맨 앞을 resolve.
fn dispatch_line(inner: &Arc<Inner>, response_acc: &mut String, line: &str) {
    if parser::is_analysis_line(line) {
        if let Some(result) = parser::parse_kata_analyze(line) {
            let _ = inner.app.emit(KATA_ANALYZE_EVENT, result);
        }
        return;
    }

    if line.is_empty() {
        if !response_acc.is_empty() {
            let response = std::mem::take(response_acc);
            if let Some(tx) = inner.pending.lock().unwrap().pop_front() {
                let _ = tx.send(Ok(response));
            }
        }
        return;
    }

    if !response_acc.is_empty() {
        response_acc.push('\n');
    }
    response_acc.push_str(line);
}

async fn reconnect_loop(inner: Arc<Inner>) {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(10)) => {}
            _ = inner.reconnect_cancel.notified() => return,
        }

        if inner.stopped.load(Ordering::SeqCst) {
            return;
        }

        match SshSession::connect(
            &inner.profile.host,
            inner.profile.port,
            &inner.profile.username,
            &inner.profile.private_key,
            &inner.profile.engine_command,
        )
        .await
        {
            Ok(ssh) => {
                *inner.writer.lock().await = ssh.writer;
                *inner.ssh_handle.lock().await = Some(ssh.handle);
                spawn_reader(inner.clone(), ssh.channel);
                emit_status(&inner.app, "connected", None);
                return;
            }
            Err(e) => {
                // 매 시도 실패 이유를 그대로 버리지 않고 다시 emit - 그래야
                // "reconnecting" 상태에서 멈춰 있을 때 왜 계속 실패하는지(예: engine
                // 명령 자체가 잘못됨) 사용자가 알 수 있다.
                emit_status(&inner.app, "reconnecting", Some(e));
                continue;
            }
        }
    }
}
