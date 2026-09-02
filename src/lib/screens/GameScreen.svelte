<script lang="ts">
  // Phase 1~3 메인 화면: 상단 바 + 바둑판 + 분석 오버레이 + 대국 컨트롤
  // 레이아웃: 세로(모바일 portrait) - 상단 바 / 바둑판 / 승률 막대 / 하단 버튼
  //          가로(모바일 landscape, desktop) - 상단 바 / [바둑판 + 승률 막대](좌) + 버튼 패널(우)
  // 승률 막대는 버튼 패널이 아니라 바둑판 바로 옆(board-area)에 붙어 있음 - 버튼과는
  // 별개로 바둑판에 딸린 요소이기 때문. GoBoard와 WinrateGraph를 board-area 안에
  // 나란히 넣어 두 요소를 한 덩어리로 같이 가운데 정렬시킴 - board-area를 각각 따로
  // 채우고 따로 가운데 정렬하면(예전 board-column 방식) 정사각형 바둑판과 컨테이너
  // 가장자리 사이에 남는 여백만큼 막대가 바둑판에서 떨어져 보이는 문제가 있었음.
  // CaptureCounter(포로 수 배지)는 board-area/control-panel 어느 쪽에도 속하지 않고
  // position: fixed로 화면 오른쪽 아래 구석에 항상 고정되므로 이 둘과 형제로 최상위에
  // 둔다(CaptureCounter.svelte 참고).
  import TopBar from "../components/common/TopBar.svelte";
  import GoBoard from "../components/board/GoBoard.svelte";
  import GameControls from "../components/game/GameControls.svelte";
  import WinrateGraph from "../components/game/WinrateGraph.svelte";
  import CaptureCounter from "../components/game/CaptureCounter.svelte";
  import { boardStore } from "../stores/board.svelte";

  let { onOpenSettings }: { onOpenSettings: () => void } = $props();

  // 바둑판 바깥의 빈 여백(패딩/가운데 정렬로 남는 공간)을 클릭/터치하면 임시 선택을
  // 취소한다. GoBoard/WinrateGraph 등 실제 자식 요소를 클릭했을 때는 target이
  // board-area 자신이 아니므로(각자 자기 클릭을 먼저 처리) 여기서는 무시된다.
  function handleBoardAreaClick(evt: MouseEvent) {
    if (evt.target === evt.currentTarget) {
      boardStore.cancelPending();
    }
  }
</script>

<div class="game-screen">
  <TopBar {onOpenSettings} />

  <div class="game-body">
    <div class="board-area" onclick={handleBoardAreaClick}>
      <GoBoard />
      <WinrateGraph />
    </div>

    <div class="control-panel">
      <GameControls />
    </div>
  </div>

  <CaptureCounter />
</div>

<style>
  .game-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .game-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  /* GoBoard(정사각형) + WinrateGraph를 같이 담는 컨테이너. 이 안에서 GoBoard와
     WinrateGraph를 gap 없이 나란히 배치해 막대가 바둑판 가장자리에 그대로 맞닿게 하고,
     justify-content: center로 그 둘을 하나의 덩어리로 묶어 가운데 정렬한다(각각 따로
     정렬하면 정사각형 바둑판과 컨테이너 사이에 남는 여백만큼 막대가 떨어져 보임).
     세로 모드: 세로로 쌓여 막대가 바둑판 바로 아래 / 가로 모드: 가로로 나란히
     막대가 바둑판 오른쪽 - game-body와 같은 방향으로 전환되며, 버튼 패널
     (control-panel)과는 독립적으로 움직임. */
  .board-area {
    flex: 1;
    min-width: 0;
    min-height: 0;
    container-type: size;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 8px;
  }

  .control-panel {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    /* Android 제스처 내비게이션 바에 버튼이 가려지지 않도록 하단 안전 영역만큼
       여백 추가 - portrait에서는 이 패널 자체가 화면 맨 아래에 위치함 */
    padding-bottom: calc(8px + env(safe-area-inset-bottom, 0px));
  }

  /* 가로 방향(모바일 landscape) / 데스크탑: 버튼 패널을 우측 세로 컬럼으로,
     바둑판+승률 막대 묶음을 좌측에서 가로 배치(막대가 바둑판 오른쪽에 옴)로 전환 */
  @media (orientation: landscape) {
    .game-body {
      flex-direction: row;
    }

    .board-area {
      flex-direction: row;
    }

    .control-panel {
      flex-direction: column;
      justify-content: center;
      width: 240px;
      /* landscape에서는 패널이 화면 오른쪽 가장자리에 붙으므로(기기를 반대로
         돌리면 내비게이션 바가 오른쪽에 올 수 있음) 오른쪽 안전 영역도 챙긴다.
         하단 여백은 위에서 이미 처리된 값을 그대로 유지. */
      padding-right: calc(8px + env(safe-area-inset-right, 0px));
    }
  }

  /* 세로 방향(모바일 portrait): 버튼 패널을 하단 가로 바로 전환 */
  @media (orientation: portrait) {
    .control-panel {
      flex-direction: row;
      flex-wrap: wrap;
    }
  }
</style>
