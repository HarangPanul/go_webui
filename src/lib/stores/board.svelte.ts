// 바둑판 상태 store: 착수/따내기/게임 트리 판정은 전부 Rust(src-tauri/src/game)에서
// 처리하고, 여기서는 그 결과 스냅샷을 담아 두는 것과 임시 선택(pendingMove, 아직
// 서버에 확정 요청을 보내지 않은 UI 상태) 관리만 담당하는 얇은 클라이언트.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Stone = "black" | "white" | null;

// KataGo가 흑/백을 각각 자동으로 둘지 여부(색상별 독립 on/off). 둘 다 켜져 있으면
// KataGo가 자기 자신과 대국하듯 양쪽을 계속 두고, 둘 다 꺼져 있으면 자동 착수 없이
// 사람이 양쪽을 다 둠 - Rust state::EngineColors와 필드 대응(serde camelCase).
export interface EngineColors {
  black: boolean;
  white: boolean;
}

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
  // 현재 노드로 이어진 수가 pass였는지. true면 lastMove는 항상 null.
  lastMoveIsPass: boolean;
  currentChildren: MoveInfo[];
  canGoBack: boolean;
  // 게임 트리 arena 안에서 이 노드의 고유 인덱스. 절대 재사용되지 않으므로
  // analysisStore가 노드별 kata-analyze 결과를 캐싱하는 키로 사용.
  nodeId: number;
  // 현재 노드부터 루트까지 이어지는 조상 체인(자기 자신부터 시작). WinrateGraph가
  // "이 노드 자체는 아직 분석된 적 없어도 가장 가까운 조상의 결과"를 대신 표시할 때 사용.
  ancestorChain: number[];
  // 현재 노드까지 누적으로 흑/백이 각각 잡은 상대 돌 수(포로 수). CaptureCounter 표시용.
  captures: { black: number; white: number };
}

function emptySnapshot(): BoardSnapshot {
  const size = 19;
  return {
    size,
    stones: Array.from({ length: size }, () => Array(size).fill(null)),
    currentTurn: "black",
    lastMove: null,
    lastMoveIsPass: false,
    currentChildren: [],
    canGoBack: false,
    nodeId: 0,
    ancestorChain: [0],
    captures: { black: 0, white: 0 },
  };
}

function createBoardStore() {
  let snapshot = $state<BoardSnapshot>(emptySnapshot());
  let pendingMove = $state<{ x: number; y: number } | null>(null);
  let engineColors = $state<EngineColors>({ black: false, white: false });

  // 앱 시작 시 Rust 쪽 게임 트리 / 엔진 색 설정의 현재 상태를 한 번 가져와 동기화
  invoke<BoardSnapshot>("get_board_state").then((s) => (snapshot = s));
  invoke<EngineColors>("get_engine_colors").then((c) => (engineColors = c));

  // 흑/백이 둘 다 켜져 있으면 백엔드가 사람 턴이 될 때까지(또는 pass/resign) 여러 수를
  // 연달아 자동으로 두므로, confirmMove() 호출 하나의 반환값만 기다리면 그 사이 수들이
  // 화면에 한 수씩 나타나지 않고 다 끝난 뒤에야 한 번에 반영된다. 그래서 백엔드가 매 수
  // 반영 직후 emit하는 이벤트를 구독해 진행 중에도 실시간으로 갱신한다.
  listen<BoardSnapshot>("board-updated", (event) => {
    snapshot = event.payload;
  });

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
    // 마지막 착수 표시(반대색 동그라미)용: 현재 노드로 이어진 수. 루트거나 pass면 없음.
    get lastMove() {
      return snapshot.lastMove;
    },
    // 현재 노드로 이어진 수가 pass였는지 - BoardCanvas가 "PASS" 안내 문구를 그릴 때 사용.
    get lastMoveIsPass() {
      return snapshot.lastMoveIsPass;
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
    // analysisStore가 노드별 kata-analyze 결과를 조회/캐싱하는 키로 사용.
    get nodeId() {
      return snapshot.nodeId;
    },
    // WinrateGraph가 "가장 가까운 분석된 조상"을 찾을 때 사용(자기 자신부터 루트 순).
    get ancestorChain() {
      return snapshot.ancestorChain;
    },
    // 현재 노드까지 누적 포로 수(흑이 잡은 백돌 / 백이 잡은 흑돌). 착수 한 번에 여러
    // 수가 자동으로 이어질 수 있는 경우(흑/백 둘 다 자동 착수 켜짐)에도 board-updated
    // 이벤트로 매 수마다 snapshot이 갱신되므로 그때그때 최신 값이 반영됨.
    get captures() {
      return snapshot.captures;
    },
    // KataGo가 흑/백을 각각 자동으로 둘지 여부 - confirmMove() 후 백엔드가 이 값을
    // 보고 필요하면 자동으로 genmove까지 반영해 돌려주므로, 프론트는 이 값을 버튼
    // on/off 표시에만 사용하면 됨.
    get engineColors() {
      return engineColors;
    },
    async setEngineColor(color: "black" | "white", enabled: boolean) {
      engineColors = { ...engineColors, [color]: enabled };
      await invoke("set_engine_color", { color, enabled });
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
    // 착수 없이 차례만 넘김. confirmMove와 마찬가지로 진행 중이던 임시 선택은 비움
    // (pass 버튼을 누르는 시점엔 어차피 확정하려던 게 아니므로).
    async passMove() {
      snapshot = await invoke<BoardSnapshot>("pass_move");
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
