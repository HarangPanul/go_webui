// gtp_line은 여기 없다 - 프런트에서 invoke()로 호출하는 경로가 아니라
// gtp::android_transport::AndroidLocalTransport가 app.katago_local().gtp_line(...)을
// Rust 코드에서 직접 부른다(이 커맨드 레이어를 거치지 않음). GtpLineRequest에
// kata-analyze 스트리밍용 Channel 필드가 생기면서 Deserialize를 derive할 수 없게 됐는데,
// #[tauri::command] 인자는 Deserialize가 필요해서 이 방식의 프런트 노출 자체가 더 이상
// 불가능해졌다 - 애초에 안 쓰던 경로라 그냥 들어냄.
use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::KatagoLocalExt;
use crate::Result;

#[command]
pub(crate) async fn ping<R: Runtime>(
    app: AppHandle<R>,
    payload: PingRequest,
) -> Result<PingResponse> {
    app.katago_local().ping(payload)
}
