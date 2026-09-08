// 게임 트리 상태 store: 착수/따내기/게임 트리 판정은 전부 Rust(src-tauri/src/game)에서
// 처리하고, 여기서는 그 결과 스냅샷을 담아두는 것만 담당하는 얇은 클라이언트. 이
// 파일은 스냅샷 자체와 그걸 바꾸는 6개 커맨드 래퍼(confirmMove/passMove/goBack/
// goForward/removeLastMove/toggleTurn)만 담당하고, 임시 선택 UI 상태는
// pendingMove.svelte.ts, 엔진 자동 착수 설정은 engineAssignment.svelte.ts가 따로 맡는다.
//
// confirmMove/passMove/goBack/goForward/removeLastMove는 pendingMove(다른
// store - pendingMove.svelte.ts)를 읽고 끝나면 비워야 해서 그쪽을 import한다 -
// pendingMove.svelte.ts도 pendingOnStone 계산에 이 store의 stones/isEmpty를
// 읽으려고 반대 방향으로 이 파일을 import하는데, 두 store 모두 최상위에서
// 서로를 즉시 호출하지 않고(각자 $state 초기화만 하고 실제 참조는 나중에 호출되는
// 함수/getter 안에서만 일어남) 순환 import 자체는 안전하다.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { pendingMoveStore } from "./pendingMove.svelte";
import { gameResultStore } from "./gameResult.svelte";

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
  // 현재 노드로 이어진 수가 pass였는지. true면 lastMove는 항상 null.
  lastMoveIsPass: boolean;
  currentChildren: MoveInfo[];
  canGoBack: boolean;
  // 현재 노드에 자식이 하나 이상 있는지("]"/앞으로 가기 버튼·단축키 활성화 여부).
  canGoForward: boolean;
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
    canGoForward: false,
    nodeId: 0,
    ancestorChain: [0],
    captures: { black: 0, white: 0 },
  };
}

function createGameTreeStore() {
  let snapshot = $state<BoardSnapshot>(emptySnapshot());

  // 앱 시작 시 Rust 쪽 게임 트리의 현재 상태를 한 번 가져와 동기화
  invoke<BoardSnapshot>("get_board_state")
    .then((s) => (snapshot = s))
    .catch((e) => console.error("get_board_state 초기 동기화 실패:", e));

  // 흑/백이 둘 다 켜져 있으면 백엔드가 사람 턴이 될 때까지(또는 pass/resign) 여러 수를
  // 연달아 자동으로 두므로, confirmMove() 호출 하나의 반환값만 기다리면 그 사이 수들이
  // 화면에 한 수씩 나타나지 않고 다 끝난 뒤에야 한 번에 반영된다. 그래서 백엔드가 매 수
  // 반영 직후 emit하는 이벤트를 구독해 진행 중에도 실시간으로 갱신한다.
  listen<BoardSnapshot>("board-updated", (event) => {
    setSnapshot(event.payload);
  });

  // 보드 상태가 실제로 바뀌는(=수순이 이동하는) 모든 지점에서 공통으로 씀 - 기권
  // 결과(gameResultStore)는 그 기권이 일어난 바로 그 지점에서만 유효하므로, 사람이
  // 이어서 두거나 뒤로/앞으로 이동하는 등 보드가 조금이라도 움직이면 항상 함께 지운다.
  function setSnapshot(next: BoardSnapshot) {
    snapshot = next;
    gameResultStore.clear();
  }

  return {
    get size() {
      return snapshot.size;
    },
    get stones() {
      return snapshot.stones;
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
    // 현재 노드에 자식이 하나 이상 있으면(=한 번이라도 그 지점에서 착수/pass한 적이
    // 있으면) 앞으로 갈 수 있음
    get canGoForward() {
      return snapshot.canGoForward;
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
    isEmpty(x: number, y: number) {
      return snapshot.stones[y]?.[x] === null;
    },
    // 돌을 실제로 두지 않고도 다음에 둘 색을 수동으로 바꿈 (예: 상대 대신 두는 경우 등)
    async toggleTurn() {
      setSnapshot(await invoke<BoardSnapshot>("toggle_turn"));
    },
    // 빈 칸을 확정하면 착수. 실제 따내기/게임 트리 갱신은 Rust에서 판정하고, 여기서는
    // 그 결과 스냅샷을 반영하기만 함.
    async confirmMove() {
      const pending = pendingMoveStore.pendingMove;
      if (!pending) return;
      // 착수 예정 십자선은 사람이 확정한 그 순간 끝난 UI 상태이므로, 백엔드 응답을
      // 기다리지 않고 요청 직후 바로 지운다. 엔진 자동 착수가 켜져 있으면
      // confirm_move는 사람 차례가 돌아올 때까지(엔진 genmove 포함) 응답하지 않는데,
      // await 뒤에서 지우면 그동안 십자선이 계속 남아있는 것처럼 보였다(로컬 엔진처럼
      // genmove가 느릴 때 특히 눈에 띔).
      pendingMoveStore.cancelPending();
      setSnapshot(await invoke<BoardSnapshot>("confirm_move", pending));
    },
    // 착수 없이 차례만 넘김. confirmMove와 마찬가지로 진행 중이던 임시 선택은 비움
    // (pass 버튼을 누르는 시점엔 어차피 확정하려던 게 아니므로), 역시 응답을 기다리지 않음.
    async passMove() {
      pendingMoveStore.cancelPending();
      setSnapshot(await invoke<BoardSnapshot>("pass_move"));
    },
    // 게임 트리에서 부모 노드로 이동(뒤로 가기)
    async goBack() {
      pendingMoveStore.cancelPending();
      setSnapshot(await invoke<BoardSnapshot>("go_back"));
    },
    // 게임 트리에서 자식 노드로 이동(앞으로 가기). 여러 갈래가 있으면 가장 마지막으로
    // 방문했던 자식으로(한 번도 안 가봤으면 가장 최근에 만들어진 자식으로) 이동함 -
    // 실제 판단은 백엔드(game::GameTree::go_forward)가 함.
    async goForward() {
      pendingMoveStore.cancelPending();
      setSnapshot(await invoke<BoardSnapshot>("go_forward"));
    },
    // 현재 노드(=가장 마지막으로 둔 수)를 게임 트리에서 통째로 삭제하고 그 부모로 이동
    async removeLastMove() {
      pendingMoveStore.cancelPending();
      setSnapshot(await invoke<BoardSnapshot>("remove_last_move"));
    },
  };
}

export const gameTreeStore = createGameTreeStore();
