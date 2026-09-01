// 바둑판 상태 store: 착수/따내기/게임 트리 판정은 전부 Rust(src-tauri/src/game)에서
// 처리하고, 여기서는 그 결과 스냅샷을 담아 두는 것과 임시 선택(pendingMove, 아직
// 서버에 확정 요청을 보내지 않은 UI 상태) 관리만 담당하는 얇은 클라이언트.

import { invoke } from "@tauri-apps/api/core";

export type Stone = "black" | "white" | null;

interface MoveInfo {
  x: number;
  y: number;
  color: "black" | "white";
}

// Rust game::BoardSnapshot과 필드 대응 (serde camelCase)
interface BoardSnapshot {
  size: number;
  stones: Stone[][];
  currentTurn: "black" | "white";
  lastMove: { x: number; y: number } | null;
  currentChildren: MoveInfo[];
  canGoBack: boolean;
}

function emptySnapshot(): BoardSnapshot {
  const size = 19;
  return {
    size,
    stones: Array.from({ length: size }, () => Array(size).fill(null)),
    currentTurn: "black",
    lastMove: null,
    currentChildren: [],
    canGoBack: false,
  };
}

function createBoardStore() {
  let snapshot = $state<BoardSnapshot>(emptySnapshot());
  let pendingMove = $state<{ x: number; y: number } | null>(null);

  // 앱 시작 시 Rust 쪽 게임 트리의 현재 상태를 한 번 가져와 동기화
  invoke<BoardSnapshot>("get_board_state").then((s) => (snapshot = s));

  return {
    get size() {
      return snapshot.size;
    },
    get stones() {
      return snapshot.stones;
    },
    get pendingMove() {
      return pendingMove;
    },
    get currentTurn() {
      return snapshot.currentTurn;
    },
    // 마지막 착수 표시(반대색 동그라미)용: 현재 노드로 이어진 수. 루트면 없음.
    get lastMove() {
      return snapshot.lastMove;
    },
    // 현재 노드에서 갈라지는 다음 수 후보들 (게임 트리의 자식 노드들). 보드 위에
    // 반투명하고 작은 동그라미로 표시하는 데 사용.
    get currentChildren() {
      return snapshot.currentChildren;
    },
    // 루트가 아니면(=이전 수가 있으면) 뒤로 갈 수 있음 / 마지막 수를 지울 수 있음
    get canGoBack() {
      return snapshot.canGoBack;
    },
    // pendingMove가 이미 돌이 놓인 칸을 가리키는지 여부. 그 칸에는 착수할 수 없으므로
    // UI에서 확정 버튼을 비활성화하는 데 사용.
    get pendingOnStone() {
      if (!pendingMove) return false;
      return snapshot.stones[pendingMove.y][pendingMove.x] !== null;
    },
    isEmpty(x: number, y: number) {
      return snapshot.stones[y]?.[x] === null;
    },
    selectPending(x: number, y: number) {
      pendingMove = { x, y };
    },
    cancelPending() {
      pendingMove = null;
    },
    // 돌을 실제로 두지 않고도 다음에 둘 색을 수동으로 바꿈 (예: 상대 대신 두는 경우 등)
    async toggleTurn() {
      snapshot = await invoke<BoardSnapshot>("toggle_turn");
    },
    // 빈 칸을 확정하면 착수. 실제 따내기/게임 트리 갱신은 Rust에서 판정하고, 여기서는
    // 그 결과 스냅샷을 반영하기만 함.
    async confirmMove() {
      if (!pendingMove) return;
      const { x, y } = pendingMove;
      snapshot = await invoke<BoardSnapshot>("confirm_move", { x, y });
      pendingMove = null;
    },
    // 게임 트리에서 부모 노드로 이동(뒤로 가기)
    async goBack() {
      snapshot = await invoke<BoardSnapshot>("go_back");
      pendingMove = null;
    },
    // 현재 노드(=가장 마지막으로 둔 수)를 게임 트리에서 통째로 삭제하고 그 부모로 이동
    async removeLastMove() {
      snapshot = await invoke<BoardSnapshot>("remove_last_move");
      pendingMove = null;
    },
  };
}

export const boardStore = createBoardStore();
