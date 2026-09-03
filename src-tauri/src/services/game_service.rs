// 게임 액션(착수/pass/뒤로 가기/앞으로 가기/마지막 수 제거/색 전환)에 공통되는
// "로컬 게임 트리 변경 -> 최신 스냅샷 반환 -> (필요하면) board-updated emit -> 엔진
// 동기화" 흐름을 한 곳에 모은 서비스. commands/game.rs의 각 커맨드는 이 타입을 호출마다
// 새로 만들어(state/app 참조만 들고 있는 얇은 래퍼) 대응되는 메서드 하나만 호출한다.
//
// 중요: confirm_move/pass는 사람이 방금 둔 수를 먼저 "board-updated" 이벤트로 즉시
// emit한 뒤에야 엔진 미러링/자동 응수를 시작한다 - 그렇지 않으면(엔진 응답을 기다리는
// 수백ms~수초 동안) 사람이 둔 돌이 화면에 바로 안 보이다가 엔진 응수와 한꺼번에
// 나타나는 문제가 생긴다. go_back/go_forward/remove_last_move는 (기존과 동일하게)
// board-updated를 직접 emit하지 않는다 - 프론트가 이 커맨드들의 반환값(BoardSnapshot)을
// 그대로 쓰기 때문.
use tauri::{AppHandle, Emitter};

use crate::error::AppError;
use crate::game::BoardSnapshot;
use crate::services::engine_sync;
use crate::state::AppState;

pub struct GameService<'a> {
    state: &'a AppState,
    app: &'a AppHandle,
}

impl<'a> GameService<'a> {
    pub fn new(state: &'a AppState, app: &'a AppHandle) -> Self {
        GameService { state, app }
    }

    pub async fn confirm_move(&self, x: usize, y: usize) -> Result<BoardSnapshot, AppError> {
        let color = self.state.game_snapshot().current_turn;
        let moved = self.state.with_game(|game| game.confirm_move(x, y));

        if moved {
            let snapshot = self.state.game_snapshot();
            let _ = self.app.emit("board-updated", snapshot);

            engine_sync::sync_move_to_engine(self.state, self.app, color, x, y).await;
        }

        Ok(self.state.game_snapshot())
    }

    // pass_move와 마찬가지로 사람의 pass를 먼저 "board-updated"로 즉시 반영한 뒤에
    // 엔진 미러링/자동 응수를 시작한다 - 이유도 동일(엔진 응답을 기다리는 동안 화면이
    // 멈춰 보이지 않도록).
    pub async fn pass(&self) -> Result<BoardSnapshot, AppError> {
        let color = self.state.game_snapshot().current_turn;
        self.state.with_game(|game| game.pass_turn());

        let snapshot = self.state.game_snapshot();
        let _ = self.app.emit("board-updated", snapshot);

        engine_sync::sync_pass_to_engine(self.state, self.app, color).await;

        Ok(self.state.game_snapshot())
    }

    // go_back/remove_last_move 둘 다 "로컬 트리를 한 수 되돌리기"라 엔진에도 `undo`를
    // 미러링해야 한다(그렇지 않으면 로컬과 엔진 보드가 어긋나 kata-analyze 결과의 기준
    // 색이 뒤집혀 보이는 문제가 생김 - engine_sync::sync_undo_to_engine 참고). 이미
    // 루트라서 실제로 되돌릴 수가 없었으면(can_go_back == false) undo를 보내지 않는다.
    pub async fn go_back(&self) -> Result<BoardSnapshot, AppError> {
        let could_go_back = self.state.with_game(|game| {
            let could_go_back = game.snapshot().can_go_back;
            game.go_back();
            could_go_back
        });
        if could_go_back {
            engine_sync::sync_undo_to_engine(self.state).await;
        }
        Ok(self.state.game_snapshot())
    }

    // go_back의 반대 방향: 게임 트리에서 자식 노드로 이동(앞으로 가기). 이동한 수가
    // 있으면(자식이 하나라도 있었으면) 그 수를 엔진에도 `play`로 재생해 로컬/엔진 보드를
    // 맞춘다(go_back의 undo 미러링과 대칭 - engine_sync::sync_forward_to_engine 참고).
    // 이미 리프 노드라 이동할 자식이 없었으면 엔진에는 아무것도 보내지 않는다.
    pub async fn go_forward(&self) -> Result<BoardSnapshot, AppError> {
        let mv = self.state.with_game(|game| game.go_forward());
        if let Some(mv) = mv {
            engine_sync::sync_forward_to_engine(self.state, mv).await;
        }
        Ok(self.state.game_snapshot())
    }

    pub async fn remove_last_move(&self) -> Result<BoardSnapshot, AppError> {
        let could_go_back = self.state.with_game(|game| {
            let could_go_back = game.snapshot().can_go_back;
            game.remove_last_move();
            could_go_back
        });
        if could_go_back {
            engine_sync::sync_undo_to_engine(self.state).await;
        }
        Ok(self.state.game_snapshot())
    }

    pub fn toggle_turn(&self) -> BoardSnapshot {
        self.state.with_game(|game| {
            game.toggle_turn();
            game.snapshot()
        })
    }
}
