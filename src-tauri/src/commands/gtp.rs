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

/// kata-analyze를 보낼 세션을 고른다: 사용자가 Settings에서 Analysis 엔진을 직접
/// 지정해뒀으면(state.analysis_engine) 그 세션을 최우선으로 쓴다 - self-play처럼
/// 흑/백 둘 다 같은 서버에 배정해 genmove가 끊임없이 도는 상황에서는 그 서버가
/// kata-analyze도 함께 맡으면 GTP가 한 세션에서 genmove/kata-analyze를 동시에 못 하는
/// 특성상 분석이 사실상 전혀 나오지 않기 때문(engine_sync::request_engine_move_if_needed의
/// 매 genmove가 곧바로 kata-analyze를 인터럽트해버림) - 다른 서버를 하나 더 연결해
/// 이걸로 지정해두면 genmove와 분석이 서로 다른 세션을 써서 서로 끊지 않는다.
///
/// 지정된 게 없거나 그 프로필이 연결되어 있지 않으면, 지금 차례 색에 엔진이
/// 배정되어 있는 세션을(genmove가 쓸 세션과 같은 곳을 분석해야 착수 직후에도 결과가
/// 이어짐), 그것도 없으면 연결된 세션 아무거나 하나를 폴백으로 쓴다.
///
/// profile_id도 함께 돌려준다 - start_kata_analyze가 이걸 state.set_analyzing_profile로
/// 기록해둬야, 그 사이 배정/차례가 바뀌어도 stop_kata_analyze가 (다시 계산하지 않고)
/// 정확히 이 세션을 찾아 인터럽트를 보낼 수 있다.
async fn session_for_analysis(state: &AppState, current_turn: Color) -> Option<(String, Arc<GtpSession>)> {
    if let Some(id) = state.analysis_engine() {
        if let Some(session) = state.session_for_profile(&id).await {
            return Some((id, session));
        }
    }
    if let Some(found) = state.session_for_color_with_id(current_turn).await {
        return Some(found);
    }
    state.any_session_with_id().await
}

#[tauri::command]
#[specta::specta]
pub fn get_analysis_engine(state: State<AppState>) -> Option<String> {
    state.analysis_engine()
}

/// Analysis/Ownership에 쓸 세션을 사용자가 직접 지정(또는 해제, None)한다. 값을
/// 저장해둘 뿐 여기서 바로 스트림을 다시 걸지는 않는다 - 이미 분석이 켜져 있는
/// 동안 지정을 바꾸면 프런트(GameControls.svelte)가 이어서 start_kata_analyze를 다시
/// 호출해 즉시 새 세션으로 전환한다(엔진 배정을 바꿀 때 곧장 자동 응수를 시도하는
/// set_engine_assignment와 같은 패턴).
#[tauri::command]
#[specta::specta]
pub fn set_analysis_engine(profile_id: Option<String>, state: State<AppState>) {
    state.set_analysis_engine(profile_id);
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

    let found = session_for_analysis(&state, snapshot.current_turn).await;
    let (profile_id, session) = found.ok_or(AppError::NotConnected)?;

    // 방금 고른 세션이 지금까지 분석 중이던 세션과 다르면(사용자가 GameControls의
    // Analysis 엔진 드롭다운으로 직접 바꿨을 때만 일어남 - 보통의 재시작은 항상 같은
    // 세션이라 이 분기를 안 탐) 그 이전 세션에도 인터럽트를 보내 스트림을 멈춰야
    // 한다. 안 그러면 이전 세션이 아무도 안 막았으니 계속 kata-analyze를 돌리면서,
    // 같은 노드 id에 대해 새 세션과 서로 다른 결과를 각자의 갱신 주기마다 번갈아
    // emit해버린다 - analysisStore는 노드 id 하나에 결과 하나만 캐싱하므로
    // (analysis.svelte.ts 참고) 화면이 두 엔진의 의견 사이에서 계속 요동치는 것으로
    // 보였다(go_webui 세션에서 사용자가 직접 보고).
    if let Some(previous_id) = state.analyzing_profile() {
        if previous_id != profile_id {
            if let Some(previous_session) = state.session_for_profile(&previous_id).await {
                previous_session.clear_analysis_wanted();
                let _ = previous_session.send("name").await;
            }
        }
    }

    state.set_analyzing_profile(Some(profile_id));
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
/// 세션으로 보내야 할지는 start_kata_analyze가 state.set_analyzing_profile로 이미
/// 기록해둔 profile_id를 그대로 다시 찾아 쓴다 - session_for_analysis로 다시
/// 계산하지 않는다: 그 사이 engine_assignment나 차례 색이 바뀌면 여기서 다시 계산한
/// 세션이 실제로 분석 중이던 세션과 달라질 수 있고, 그러면 진짜 분석 중이던 세션은
/// 아무 인터럽트도 못 받아 스트림이 계속 남아버린다(엔진 리소스가 계속 소모됨).
/// 이미 스트림이 멈춰 있거나(analyzing_profile이 None) 그 세션이 연결이 끊겼어도
/// 조용히 무시(best-effort).
#[tauri::command]
#[specta::specta]
pub async fn stop_kata_analyze(state: State<'_, AppState>) -> Result<(), AppError> {
    let Some(profile_id) = state.take_analyzing_profile() else {
        return Ok(());
    };
    if let Some(session) = state.session_for_profile(&profile_id).await {
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
