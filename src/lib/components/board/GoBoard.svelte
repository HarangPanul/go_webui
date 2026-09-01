<script lang="ts">
  // 19x19 바둑판 컨테이너: 레이어 순서 = 배경 -> 실제 돌 -> AI 추천 수 -> 임시 하이라이트
  // 부모(.board-area, container-type: size)의 가로/세로 중 짧은 쪽에 맞춰 정사각형 유지
  import BoardCanvas from "./BoardCanvas.svelte";
  import AnalysisOverlay from "./AnalysisOverlay.svelte";
  import TerritoryOverlay from "./TerritoryOverlay.svelte";
</script>

<div class="go-board">
  <BoardCanvas />
  <TerritoryOverlay />
  <AnalysisOverlay />
</div>

<style>
  .go-board {
    position: relative;
    width: 100cqmin;
    height: 100cqmin;
  }
</style>
