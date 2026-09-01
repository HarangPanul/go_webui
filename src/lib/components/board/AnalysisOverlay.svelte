<script lang="ts">
  // kata-analyze 스트리밍 결과(PV, 승률, 후보수)를 바둑판 위에 오버레이
  // TODO: analysis store 구독 후 후보수 좌표에 승률/방문수 텍스트 렌더링
</script>

<canvas class="board-layer analysis-overlay"></canvas>

<style>
  .board-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
</style>
