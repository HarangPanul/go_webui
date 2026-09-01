// 게임 트리(착수/따내기/뒤로 가기/마지막 수 제거/색 전환) 커맨드.
// 규칙 판정은 전부 crate::game::GameTree가 담당하고, 여기서는 AppState의 GameTree를
// 잠그고 호출한 뒤 최신 스냅샷을 반환하기만 함.
use tauri::State;

use crate::game::BoardSnapshot;
use crate::state::AppState;

#[tauri::command]
pub fn get_board_state(state: State<AppState>) -> BoardSnapshot {
    state.game.lock().unwrap().snapshot()
}

#[tauri::command]
pub fn confirm_move(x: usize, y: usize, state: State<AppState>) -> BoardSnapshot {
    let mut game = state.game.lock().unwrap();
    game.confirm_move(x, y);
    game.snapshot()
}

#[tauri::command]
pub fn go_back(state: State<AppState>) -> BoardSnapshot {
    let mut game = state.game.lock().unwrap();
    game.go_back();
    game.snapshot()
}

#[tauri::command]
pub fn remove_last_move(state: State<AppState>) -> BoardSnapshot {
    let mut game = state.game.lock().unwrap();
    game.remove_last_move();
    game.snapshot()
}

#[tauri::command]
pub fn toggle_turn(state: State<AppState>) -> BoardSnapshot {
    let mut game = state.game.lock().unwrap();
    game.toggle_turn();
    game.snapshot()
}
