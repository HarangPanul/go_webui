<script lang="ts">
  // kata-analyze 스트리밍 결과(analysisStore)를 바둑판 위에 "bluespot" 스타일로
  // 오버레이: 후보 수 지점마다 원 + 승률/집차이/visits 텍스트를 그린다. 원 색은
  // 현재 화면에 표시 중인 후보들 중 승률이 가장 높은 것을 파란색, 가장 낮은 것을
  // 주황색으로 놓고 그 사이를 선형 보간함(고정된 0~1 절대 스케일이 아니라
  // 매번 그려지는 spots 집합 기준 상대 스케일 - 이러면 후보들 승률이 다 비슷하게
  // 몰려 있어도 색 차이가 뚜렷하게 드러남).
  //
  // kata-analyze의 candidate.winrate/scoreLead는 이미 "그 수를 두는 사람(현재
  // 분석 요청 시점에 둘 차례였던 색)" 기준이라 후보별로 그대로 색/텍스트에 쓰면
  // 됨 - WinrateGraph.svelte가 흑 하나 기준 막대를 만들기 위해 forColor로 다시
  // 반전시켜야 했던 것과는 다른 용도라 반전이 필요 없음.
  //
  // analysisStore.showAnalysis 토글이 켜져 있을 때만 그림 - kata-analyze 스트림
  // 자체는(Ownership만 켜져 있어도) 계속 돌면서 결과를 캐싱해두므로, 이 토글을
  // 나중에 켜는 순간 이미 받아둔 최신 결과를 바로 보여줄 수 있다.
  import { boardStore } from "../../stores/board.svelte";
  import { analysisStore } from "../../stores/analysis.svelte";
  import { fromVertex } from "../../utils/coords";

  // 방문수 상위 N개만 표시 - 다 그리면 좁은 칸에 텍스트가 겹쳐 오히려 안 보임
  const MAX_SPOTS = 16;

  // t=0(그 순간 표시된 후보 중 승률 최저) -> 주황, t=1(승률 최고) -> 파랑
  // (matplotlib tab:orange/tab:blue 참고)
  const LOW_COLOR: readonly [number, number, number] = [255, 127, 14];
  const HIGH_COLOR: readonly [number, number, number] = [31, 119, 180];

  function spotColor(t: number): string {
    const clamped = Math.min(1, Math.max(0, t));
    const r = Math.round(LOW_COLOR[0] + (HIGH_COLOR[0] - LOW_COLOR[0]) * clamped);
    const g = Math.round(LOW_COLOR[1] + (HIGH_COLOR[1] - LOW_COLOR[1]) * clamped);
    const b = Math.round(LOW_COLOR[2] + (HIGH_COLOR[2] - LOW_COLOR[2]) * clamped);
    return `rgb(${r}, ${g}, ${b})`;
  }

  function formatScoreLead(scoreLead: number): string {
    return scoreLead >= 0 ? `+${scoreLead.toFixed(1)}` : scoreLead.toFixed(1);
  }

  function formatVisits(visits: number): string {
    return visits >= 1000 ? `${(visits / 1000).toFixed(1)}k` : String(visits);
  }

  let canvasEl: HTMLCanvasElement | undefined = $state();

  // BoardCanvas.svelte와 같은 격자 치수 계산(캔버스 backing store 좌표 기준) -
  // 후보 지점을 같은 격자 교차점 위에 정확히 겹쳐 그리려면 동일한 공식이어야 함.
  function metrics(canvasSize: number, boardSize: number) {
    const margin = canvasSize / (boardSize + 1);
    const step = (canvasSize - margin * 2) / (boardSize - 1);
    return { margin, step };
  }

  function draw() {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;

    const size = canvasEl.width;
    ctx.clearRect(0, 0, size, size);

    if (!analysisStore.showAnalysis) return;

    const result = analysisStore.current?.result;
    if (!result) return;

    const boardSize = boardStore.size;
    const { margin, step } = metrics(size, boardSize);
    const spotRadius = step * 0.475; // BoardCanvas의 돌 반지름과 같은 비율 - 칸을 꽉 채움

    const spots = [...result.candidates]
      .sort((a, b) => b.visits - a.visits)
      .slice(0, MAX_SPOTS);

    // 표시되는 후보들 기준 상대 스케일 - 전부 같은 승률이면(후보가 하나뿐이거나
    // 완전히 동률) 나눗셈이 0/0이 되지 않도록 그런 경우엔 전부 최고색(t=1)으로 고정.
    const winrates = spots.map((c) => c.winrate);
    const minWinrate = Math.min(...winrates);
    const maxWinrate = Math.max(...winrates);
    const winrateRange = maxWinrate - minWinrate;

    const pending = boardStore.pendingMove;

    for (const candidate of spots) {
      const vertex = fromVertex(candidate.move, boardSize);
      if (!vertex) continue; // "pass" 등 좌표가 없는 수는 건너뜀
      // 지금 착수 예정으로 선택된 칸이면 이 원/텍스트를 그리지 않는다 - 그려버리면
      // BoardCanvas가 같은 칸에 그리는 반투명 미리보기 돌/십자선을 완전히 덮어써서
      // "어디를 두려는지"가 안 보이게 됨. 추천 정보보다 지금 두려는 위치 확인이
      // 우선이므로 그 칸만 건너뛴다.
      if (pending && pending.x === vertex.x && pending.y === vertex.y) continue;
      const cx = margin + vertex.x * step;
      const cy = margin + vertex.y * step;

      const t =
        winrateRange === 0 ? 1 : (candidate.winrate - minWinrate) / winrateRange;

      ctx.save();
      ctx.globalAlpha = 0.85;
      ctx.beginPath();
      ctx.arc(cx, cy, spotRadius, 0, Math.PI * 2);
      ctx.fillStyle = spotColor(t);
      ctx.fill();
      ctx.restore();

      ctx.save();
      ctx.fillStyle = "#ffffff";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.shadowColor = "rgba(0, 0, 0, 0.7)";
      ctx.shadowBlur = step * 0.06;

      ctx.font = `bold ${Math.round(step * 0.24)}px sans-serif`;
      ctx.fillText(`${(candidate.winrate * 100).toFixed(0)}%`, cx, cy - step * 0.18);

      ctx.font = `${Math.round(step * 0.17)}px sans-serif`;
      ctx.fillText(formatScoreLead(candidate.scoreLead), cx, cy + step * 0.05);

      ctx.font = `${Math.round(step * 0.15)}px sans-serif`;
      ctx.fillText(formatVisits(candidate.visits), cx, cy + step * 0.27);
      ctx.restore();
    }
  }

  $effect(() => {
    if (!canvasEl) return;
    const parent = canvasEl.parentElement;
    if (!parent) return;

    const resize = () => {
      if (!canvasEl) return;
      const dpr = window.devicePixelRatio || 1;
      const cssSize = parent.clientWidth;
      canvasEl.width = Math.round(cssSize * dpr);
      canvasEl.height = Math.round(cssSize * dpr);
      draw();
    };

    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(parent);

    return () => observer.disconnect();
  });

  $effect(() => {
    // 보드 크기, 최신 분석 결과, 착수 예정 위치(pendingMove - 바뀔 때마다 가려야 할
    // 칸도 바뀜), 표시 토글 변경 시 다시 그림 (analysisStore.current는 kata-analyze
    // 이벤트마다 또는 노드 이동 시 새 값으로 바뀌므로 참조만 읽어도 반응함)
    boardStore.size;
    boardStore.pendingMove;
    analysisStore.current;
    analysisStore.showAnalysis;
    draw();
  });
</script>

<canvas bind:this={canvasEl} class="board-layer analysis-overlay"></canvas>

<style>
  .board-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
</style>
