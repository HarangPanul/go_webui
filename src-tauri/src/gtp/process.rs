// GTP 세션: 명령 큐잉/FIFO 매칭/kata-analyze 스트리밍 분리 등 GTP 텍스트 프로토콜
// 자체의 규칙만 다루고, 그 반대편이 실제로 무엇인지(원격 SSH 채널의 `katago gtp`
// 프로세스든, 나중에 추가될 다른 트랜스포트든)는 gtp::transport::GtpTransport 뒤로
// 완전히 숨긴다 - SSH 채널 프레이밍/재연결 정책 같은 세부사항은 gtp::ssh_transport에
// 있다.
//
// - 명령 큐: GTP는 보내는 순서대로 응답이 오는 FIFO 프로토콜이므로, send()마다 oneshot을
//   대기열 뒤에 push하고 reader task가 응답을 완성할 때마다 대기열 맨 앞을 resolve.
// - GTP 응답은 `=`/`?`로 시작해 빈 줄로 끝날 때까지 여러 줄일 수 있음 -> 빈 줄을 만날
//   때까지 누적.
// - kata-analyze 스트리밍(`info ` 라인)은 일반 응답 큐에 섞이지 않도록 즉시 분리해서
//   파싱 후 이벤트로 emit.
// - 무응답/연결 끊김 감지 시 트랜스포트의 재연결 정책(GtpTransport::reconnect_policy)에
//   따라 재시도, 명시적 disconnect()는 Notify로 즉시 취소.

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex, Notify};

use crate::error::AppError;
use crate::game::Color;
use crate::gtp::parser;
use crate::gtp::transport::{GtpTransport, ReconnectPolicy, TransportEvent, TransportHandle};
use crate::services::engine_sync;
use crate::state::AppState;

const CONNECTION_STATUS_EVENT: &str = "connection-status";
const KATA_ANALYZE_EVENT: &str = "kata-analyze";

/// kata-analyze 스트림이 (재)시작될 때 GtpSession::set_analysis_context()로 기록해두는
/// "지금 이 스트림이 어느 게임 트리 노드/색 기준으로 진행 중인지". 매 "info" 라인을
/// 이 값과 함께 emit해서, 프런트엔드가 노드별로 결과를 정확히 캐싱할 수 있게 한다.
///
/// 알려진 한계: kata-analyze를 다시 시작할 때(start_kata_analyze) 이 값을 새 값으로
/// 즉시 덮어쓰는데, 그 시점에 이전 스트림이 아직 완전히 멈추지 않았다면(엔진이 인터럽트를
/// 받아 처리하기 전에 이미 큐잉되어 있던 "info" 줄 몇 개) 그 몇 줄이 새 노드/색 기준으로
/// 잘못 태깅될 수 있다. 다음 kata-analyze 갱신 주기(수백ms) 안에 진짜 새 데이터로 바로
/// 덮어써지므로 실질적으로는 눈에 띄지 않지만, 완벽히 경합을 없애려면 "이전 스트림의
/// 종료를 확인한 뒤에만 새 컨텍스트를 적용" 하는 별도의 pending/active 2단계 큐잉이
/// 필요함(현재는 그 정도 정교함이 필요할 만큼의 이득이 없다고 보고 생략).
#[derive(Clone, Copy)]
pub struct AnalysisContext {
    pub node_id: usize,
    pub for_color: Color,
}

// "connection-status" 이벤트 payload. 커맨드 어디에도 인자/반환값으로 나타나지
// 않아 lib.rs에서 `.typ::<>()`로 직접 등록해야 bindings.ts에 타입이 노출된다.
// profile_id는 동시에 여러 프로필이 연결될 수 있어서(여러 서버 동시 연결) 추가됨 -
// 이게 없으면 프런트가 이 상태 변화가 어느 프로필 얘기인지 구분할 수 없다.
#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    profile_id: String,
    status: &'static str,
    message: Option<String>,
}

fn emit_status(app: &AppHandle, profile_id: &str, status: &'static str, message: Option<String>) {
    let _ = app.emit(
        CONNECTION_STATUS_EVENT,
        StatusPayload {
            profile_id: profile_id.to_string(),
            status,
            message,
        },
    );
}

type PendingQueue = StdMutex<VecDeque<oneshot::Sender<Result<String, AppError>>>>;

struct Inner {
    app: AppHandle,
    transport: Arc<dyn GtpTransport>,
    writer: AsyncMutex<Pin<Box<dyn AsyncWrite + Send>>>,
    // 연결을 살아있게 유지하는 동시에, disconnect() 시 실제로 트랜스포트를 끊기 위해
    // 필요 (reader task는 line 채널만 소유하고 handle은 여기서 관리).
    handle: AsyncMutex<Option<Box<dyn TransportHandle>>>,
    pending: PendingQueue,
    reconnect_cancel: Notify,
    stopped: AtomicBool,
    analysis_context: StdMutex<Option<AnalysisContext>>,
    // 지금 이 세션에 kata-analyze가 "켜져 있어야 하는지"와 그 간격.
    // start_kata_analyze가 설정하고 stop_kata_analyze가 지운다(analysis_context와
    // 달리 이건 끄면 반드시 None으로 돌아옴). 재연결 직후 이 값이 Some이면 - 즉
    // 끊기기 직전까지 분석이 켜져 있었다면 - 새로 뜬 세션에도 즉시 kata-analyze를
    // 다시 걸어준다(reconnect_loop 참고). 안 그러면 GameControls.svelte가 보드
    // 위치 변화에만 반응해 재시작하므로, 다음 착수가 생기기 전까지 화면엔 끊기기
    // 직전의 낡은 분석 결과가 그대로 남는다.
    analysis_interval: StdMutex<Option<u32>>,
}

fn fail_all_pending(inner: &Inner, message: &str) {
    let mut pending = inner.pending.lock().unwrap();
    while let Some(tx) = pending.pop_front() {
        let _ = tx.send(Err(AppError::GtpSendFailed(message.to_string())));
    }
}

pub struct GtpSession {
    inner: Arc<Inner>,
}

impl GtpSession {
    pub async fn connect(app: AppHandle, transport: Arc<dyn GtpTransport>) -> Result<Self, AppError> {
        emit_status(&app, transport.profile_id(), "connecting", None);

        let opened = transport
            .open()
            .await
            .inspect_err(|e| emit_status(&app, transport.profile_id(), "error", Some(e.to_string())))?;

        let inner = Arc::new(Inner {
            app: app.clone(),
            transport: transport.clone(),
            writer: AsyncMutex::new(opened.writer),
            handle: AsyncMutex::new(Some(opened.handle)),
            pending: StdMutex::new(VecDeque::new()),
            reconnect_cancel: Notify::new(),
            stopped: AtomicBool::new(false),
            analysis_context: StdMutex::new(None),
            analysis_interval: StdMutex::new(None),
        });

        spawn_reader(inner.clone(), opened.lines);
        emit_status(&app, inner.transport.profile_id(), "connected", None);

        Ok(GtpSession { inner })
    }

    /// kata-analyze를 (재)시작하기 직전에 호출: 이후 도착하는 "info" 라인들을 어느
    /// 게임 트리 노드/색 기준으로 emit할지 기록한다. 실제 명령 전송은 호출자가 이어서
    /// send()로 한다(둘을 분리해두는 건 commands::gtp::start_kata_analyze가 "노드 id를
    /// 정하는 것"과 "명령을 보내는 것" 사이에 다른 잠금 없이 최대한 가깝게 붙여 두기
    /// 위함).
    pub fn set_analysis_context(&self, node_id: usize, for_color: Color) {
        *self.inner.analysis_context.lock().unwrap() = Some(AnalysisContext { node_id, for_color });
    }

    /// start_kata_analyze가 명령을 보내기 직전에 호출: "지금 이 세션에 이 간격으로
    /// kata-analyze가 켜져 있어야 한다"를 기록해둔다 - 재연결 시 되살릴 수 있게.
    pub fn set_analysis_wanted(&self, interval_centiseconds: u32) {
        *self.inner.analysis_interval.lock().unwrap() = Some(interval_centiseconds);
    }

    /// stop_kata_analyze가 호출: 더 이상 분석을 원하지 않음을 기록. 이후 재연결돼도
    /// 자동으로 kata-analyze를 다시 걸지 않는다.
    pub fn clear_analysis_wanted(&self) {
        *self.inner.analysis_interval.lock().unwrap() = None;
    }

    /// 지금 이 세션에 kata-analyze가 켜져 있어야 하는지, 켜져 있어야 한다면 그 간격.
    /// reconnect_loop가 재연결 직후 분석을 되살릴지 판단하는 데 사용.
    pub fn analysis_interval(&self) -> Option<u32> {
        *self.inner.analysis_interval.lock().unwrap()
    }

    /// engine_sync::resync_session_to_history가 komi 등 표준 시퀀스와 함께 보낼,
    /// 이 세션의 트랜스포트가 추가로 필요로 하는 명령들. GtpSession 자신은 그 명령이
    /// 뭘 뜻하는지 모른 채 transport::GtpTransport::extra_resync_commands를 그대로
    /// 전달만 한다(파일 상단 설명 참고).
    pub fn extra_resync_commands(&self) -> Vec<String> {
        self.inner.transport.extra_resync_commands()
    }

    /// GTP 명령을 보내고 응답 전체(여러 줄일 수 있음, 빈 줄 이전까지)를 받아온다.
    pub async fn send(&self, command: &str) -> Result<String, AppError> {
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
                return Err(AppError::GtpSendFailed(format!("GTP 명령 전송 실패: {e}")));
            }
            if let Err(e) = writer.flush().await {
                return Err(AppError::GtpSendFailed(format!("GTP 명령 flush 실패: {e}")));
            }
        }

        rx.await
            .map_err(|_| AppError::GtpSendFailed("연결이 끊겨 응답을 받지 못했습니다".to_string()))?
    }

    pub async fn disconnect(&self) {
        self.inner.stopped.store(true, Ordering::SeqCst);
        self.inner.reconnect_cancel.notify_one();
        fail_all_pending(&self.inner, "연결이 종료되었습니다");

        // 실제 연결을 정리해야 원격 katago 프로세스도(SSH의 경우) 정리되고 reader
        // task도 라인 채널이 끊기는 것을 보고 루프를 빠져나감.
        let handle = self.inner.handle.lock().await.take();
        if let Some(handle) = handle {
            handle.close().await;
        }

        emit_status(&self.inner.app, self.inner.transport.profile_id(), "disconnected", None);
    }
}

fn spawn_reader(inner: Arc<Inner>, mut lines: mpsc::UnboundedReceiver<TransportEvent>) {
    tokio::spawn(async move {
        let mut response_acc = String::new();
        // recv()가 Closed 이벤트 없이 그냥 None을 반환하는 건 정상 흐름에서는 일어나지
        // 않아야 하지만(트랜스포트 쪽 pump task가 항상 Closed를 보내고 끝남), 혹시
        // 그 task가 패닉 등으로 죽는 경우를 대비한 기본값.
        let mut close_reason = "연결이 끊겼습니다".to_string();

        while let Some(event) = lines.recv().await {
            match event {
                TransportEvent::Line(line) => dispatch_line(&inner, &mut response_acc, &line),
                TransportEvent::Closed { reason } => {
                    close_reason = reason;
                    break;
                }
            }
        }

        fail_all_pending(&inner, &close_reason);

        if inner.stopped.load(Ordering::SeqCst) {
            return;
        }

        emit_status(&inner.app, inner.transport.profile_id(), "reconnecting", Some(close_reason));
        reconnect_loop(inner).await;
    });
}

/// kata-analyze 결과에 "어느 노드/색 기준인지"를 얹어 프런트엔드로 보내는 이벤트
/// payload. serde(flatten)으로 KataAnalyzeResult의 필드들이 nodeId/forColor와 같은
/// 레벨에 나란히 실린다(프런트 `analysisStore`가 이 하나의 이벤트를 통째로 캐싱).
// "kata-analyze" 이벤트 payload. StatusPayload와 마찬가지로 커맨드 시그니처에
// 나타나지 않아 lib.rs에서 `.typ::<>()`로 직접 등록한다.
#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KataAnalyzeEvent {
    node_id: usize,
    for_color: Color,
    #[serde(flatten)]
    result: parser::KataAnalyzeResult,
}

/// 한 줄을 처리: kata-analyze 스트리밍 라인이면 즉시 파싱+emit, 아니면 GTP 응답
/// 누적 버퍼에 쌓다가 빈 줄에서 대기열 맨 앞을 resolve.
fn dispatch_line(inner: &Arc<Inner>, response_acc: &mut String, line: &str) {
    if parser::is_analysis_line(line) {
        // 컨텍스트가 아직 한 번도 설정된 적 없으면(이론상 발생하지 않아야 함 - kata-analyze는
        // 항상 set_analysis_context() 직후에만 보내므로) 어느 노드 것인지 알 수 없으니
        // 잘못 태깅하지 않고 그냥 버린다.
        let Some(ctx) = *inner.analysis_context.lock().unwrap() else {
            return;
        };
        if let Some(result) = parser::parse_kata_analyze(line) {
            let event = KataAnalyzeEvent {
                node_id: ctx.node_id,
                for_color: ctx.for_color,
                result,
            };
            let _ = inner.app.emit(KATA_ANALYZE_EVENT, event);
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
    let delay = match inner.transport.reconnect_policy() {
        // 이 트랜스포트는 자동 재연결을 지원하지 않음 - 세션은 끊긴 채로 남고
        // "reconnecting" 대신 이미 emit된 상태 그대로(위 spawn_reader의 "reconnecting")
        // 둔다. 지금은 SSH만 있어 이 경로가 실제로 쓰이지 않지만, 재연결 개념이
        // 다르거나 없는 트랜스포트(예: 로컬 온디바이스 엔진)를 위해 마련해둠.
        ReconnectPolicy::None => return,
        ReconnectPolicy::Retry(delay) => delay,
    };

    loop {
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = inner.reconnect_cancel.notified() => return,
        }

        if inner.stopped.load(Ordering::SeqCst) {
            return;
        }

        match inner.transport.open().await {
            Ok(opened) => {
                *inner.writer.lock().await = opened.writer;
                *inner.handle.lock().await = Some(opened.handle);
                spawn_reader(inner.clone(), opened.lines);
                emit_status(&inner.app, inner.transport.profile_id(), "connected", None);

                // 새로 뜬 katago 프로세스는 내부적으로 텅 빈 보드에서 시작하므로,
                // 끊기기 전까지 진행됐던 실제 대국 상태(보드 크기/덤/수순 전부)를
                // 그대로 재생해 맞춰준다 - 안 그러면 이 세션은 (연결 상태만
                // "connected"로 정상 복구된 것처럼 보일 뿐) 로컬 게임 트리와 완전히
                // 다른 보드를 기준으로 genmove/kata-analyze를 하게 되어 추천수가
                // 뜬금없어 보이는 원인이 된다(services::engine_sync 참고).
                let state = inner.app.state::<AppState>();
                let session = GtpSession {
                    inner: inner.clone(),
                };
                engine_sync::resync_session_to_history(&state, &session).await;

                // 끊기기 직전까지 이 세션에 kata-analyze가 켜져 있었다면(analysis_
                // interval) 지금 다시 걸어준다 - 이 뒤에 이어지는 send()는 kata-analyze
                // 특유의 "다른 입력이 올 때까지 안 끝나는" 스트리밍 명령이라 이 await는
                // 그 세션의 분석이 실제로 멈출 때까지(다음 genmove 등으로 인터럽트될
                // 때까지) 안 끝나는 게 정상이다 - reconnect_loop는 이미 독립된
                // tokio::spawn 태스크라 다른 곳을 막지 않는다.
                engine_sync::resume_analysis_if_wanted(&state, &session).await;

                return;
            }
            Err(e) => {
                // 매 시도 실패 이유를 그대로 버리지 않고 다시 emit - 그래야
                // "reconnecting" 상태에서 멈춰 있을 때 왜 계속 실패하는지(예: engine
                // 명령 자체가 잘못됨) 사용자가 알 수 있다.
                emit_status(&inner.app, inner.transport.profile_id(), "reconnecting", Some(e.to_string()));
                continue;
            }
        }
    }
}
