// 게임 트리(착수/따내기/뒤로 가기/마지막 수 제거/색 전환) 커맨드.
// 실제 로직(락/스냅샷/이벤트 emit/엔진 동기화 순서)은 전부
// crate::services::game_service::GameService가 담당한다 - 여기서는 State/AppHandle을
// 받아 커맨드 호출마다 GameService를 만들어 위임하기만 한다.
use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::game::BoardSnapshot;
use crate::services::game_service::GameService;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn get_board_state(state: State<AppState>) -> BoardSnapshot {
    state.game_snapshot()
}

#[tauri::command]
#[specta::specta]
pub async fn confirm_move(
    x: usize,
    y: usize,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BoardSnapshot, AppError> {
    GameService::new(&state, &app).confirm_move(x, y).await
}

#[tauri::command]
#[specta::specta]
pub async fn pass_move(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BoardSnapshot, AppError> {
    GameService::new(&state, &app).pass().await
}

#[tauri::command]
#[specta::specta]
pub async fn go_back(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BoardSnapshot, AppError> {
    GameService::new(&state, &app).go_back().await
}

#[tauri::command]
#[specta::specta]
pub async fn go_forward(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BoardSnapshot, AppError> {
    GameService::new(&state, &app).go_forward().await
}

#[tauri::command]
#[specta::specta]
pub async fn remove_last_move(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BoardSnapshot, AppError> {
    GameService::new(&state, &app).remove_last_move().await
}

#[tauri::command]
#[specta::specta]
pub fn toggle_turn(state: State<AppState>, app: AppHandle) -> BoardSnapshot {
    GameService::new(&state, &app).toggle_turn()
}
