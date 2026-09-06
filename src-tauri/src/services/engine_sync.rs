// 로컬 게임 트리 <-> GTP 엔진(들) 자동 동기화: 사람 착수를 연결된 모든 엔진에
// 미러링하고, 흑/백 각각 배정된 엔진(state.engine_assignment)이 다음 차례를 맡고
// 있으면 자동으로 genmove를 요청해 로컬 게임 트리에도 반영한다.
//
// 흑/백에 서로 다른 서버가 배정될 수 있으므로, 어느 세션이 지금 차례를 맡고 있든
// 항상 전체 보드 히스토리를 알고 있어야 나중에 자기 차례가 왔을 때 정확한 수를 낼 수
// 있다 - 그래서 착수/pass/undo는 "지금 차례 색의 세션"이 아니라 "연결된 세션 전부"에
// 미러링한다.
//
// 두 곳에서 호출된다:
// - commands::game::confirm_move: 사람이 막 착수했을 때, 그 수를 미러링한 뒤 이어서.
// - commands::gtp::set_engine_assignment: 흑/백에 새 프로필을 배정했을 때 - 이 경우
//   사람의 착수가 전혀 없을 수도 있으므로(예: 막 배정한 직후) 별도의 시작점이 필요하다.
use tauri::{AppHandle, Emitter};

use crate::game::{Color, MoveInfo};
use crate::gtp::coords;
use crate::gtp::process::GtpSession;
use crate::state::AppState;

/// 연결된 모든 세션에 같은 명령을 best-effort로 보낸다. 개별 세션이 실패해도(연결
/// 끊김, 보드 상태 어긋남 등) 다른 세션과 로컬 대국에는 영향이 없어야 하므로 에러는
/// 무시한다(연결 상태는 각 세션이 별도로 connection-status 이벤트로 알림).
async fn mirror_to_all_sessions(state: &AppState, cmd: &str) {
    for session in state.all_sessions().await {
        let _ = session.send(cmd).await;
    }
}

/// 사람의 착수를 연결된 모든 GTP 엔진에 `play`로 반영(미러링)하고, 이어서
/// [`request_engine_move_if_needed`]로 자동 응수를 이어간다.
pub async fn sync_move_to_engine(
    state: &AppState,
    app: &AppHandle,
    color: Color,
    x: usize,
    y: usize,
) {
    let size = state.with_game(|game| game.size());
    let vertex = coords::to_vertex(x, y, size);
    mirror_to_all_sessions(state, &format!("play {} {}", color.gtp_letter(), vertex)).await;

    request_engine_move_if_needed(state, app).await;
}

/// 사람의 pass를 연결된 모든 GTP 엔진에 `play <color> pass`로 반영(미러링)하고,
/// 이어서 [`request_engine_move_if_needed`]로 자동 응수를 이어간다.
pub async fn sync_pass_to_engine(state: &AppState, app: &AppHandle, color: Color) {
    mirror_to_all_sessions(state, &format!("play {} pass", color.gtp_letter())).await;

    request_engine_move_if_needed(state, app).await;
}

/// go_back/remove_last_move로 로컬 게임 트리를 한 수 되돌릴 때 연결된 모든 GTP
/// 엔진에도 `undo`를 보내 같은 수만큼 되돌린다. 이 미러링이 빠지면 로컬 트리와 엔진
/// 내부 보드가 어긋나서(엔진은 되돌리기 전 수를 그대로 들고 있음) 그 뒤 시작하는
/// kata-analyze가 실제로는 로컬이 생각하는 위치보다 한 수 앞선 위치를 분석하게 되고,
/// 그 결과의 기준 색(엔진 입장의 pla)이 로컬에서 기대하는 색과 정확히 반대가 되어
/// winrate bar/ownership이 뒤집혀 보이는 문제가 생긴다.
pub async fn sync_undo_to_engine(state: &AppState) {
    mirror_to_all_sessions(state, "undo").await;
}

/// go_forward로 로컬 게임 트리에서 이미 존재하는 자식 노드로 이동할 때, 그 수를
/// 연결된 모든 GTP 엔진에도 `play`로 그대로 재생해 로컬/엔진 보드를 다시 맞춘다.
/// go_back으로 되돌아갔다가(엔진에는 이미 `undo`가 반영되어 있음) 다시 앞으로 갈 때
/// 필요한 동기화. sync_move_to_engine과 달리 이 수는 이미 로컬 트리에 존재하던 걸
/// 그대로 "재생"하는 것뿐이라 request_engine_move_if_needed(자동 응수)는 이어서
/// 부르지 않는다 - 그 다음 수도 이미 로컬 트리에 기록되어 있을 수 있는데, 여기서 또
/// genmove를 부르면 그 위에 엉뚱한 새 가지가 생겨버린다.
pub async fn sync_forward_to_engine(state: &AppState, mv: MoveInfo) {
    let cmd = if mv.is_pass {
        format!("play {} pass", mv.color.gtp_letter())
    } else {
        let size = state.with_game(|game| game.size());
        let vertex = coords::to_vertex(mv.x, mv.y, size);
        format!("play {} {}", mv.color.gtp_letter(), vertex)
    };
    mirror_to_all_sessions(state, &cmd).await;
}

/// 세션 하나(주로 방금 새로 (재)연결된 세션)를 지금까지의 실제 대국 상태(보드
/// 크기 + 덤 + 수순 전부)로 맞춘다. GtpSession은 (재)연결마다 katago 프로세스를
/// 통째로 새로 띄우므로 내부적으로는 항상 텅 빈 보드에서 시작한다 - 대국이 이미
/// 진행된 뒤에 새로 연결하거나(예: 몇 수 두고 나서야 엔진을 흑/백에 배정) SSH가
/// 조용히 끊겼다 자동으로 재연결되는 경우(gtp/process.rs::reconnect_loop) 둘 다 이
/// 재생 없이는 그 세션이 로컬 게임 트리와 완전히 다른(대개 텅 빈) 보드를 기준으로
/// genmove/kata-analyze를 하게 되어 추천수가 뜬금없어 보이는 원인이 된다.
/// best-effort: 개별 명령이 실패해도(연결이 막 끊긴 경우 등) 무시 - 그 세션의
/// 다음 실제 사용(genmove 등)이 알아서 실패로 드러난다.
pub async fn resync_session_to_history(state: &AppState, session: &GtpSession) {
    let (size, history) = state.with_game(|game| (game.size(), game.move_history()));
    let komi = state.komi();

    // boardsize 자체가 보드를 비우는 게 GTP 관례지만, 그걸 구현하지 않았거나 크기가
    // 그대로라 아무것도 안 하는 엔진도 있을 수 있으니 clear_board로 한 번 더 확실히 한다.
    let _ = session.send(&format!("boardsize {size}")).await;
    let _ = session.send("clear_board").await;
    let _ = session.send(&format!("komi {komi}")).await;
    // 트랜스포트별 추가 확장 명령(예: 로컬 온디바이스 엔진의 max_visits) - 이 함수
    // 자신은 그게 뭘 뜻하는지 몰라도 되게, transport::GtpTransport::
    // extra_resync_commands 뒤로 감춰져 있다.
    for cmd in session.extra_resync_commands() {
        let _ = session.send(&cmd).await;
    }

    for mv in history {
        let cmd = if mv.is_pass {
            format!("play {} pass", mv.color.gtp_letter())
        } else {
            let vertex = coords::to_vertex(mv.x, mv.y, size);
            format!("play {} {}", mv.color.gtp_letter(), vertex)
        };
        let _ = session.send(&cmd).await;
    }
}

/// 재연결 직후, 끊기기 전까지 이 세션에 kata-analyze가 켜져 있었다면(GtpSession::
/// analysis_interval) 지금 게임 트리 기준(끊긴 사이 다른 세션이 대신 두어 위치가
/// 바뀌었을 수도 있으므로 반드시 최신 노드/색으로 다시 계산)으로 즉시 다시 걸어준다.
/// 그러지 않으면 GameControls.svelte는 보드 위치가 바뀔 때만 kata-analyze를 다시
/// 시작하므로, 다음 착수가 생기기 전까지 화면엔 끊기기 직전의 낡은 분석 결과가 그대로
/// 남아있게 된다. 분석을 원한 적이 없었거나 이미 꺼둔 상태면(None) 아무것도 하지 않음.
pub async fn resume_analysis_if_wanted(state: &AppState, session: &GtpSession) {
    let Some(interval) = session.analysis_interval() else {
        return;
    };
    let snapshot = state.game_snapshot();
    session.set_analysis_context(snapshot.node_id, snapshot.current_turn);
    let _ = session
        .send(&format!("kata-analyze {interval} ownership true"))
        .await;
}

/// 다음 차례 색에 엔진이 배정되어 있고(state.engine_assignment) 그 프로필이 실제로
/// 연결되어 있는 동안 계속 그 세션에 `genmove`를 요청해 로컬 게임 트리에 반영한다 -
/// 흑/백에 서로 다른 서버가 배정돼 있으면 매 차례 그 색 담당 세션에 물어보고, 같은
/// 서버가 양쪽에 배정돼 있으면(세션은 하나뿐이므로) 자기 자신과 대국하듯 계속
/// 이어간다. 매 수 반영 직후 "board-updated" 이벤트를 emit해서, 이 반복이 오래
/// 걸려도(생각 시간 등) 프론트가 한 수씩 실시간으로 그릴 수 있게 한다.
pub async fn request_engine_move_if_needed(state: &AppState, app: &AppHandle) {
    loop {
        let (current_turn, size) =
            state.with_game(|game| (game.snapshot().current_turn, game.size()));

        let Some(session) = state.session_for_color(current_turn).await else {
            return;
        };

        let Ok(resp) = session
            .send(&format!("genmove {}", current_turn.gtp_letter()))
            .await
        else {
            return;
        };

        let Some(reply) = resp.trim().strip_prefix('=') else {
            return; // "?"로 시작하는 에러 응답 등 - 무시(로그/연결 상태로 원인 파악)
        };
        let reply = reply.trim();

        if reply.eq_ignore_ascii_case("resign") {
            // payload는 기권한 색(current_turn) - 프런트가 반대색 승리로("B+R"/"W+R")
            // 표시하려면 누가 기권했는지 알아야 한다.
            let _ = app.emit("engine-resigned", current_turn);
            return;
        }
        if reply.eq_ignore_ascii_case("pass") {
            // 로컬 게임 트리에도 pass를 그대로 반영 - 그래야 이어서 반대쪽 색도
            // 엔진에 배정되어 있는 경우(자기 자신과 대국) 그 차례로 넘어가 계속
            // 진행되고, 사람이 이어받을 때도 로컬/엔진 보드가 어긋나지 않는다.
            state.with_game(|game| game.pass_turn());
            let snapshot = state.game_snapshot();
            let _ = app.emit("board-updated", snapshot);
            let _ = app.emit("engine-passed", ());
            continue; // 다음 차례도 엔진 담당이면(둘 다 자동) 계속 이어감
        }

        let Some((ex, ey)) = coords::from_vertex(reply, size) else {
            return;
        };
        let moved = state.with_game(|game| game.confirm_move(ex, ey));
        if !moved {
            // 엔진이 로컬 트리 기준으로 이미 돌이 있는 칸을 알려줌 - 보드 상태가
            // 어긋난 것이므로 더 진행하지 않고 멈춘다.
            return;
        }

        let snapshot = state.game_snapshot();
        let _ = app.emit("board-updated", snapshot);
    }
}
