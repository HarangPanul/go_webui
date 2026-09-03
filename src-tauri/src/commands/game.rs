// 게임 트리(착수/따내기/뒤로 가기/마지막 수 제거/색 전환) 커맨드.
// 규칙 판정은 전부 crate::game::GameTree가 담당하고, 여기서는 AppState의 GameTree를
// 잠그고 호출한 뒤 최신 스냅샷을 반환하기만 함.
//
// confirm_move는 추가로 "엔진과의 동기화"까지 담당한다(실제 로직은
// crate::services::engine_sync가 처리). 중요: 사람이 방금 둔 수를 먼저 "board-updated"
// 이벤트로 즉시 emit한 뒤에야 엔진 미러링/자동 응수를 시작한다 - 그렇지 않으면
// (엔진 응답을 기다리는 수백ms~수초 동안) 사람이 둔 돌이 화면에 바로 안 보이다가
// 엔진 응수와 한꺼번에 나타나는 문제가 생긴다.
use tauri::{AppHandle, Emitter, State};

use crate::error::AppError;
use crate::game::BoardSnapshot;
use crate::services::engine_sync;
use crate::state::AppState;

#[tauri::command]
pub fn get_board_state(state: State<AppState>) -> BoardSnapshot {
    state.game_snapshot()
}

#[tauri::command]
pub async fn confirm_move(
    x: usize,
    y: usize,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BoardSnapshot, AppError> {
    let color = state.game_snapshot().current_turn;
    let moved = state.with_game(|game| game.confirm_move(x, y));

    if moved {
        let snapshot = state.game_snapshot();
        let _ = app.emit("board-updated", snapshot);

        engine_sync::sync_move_to_engine(state.inner(), &app, color, x, y).await;
    }

    Ok(state.game_snapshot())
}

// confirm_move와 마찬가지로 사람의 pass를 먼저 "board-updated"로 즉시 반영한 뒤에
// 엔진 미러링/자동 응수를 시작한다 - 이유도 동일(엔진 응답을 기다리는 동안 화면이
// 멈춰 보이지 않도록).
#[tauri::command]
pub async fn pass_move(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BoardSnapshot, AppError> {
    let color = state.game_snapshot().current_turn;
    state.with_game(|game| game.pass_turn());

    let snapshot = state.game_snapshot();
    let _ = app.emit("board-updated", snapshot);

    engine_sync::sync_pass_to_engine(state.inner(), &app, color).await;

    Ok(state.game_snapshot())
}

// go_back/remove_last_move 둘 다 "로컬 트리를 한 수 되돌리기"라 엔진에도 `undo`를
// 미러링해야 한다(그렇지 않으면 로컬과 엔진 보드가 어긋나 kata-analyze 결과의 기준
// 색이 뒤집혀 보이는 문제가 생김 - engine_sync::sync_undo_to_engine 참고). 이미 루트라서
// 실제로 되돌릴 수가 없었으면(can_go_back == false) undo를 보내지 않는다.
#[tauri::command]
pub async fn go_back(state: State<'_, AppState>) -> Result<BoardSnapshot, AppError> {
    let could_go_back = state.with_game(|game| {
        let could_go_back = game.snapshot().can_go_back;
        game.go_back();
        could_go_back
    });
    if could_go_back {
        engine_sync::sync_undo_to_engine(state.inner()).await;
    }
    Ok(state.game_snapshot())
}

// go_back의 반대 방향: 게임 트리에서 자식 노드로 이동(앞으로 가기). 이동한 수가
// 있으면(자식이 하나라도 있었으면) 그 수를 엔진에도 `play`로 재생해 로컬/엔진 보드를
// 맞춘다(go_back의 undo 미러링과 대칭 - engine_sync::sync_forward_to_engine 참고).
// 이미 리프 노드라 이동할 자식이 없었으면 엔진에는 아무것도 보내지 않는다.
#[tauri::command]
pub async fn go_forward(state: State<'_, AppState>) -> Result<BoardSnapshot, AppError> {
    let mv = state.with_game(|game| game.go_forward());
    if let Some(mv) = mv {
        engine_sync::sync_forward_to_engine(state.inner(), mv).await;
    }
    Ok(state.game_snapshot())
}

#[tauri::command]
pub async fn remove_last_move(state: State<'_, AppState>) -> Result<BoardSnapshot, AppError> {
    let could_go_back = state.with_game(|game| {
        let could_go_back = game.snapshot().can_go_back;
        game.remove_last_move();
        could_go_back
    });
    if could_go_back {
        engine_sync::sync_undo_to_engine(state.inner()).await;
    }
    Ok(state.game_snapshot())
}

#[tauri::command]
pub fn toggle_turn(state: State<AppState>) -> BoardSnapshot {
    state.with_game(|game| {
        game.toggle_turn();
        game.snapshot()
    })
}
