// GTP 명령 전송 커맨드 (착수, pass, kata-analyze 시작 등)
use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::game::Color;
use crate::gtp::autoplay;
use crate::state::{AppState, EngineColors};

#[tauri::command]
pub async fn send_gtp_command(
    state: State<'_, AppState>,
    command: String,
) -> Result<String, AppError> {
    // 락을 쥔 채로 세션의 send().await(원격 응답 대기)까지 들고 가면 그동안 다른 모든
    // SSH/GTP 커맨드가 block되므로, Arc만 clone해서 즉시 락을 해제한 뒤 락 밖에서 보낸다.
    let session = state.active_session().await;

    let session = session.ok_or(AppError::NotConnected)?;
    session.send(&command).await
}

/// kata-analyze를 (재)시작한다. 일반 `send_gtp_command`로도 문자열상 똑같이 보낼 수
/// 있지만, 이 전용 커맨드를 통해야만 "지금 이 시점에 게임 트리 어느 노드/어느 색
/// 차례였는지"를 GtpSession에 정확히 기록한 뒤(set_analysis_context) 명령을 보낼 수
/// 있다 - 그래야 이어서 도착하는 "info" 라인들이 올바른 노드에 태깅되어 emit된다
/// (process.rs::AnalysisContext 참고). GameControls.svelte가 분석을 켤 때, 그리고
/// 보드 위치가 바뀔 때마다(사람 착수든 엔진 자동 응수든) 이 커맨드를 다시 호출해
/// 스트림을 새 위치 기준으로 재시작한다.
#[tauri::command]
pub async fn start_kata_analyze(
    interval_centiseconds: u32,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let snapshot = state.game_snapshot();

    let session = state.active_session().await;
    let session = session.ok_or(AppError::NotConnected)?;

    session.set_analysis_context(snapshot.node_id, snapshot.current_turn);
    session
        .send(&format!(
            "kata-analyze {interval_centiseconds} ownership true"
        ))
        .await
}

fn parse_color(s: &str) -> Result<Color, AppError> {
    match s {
        "black" => Ok(Color::Black),
        "white" => Ok(Color::White),
        other => Err(AppError::InvalidInput(format!("알 수 없는 색: {other}"))),
    }
}

/// 메인 화면에서 "KataGo가 해당 색을 자동으로 둘지"를 색상별로 독립적으로 켜고 끈다.
/// 흑/백을 둘 다 켜면 KataGo가 자기 자신과 대국하듯 양쪽을 모두 계속 두고, 둘 다
/// 끄면 자동 착수 없이 기존처럼 사람이 양쪽을 다 둔다.
///
/// 사람의 착수가 이어질 때는 `commands::game::confirm_move`가 매번 자동 응수 여부를
/// 확인하지만, 이 토글 자체를 켜는 시점(예: 흑/백을 막 둘 다 켰을 때)에는 그 뒤로
/// 사람이 아예 착수를 안 할 수도 있으므로, 여기서도 바로 한 번 확인해 지금 당장
/// 엔진 차례면 즉시 시작한다. GTP 콘솔로 보내는 수동 명령은 이 설정과 무관하게 항상
/// 그대로 동작한다.
#[tauri::command]
pub async fn set_engine_color(
    color: String,
    enabled: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    let color = parse_color(&color)?;
    state.set_engine_color(color, enabled);

    if enabled {
        autoplay::request_engine_move_if_needed(state.inner(), &app).await;
    }

    Ok(())
}

#[tauri::command]
pub fn get_engine_colors(state: State<AppState>) -> EngineColors {
    state.engine_colors()
}

#[tauri::command]
pub fn get_komi(state: State<AppState>) -> f64 {
    state.komi()
}

/// Settings에서 덤을 바꿀 때 호출. 값을 저장해두는 것과 별개로, 지금 연결된 엔진이
/// 있으면 바로 `komi <값>`을 보내 즉시 반영한다(안 그러면 다음 접속 때까지 옛 값으로
/// 계산됨). 연결이 없으면 저장만 해두고, connect_ssh가 다음 연결 시 이 값을 그대로
/// 보낸다.
#[tauri::command]
pub async fn set_komi(komi: f64, state: State<'_, AppState>) -> Result<(), AppError> {
    state.set_komi_value(komi);

    let session = state.active_session().await;
    if let Some(session) = session {
        let _ = session.send(&format!("komi {komi}")).await;
    }

    Ok(())
}
