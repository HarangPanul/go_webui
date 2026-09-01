<script lang="ts">
  // additional: 실시간 집 영역 시각화. 바둑판 상태 변경 시 즉시 갱신
  // TODO: 현재 board 상태로부터 영역 추정 후 흑/백 영역 색상 오버레이
</script>

<canvas class="board-layer territory-overlay"></canvas>

<style>
  .board-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
</style>
