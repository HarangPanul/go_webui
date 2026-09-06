// GtpSession이 GTP 텍스트 프로토콜의 반대편(원격 SSH 채널, 나중엔 온디바이스 로컬
// 엔진 등)과 실제로 어떻게 연결되는지를 추상화하는 경계. GtpSession 자신은 이 트레잇
// 뒤에서 무엇이 응답하고 있는지 전혀 몰라야 한다 - 명령 큐잉/FIFO 매칭/kata-analyze
// 스트리밍 분리 같은 GTP 프로토콜 규칙만 알면 된다.
//
// 지금은 SshTransport(ssh_transport.rs) 하나뿐이지만, 이 경계 덕분에 나중에
// 로컬(예: Android 온디바이스 엔진) 트랜스포트를 추가할 때 이 파일 아래(GtpSession
// 자체와 그 위의 connection_service/engine_sync/commands::gtp)는 손댈 필요가 없다.

use std::pin::Pin;

use tokio::io::AsyncWrite;
use tokio::sync::mpsc;

use crate::error::AppError;

/// 트랜스포트가 예기치 않게 끊겼을 때 GtpSession이 자동 재연결을 시도할지, 시도한다면
/// 얼마나 기다렸다 재시도할지. SSH는 네트워크가 일시적으로 끊길 수 있어 고정 간격
/// 재시도가 의미 있지만, 트랜스포트에 따라 "연결 끊김" 자체가 다른 의미일 수 있다
/// (예: 프로세스 내 로컬 엔진에는 재시도할 원격 대상이 없을 수 있음).
pub enum ReconnectPolicy {
    /// 자동 재연결을 시도하지 않는다 - 세션은 끊긴 채로 남고 사용자가 다시 연결해야 함.
    None,
    /// 이 간격으로 재연결을 계속 시도한다.
    Retry(std::time::Duration),
}

/// 라인 소스 쪽에서 발생하는 이벤트. GtpSession의 리더 루프는 트랜스포트 종류와
/// 무관하게 이 이벤트만 보고 동작한다.
pub enum TransportEvent {
    /// 개행/`\r`이 이미 제거된 한 줄 - GTP 응답 누적이든 kata-analyze의 "info" 줄이든
    /// 그대로 dispatch_line에 넘기면 됨.
    Line(String),
    /// 트랜스포트가 끊김(정상 종료든 오류든) - reader 루프가 이 이벤트를 받으면 더
    /// 이상 Line이 오지 않는다고 간주하고 빠져나간다.
    Closed { reason: String },
}

/// 명시적 disconnect() 시 실제 연결을 정리하기 위한 핸들. SSH는 세션 종료 패킷을
/// 보내야 원격 katago 프로세스도 함께 정리되지만, 트랜스포트에 따라 아무것도 할 일이
/// 없을 수도 있다(예: 로컬 엔진이면 자원 해제 정도).
#[async_trait::async_trait]
pub trait TransportHandle: Send + Sync {
    async fn close(&self);
}

/// 트랜스포트를 한 번 열어서 얻는 것: GTP 명령을 써넣을 writer, 상대가 보낸 줄을
/// 흘려보내는 채널, 그리고 명시적 disconnect()용 핸들.
pub struct OpenTransport {
    pub writer: Pin<Box<dyn AsyncWrite + Send>>,
    pub lines: mpsc::UnboundedReceiver<TransportEvent>,
    pub handle: Box<dyn TransportHandle>,
}

/// GtpSession이 기대는 트랜스포트 경계. `open()`은 최초 연결과 재연결 양쪽에서 모두
/// 쓰인다 - 재연결은 단순히 같은 트랜스포트에 대해 open()을 다시 호출하는 것과 같다.
#[async_trait::async_trait]
pub trait GtpTransport: Send + Sync {
    /// connection-status 이벤트 등에 실어 보낼 식별자 - 여러 프로필이 동시에 연결될
    /// 수 있으므로 프런트가 이 상태 변화가 어느 프로필 얘기인지 구분하는 데 쓰인다.
    fn profile_id(&self) -> &str;

    async fn open(&self) -> Result<OpenTransport, AppError>;

    /// 기본값은 재연결 안 함 - 재연결이 의미 있는 트랜스포트(SSH 등)만 override.
    fn reconnect_policy(&self) -> ReconnectPolicy {
        ReconnectPolicy::None
    }

    /// (재)연결 직후 engine_sync::resync_session_to_history가 보내는 표준 시퀀스
    /// (boardsize/clear_board/komi/history 재생) 말고, 이 트랜스포트가 추가로 필요로
    /// 하는 명령들 - 예: 로컬 온디바이스 엔진의 max_visits 확장 명령
    /// (gtp::android_transport::AndroidLocalTransport). engine_sync는 이 명령들이
    /// 무엇을 뜻하는지 전혀 모른 채 순서대로 보내기만 한다. 기본값은 없음 - 특정
    /// 명령이 필요한 트랜스포트만 override.
    fn extra_resync_commands(&self) -> Vec<String> {
        Vec::new()
    }
}
