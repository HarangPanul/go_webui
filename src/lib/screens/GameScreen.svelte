<script lang="ts">
  // Phase 1~3 메인 화면: 상단 바 + 바둑판 + 분석 오버레이 + 대국 컨트롤
  // 레이아웃: 세로(모바일 portrait) - 상단 바 / 바둑판 / 하단 버튼
  //          가로(모바일 landscape, desktop) - 상단 바 / 바둑판(좌) + 버튼 패널(우)
  import TopBar from "../components/common/TopBar.svelte";
  import GoBoard from "../components/board/GoBoard.svelte";
  import GameControls from "../components/game/GameControls.svelte";
  import WinrateGraph from "../components/game/WinrateGraph.svelte";

  let { onOpenSettings }: { onOpenSettings: () => void } = $props();
</script>

<div class="game-screen">
  <TopBar {onOpenSettings} />

  <div class="game-body">
    <div class="board-area">
      <GoBoard />
    </div>

    <div class="control-panel">
      <WinrateGraph />
      <GameControls />
    </div>
  </div>
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

  .board-area {
    flex: 1;
    min-width: 0;
    min-height: 0;
    container-type: size;
    display: flex;
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
  }

  /* 가로 방향(모바일 landscape) / 데스크탑: 버튼 패널을 우측 세로 컬럼으로 전환 */
  @media (orientation: landscape) {
    .game-body {
      flex-direction: row;
    }

    .control-panel {
      flex-direction: column;
      justify-content: center;
      width: 240px;
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
