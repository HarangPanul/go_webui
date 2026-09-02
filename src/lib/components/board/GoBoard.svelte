<script lang="ts">
  // 19x19 바둑판 컨테이너: 레이어 순서 = 배경 -> 실제 돌 -> 집 영역 -> ownership
  // 흑백 오버레이 -> AI 추천 수(bluespot). Ownership이 Analysis보다 아래에 있어야
  // 두 오버레이를 동시에 켜도 bluespot 원/텍스트가 ownership 음영 위에서 계속
  // 잘 보인다.
  // 부모(.board-area, container-type: size)의 가로/세로 중 짧은 쪽에 맞춰 정사각형 유지
  import BoardCanvas from "./BoardCanvas.svelte";
  import AnalysisOverlay from "./AnalysisOverlay.svelte";
  import OwnershipOverlay from "./OwnershipOverlay.svelte";
  import TerritoryOverlay from "./TerritoryOverlay.svelte";
</script>

<div class="go-board">
  <BoardCanvas />
  <TerritoryOverlay />
  <OwnershipOverlay />
  <AnalysisOverlay />
</div>

<style>
  .go-board {
    position: relative;
    width: 100cqmin;
    height: 100cqmin;
  }
</style>
