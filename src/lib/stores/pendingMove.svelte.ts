// 임시 선택(아직 서버에 확정 요청을 보내지 않은 UI 상태) store. 실제 확정
// (invoke("confirm_move"))은 gameTree.svelte.ts가 담당하고(그쪽이 이 store를 읽고
// 끝나면 비움), 여기서는 순수 "지금 어느 칸을 임시로 가리키고 있는지"만 관리한다.
//
// pendingOnStone은 gameTree.svelte.ts의 stones를 읽어야 해서 그쪽을 import한다 -
// gameTree.svelte.ts도 confirmMove 등에서 반대 방향으로 이 파일을 import하는데,
// 두 store 모두 최상위에서 서로를 즉시 호출하지 않으므로(gameTree.svelte.ts 상단
// 설명 참고) 순환 import 자체는 안전하다.
import { gameTreeStore } from "./gameTree.svelte";

function createPendingMoveStore() {
  let pendingMove = $state<{ x: number; y: number } | null>(null);

  return {
    get pendingMove() {
      return pendingMove;
    },
    // pendingMove가 이미 돌이 놓인 칸을 가리키는지 여부. 그 칸에는 착수할 수 없으므로
    // UI에서 확정 버튼을 비활성화하는 데 사용.
    get pendingOnStone() {
      if (!pendingMove) return false;
      return !gameTreeStore.isEmpty(pendingMove.x, pendingMove.y);
    },
    selectPending(x: number, y: number) {
      pendingMove = { x, y };
    },
    cancelPending() {
      pendingMove = null;
    },
  };
}

export const pendingMoveStore = createPendingMoveStore();
