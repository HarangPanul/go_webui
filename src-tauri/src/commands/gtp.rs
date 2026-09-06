// GTP 명령 전송 커맨드 (착수, pass, kata-analyze 시작 등)
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::game::Color;
use crate::gtp::process::GtpSession;
use crate::services::engine_sync;
use crate::state::{AppState, EngineAssignment};

#[tauri::command]
#[specta::specta]
pub async fn send_gtp_command(
    state: State<'_, AppState>,
    profile_id: String,
    command: String,
) -> Result<String, AppError> {
    // 락을 쥔 채로 세션의 send().await(원격 응답 대기)까지 들고 가면 그동안 다른 모든
    // SSH/GTP 커맨드가 block되므로, Arc만 clone해서 즉시 락을 해제한 뒤 락 밖에서 보낸다.
    let session = state.session_for_profile(&profile_id).await;

    let session = session.ok_or(AppError::NotConnected)?;
    session.send(&command).await
}

/// kata-analyze를 보낼 세션을 고른다: 지금 차례 색에 엔진이 배정되어 있으면 그
/// 세션을(genmove가 쓸 세션과 같은 곳을 분석해야 착수 직후에도 결과가 이어짐),
/// 아니면(예: 사람 vs 사람 대국에 엔진 하나를 분석 전용으로만 연결해둔 경우) 연결된
/// 세션 아무거나 하나를 폴백으로 쓴다. 여러 서버가 동시에 연결되어 있고 아무 배정도
/// 없을 때 그중 정확히 어느 걸 분석할지는 아직 사용자가 고를 수 없다(추후 개선 대상) -
/// 지금은 "연결된 것 중 하나"면 충분한 단일 연결 사용 패턴을 그대로 지원하는 데 목적이 있다.
async fn session_for_analysis(state: &AppState, current_turn: Color) -> Option<Arc<GtpSession>> {
    if let Some(session) = state.session_for_color(current_turn).await {
        return Some(session);
    }
    state.any_session().await
}

/// kata-analyze를 (재)시작한다. 일반 `send_gtp_command`로도 문자열상 똑같이 보낼 수
/// 있지만, 이 전용 커맨드를 통해야만 "지금 이 시점에 게임 트리 어느 노드/어느 색
/// 차례였는지"를 GtpSession에 정확히 기록한 뒤(set_analysis_context) 명령을 보낼 수
/// 있다 - 그래야 이어서 도착하는 "info" 라인들이 올바른 노드에 태깅되어 emit된다
/// (process.rs::AnalysisContext 참고). GameControls.svelte가 분석을 켤 때, 그리고
/// 보드 위치가 바뀔 때마다(사람 착수든 엔진 자동 응수든) 이 커맨드를 다시 호출해
/// 스트림을 새 위치 기준으로 재시작한다.
#[tauri::command]
#[specta::specta]
pub async fn start_kata_analyze(
    interval_centiseconds: u32,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let snapshot = state.game_snapshot();

    let session = session_for_analysis(&state, snapshot.current_turn).await;
    let session = session.ok_or(AppError::NotConnected)?;

    session.set_analysis_context(snapshot.node_id, snapshot.current_turn);
    session.set_analysis_wanted(interval_centiseconds);
    session
        .send(&format!(
            "kata-analyze {interval_centiseconds} ownership true"
        ))
        .await
}

/// GameControls.svelte가 Analysis/Ownership을 둘 다 끌 때 진행 중이던 kata-analyze
/// 스트림을 멈추기 위해 부르는 커맨드. kata-analyze는 "다른 입력"이 들어와야 멈추는
/// 스트리밍 명령이라 아무 명령이나 하나 보내면 되는데, 그 "아무 명령"을 어느
/// 세션으로 보내야 할지(=지금 분석 중인 세션이 어디인지)는 프런트가 알 수 없으므로
/// (start_kata_analyze와 똑같은 방식으로 세션을 다시 골라) 여기서 직접 계산해 보낸다.
/// 이미 스트림이 멈춰 있거나 연결이 없어도 조용히 무시(best-effort).
#[tauri::command]
#[specta::specta]
pub async fn stop_kata_analyze(state: State<'_, AppState>) -> Result<(), AppError> {
    let current_turn = state.game_snapshot().current_turn;
    if let Some(session) = session_for_analysis(&state, current_turn).await {
        session.clear_analysis_wanted();
        let _ = session.send("name").await;
    }
    Ok(())
}

fn parse_color(s: &str) -> Result<Color, AppError> {
    match s {
        "black" => Ok(Color::Black),
        "white" => Ok(Color::White),
        other => Err(AppError::InvalidInput(format!("알 수 없는 색: {other}"))),
    }
}

/// 메인 화면에서 "흑/백을 각각 어느 연결된 프로필이 자동으로 둘지"를 색상별로
/// 독립적으로 정한다(`profile_id`가 `None`이면 그 색은 사람이 둠). 흑/백에 같은
/// 프로필을 배정하면 KataGo가 자기 자신과 대국하듯 양쪽을 모두 계속 두고, 둘 다
/// `None`이면 자동 착수 없이 기존처럼 사람이 양쪽을 다 둔다.
///
/// 사람의 착수가 이어질 때는 `commands::game::confirm_move`가 매번 자동 응수 여부를
/// 확인하지만, 이 배정 자체를 바꾸는 시점(예: 막 프로필을 배정했을 때)에는 그 뒤로
/// 사람이 아예 착수를 안 할 수도 있으므로, 여기서도 바로 한 번 확인해 지금 당장
/// 엔진 차례면 즉시 시작한다. GTP 콘솔로 보내는 수동 명령은 이 설정과 무관하게 항상
/// 그대로 동작한다.
#[tauri::command]
#[specta::specta]
pub async fn set_engine_assignment(
    color: String,
    profile_id: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    let color = parse_color(&color)?;
    let assigned = profile_id.is_some();
    state.set_engine_assignment(color, profile_id);

    if assigned {
        engine_sync::request_engine_move_if_needed(state.inner(), &app).await;
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_engine_assignment(state: State<AppState>) -> EngineAssignment {
    state.engine_assignment()
}

#[tauri::command]
#[specta::specta]
pub fn get_komi(state: State<AppState>) -> f64 {
    state.komi()
}

/// Settings에서 덤을 바꿀 때 호출. 값을 저장해두는 것과 별개로, 지금 연결된 엔진이
/// 있으면(여러 개 동시에 연결되어 있을 수 있으므로 전부) 바로 `komi <값>`을 보내
/// 즉시 반영한다(안 그러면 다음 접속 때까지 옛 값으로 계산됨). 연결이 없으면 저장만
/// 해두고, connect_ssh가 다음 연결 시 이 값을 그대로 보낸다.
#[tauri::command]
#[specta::specta]
pub async fn set_komi(komi: f64, state: State<'_, AppState>) -> Result<(), AppError> {
    state.set_komi_value(komi);

    for session in state.all_sessions().await {
        let _ = session.send(&format!("komi {komi}")).await;
    }

    Ok(())
}
