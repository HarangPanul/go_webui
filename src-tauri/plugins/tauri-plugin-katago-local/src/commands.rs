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

#[command]
pub(crate) async fn gtp_line<R: Runtime>(
    app: AppHandle<R>,
    payload: GtpLineRequest,
) -> Result<GtpLineResponse> {
    app.katago_local().gtp_line(payload)
}
