// 로컬 게임 트리 <-> GTP 엔진 자동 동기화: 사람 착수를 엔진에 미러링하고, 엔진에
// 배정된 색(state.engine_colors) 차례가 되면 자동으로 genmove를 요청해 로컬 게임
// 트리에도 반영한다.
//
// 두 곳에서 호출된다:
// - commands::game::confirm_move: 사람이 막 착수했을 때, 그 수를 미러링한 뒤 이어서.
// - commands::gtp::set_engine_color: 흑/백 자동 착수 토글을 새로 켰을 때 - 이 경우
//   사람의 착수가 전혀 없을 수도 있으므로(예: 둘 다 막 켠 직후) 별도의 시작점이
//   필요하다.
use tauri::{AppHandle, Emitter};

use crate::game::Color;
use crate::gtp::coords;
use crate::state::AppState;

/// 사람의 착수를 연결된 GTP 엔진에 `play`로 반영(미러링)하고, 이어서
/// [`request_engine_move_if_needed`]로 자동 응수를 이어간다. 엔진과의 통신은 전부
/// best-effort - 연결이 없거나 실패해도 로컬 대국 자체는 계속돼야 하므로 에러는
/// 무시한다(연결 상태는 이미 GtpSession이 별도로 connection-status 이벤트로 알림).
pub async fn sync_move_to_engine(
    state: &AppState,
    app: &AppHandle,
    color: Color,
    x: usize,
    y: usize,
) {
    let Some(session) = ({
        let guard = state.gtp_session.lock().await;
        guard.clone()
    }) else {
        return;
    };

    let size = state.game.lock().unwrap().size();
    let vertex = coords::to_vertex(x, y, size);
    let cmd = format!("play {} {}", color.gtp_letter(), vertex);

    let Ok(resp) = session.send(&cmd).await else {
        return;
    };
    if resp.trim_start().starts_with('?') {
        // 엔진이 이 수를 거부함(예: 보드 상태가 어긋남) - 그대로 진행하면 엔진 응수가
        // 엉뚱한 상태 기준으로 나올 수 있으므로 여기서 중단.
        return;
    }

    request_engine_move_if_needed(state, app).await;
}

/// 사람의 pass를 연결된 GTP 엔진에 `play <color> pass`로 반영(미러링)하고, 이어서
/// [`request_engine_move_if_needed`]로 자동 응수를 이어간다. sync_move_to_engine과
/// 마찬가지로 best-effort.
pub async fn sync_pass_to_engine(state: &AppState, app: &AppHandle, color: Color) {
    let Some(session) = ({
        let guard = state.gtp_session.lock().await;
        guard.clone()
    }) else {
        return;
    };

    let cmd = format!("play {} pass", color.gtp_letter());
    let Ok(resp) = session.send(&cmd).await else {
        return;
    };
    if resp.trim_start().starts_with('?') {
        return;
    }

    request_engine_move_if_needed(state, app).await;
}

/// go_back/remove_last_move로 로컬 게임 트리를 한 수 되돌릴 때 연결된 GTP 엔진에도
/// `undo`를 보내 같은 수만큼 되돌린다. 이 미러링이 빠지면 로컬 트리와 엔진 내부 보드가
/// 어긋나서(엔진은 되돌리기 전 수를 그대로 들고 있음) 그 뒤 시작하는 kata-analyze가
/// 실제로는 로컬이 생각하는 위치보다 한 수 앞선 위치를 분석하게 되고, 그 결과의 기준
/// 색(엔진 입장의 pla)이 로컬에서 기대하는 색과 정확히 반대가 되어 winrate bar/
/// ownership이 뒤집혀 보이는 문제가 생긴다. sync_move_to_engine과 마찬가지로
/// best-effort - 연결이 없거나 실패해도 로컬 대국은 계속돼야 하므로 에러는 무시한다.
pub async fn sync_undo_to_engine(state: &AppState) {
    let Some(session) = ({
        let guard = state.gtp_session.lock().await;
        guard.clone()
    }) else {
        return;
    };
    let _ = session.send("undo").await;
}

/// 다음 차례가 엔진에 배정된 색(state.engine_colors)인 동안 계속 `genmove`로 응수를
/// 요청해 로컬 게임 트리에 반영한다 - 흑/백이 둘 다 켜져 있으면 사람 턴이 될 때까지
/// (또는 pass/resign/에러가 날 때까지) 자기 자신과 대국하듯 반복한다. 매 수 반영
/// 직후 "board-updated" 이벤트를 emit해서, 이 반복이 오래 걸려도(생각 시간 등)
/// 프론트가 한 수씩 실시간으로 그릴 수 있게 한다.
pub async fn request_engine_move_if_needed(state: &AppState, app: &AppHandle) {
    loop {
        let (current_turn, size) = {
            let game = state.game.lock().unwrap();
            (game.snapshot().current_turn, game.size())
        };
        let should_play = state.engine_colors.lock().unwrap().enabled(current_turn);
        if !should_play {
            return;
        }

        let Some(session) = ({
            let guard = state.gtp_session.lock().await;
            guard.clone()
        }) else {
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
            let _ = app.emit("engine-resigned", ());
            return;
        }
        if reply.eq_ignore_ascii_case("pass") {
            // 로컬 게임 트리에도 pass를 그대로 반영 - 그래야 이어서 반대쪽 색도
            // 엔진에 배정되어 있는 경우(자기 자신과 대국) 그 차례로 넘어가 계속
            // 진행되고, 사람이 이어받을 때도 로컬/엔진 보드가 어긋나지 않는다.
            state.game.lock().unwrap().pass_turn();
            let snapshot = state.game.lock().unwrap().snapshot();
            let _ = app.emit("board-updated", snapshot);
            let _ = app.emit("engine-passed", ());
            continue; // 다음 차례도 엔진 담당이면(둘 다 자동) 계속 이어감
        }

        let Some((ex, ey)) = coords::from_vertex(reply, size) else {
            return;
        };
        let moved = state.game.lock().unwrap().confirm_move(ex, ey);
        if !moved {
            // 엔진이 로컬 트리 기준으로 이미 돌이 있는 칸을 알려줌 - 보드 상태가
            // 어긋난 것이므로 더 진행하지 않고 멈춘다.
            return;
        }

        let snapshot = state.game.lock().unwrap().snapshot();
        let _ = app.emit("board-updated", snapshot);
    }
}
