<script lang="ts">
  // kata-analyze 스트리밍 결과(analysisStore)를 바둑판 위에 "bluespot" 스타일로
  // 오버레이: 후보 수 지점마다 원 + 승률/집차이/visits 텍스트를 그린다. 원 색은
  // 현재 화면에 표시 중인 후보들 중 승률이 가장 높은 것을 파란색, 가장 낮은 것을
  // 빨간색으로 놓고 그 사이를 보간함(고정된 0~1 절대 스케일이 아니라 매번 그려지는
  // spots 집합 기준 상대 스케일 - 이러면 후보들 승률이 다 비슷하게 몰려 있어도 색
  // 차이가 뚜렷하게 드러남).
  //
  // RGB 값을 그대로 선형 보간하면(예전 방식) 두 끝 색(파랑/주황)이 서로 거의
  // 보색이라 중간 지점에서 채도가 빠진 탁한 회색조로 뭉개져버려, 승률이 애매한
  // 후보가 오히려 가장 나쁜 후보(원색 그대로인 빨강)보다 눈에 덜 띄는 역효과가
  // 있었다. 그래서 RGB가 아니라 HSL의 색상(H)만 파랑<->빨강 사이로 보간하고
  // 채도(S)/명도(L)는 항상 고정해서, 중간 지점(주황/노랑/초록/청록)도 항상 원색
  // 수준으로 선명하게 유지되게 함.
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

  // t=0(그 순간 표시된 후보 중 승률 최저) -> 빨강(0°), t=1(승률 최고) -> 파랑(210°).
  // 채도/명도는 고정한 채 색상(hue)만 이 사이를 그대로 지나가므로(빨강->주황->
  // 노랑->초록->청록->파랑), 중간 지점도 항상 원색만큼 선명함 - canvas는 CSS
  // hsl() 문자열을 fillStyle로 바로 받아들이므로 RGB로 변환할 필요도 없음.
  const LOW_HUE = 0;
  const HIGH_HUE = 210;
  const SATURATION = 85;
  const LIGHTNESS = 50;

  // 사람 눈은 초록/노랑 파장에 특히 민감해서, 채도·명도가 같아도 초록 쪽 hue가
  // 파랑/빨강보다 훨씬 밝게 느껴짐(상대 밝기 계산 시 초록 채널의 가중치가 가장
  // 큼 - ITU-R BT.709 luma 기준 R:G:B = 0.21:0.72:0.07). 그래서 초록(120°)
  // 근처일 때만 명도를 살짝 낮춰 체감 밝기를 그라데이션의 다른 구간과 비슷하게
  // 맞추고, 양 끝(빨강 0°/파랑 210°)은 원래 명도 그대로 둔다.
  const GREEN_HUE = 120;
  const GREEN_DARKEN_RADIUS = 90; // 이 범위(120°±90°) 밖은 보정 없음
  const GREEN_DARKEN_MAX = 14; // 초록 정중앙에서 명도를 낮추는 최대 폭(퍼센트 포인트)

  function lightnessFor(hue: number): number {
    const distance = Math.abs(hue - GREEN_HUE);
    const falloff = Math.max(0, 1 - distance / GREEN_DARKEN_RADIUS);
    return LIGHTNESS - GREEN_DARKEN_MAX * falloff;
  }

  function spotColor(t: number): string {
    const clamped = Math.min(1, Math.max(0, t));
    const hue = LOW_HUE + (HIGH_HUE - LOW_HUE) * clamped;
    return `hsl(${hue}, ${SATURATION}%, ${lightnessFor(hue)}%)`;
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

    // analysisStore.current는 (OwnershipOverlay와 달리) 조상 노드로 fallback하지
    // 않고 정확히 지금 노드의 결과만 보여준다 - 후보 수는 위치마다 완전히 달라질 수
    // 있어서, 다른 노드의 후보를 그대로 보여주면 "지금 이 칸에 후보가 있다"는 잘못된
    // 인상을 주기 때문(analysis.svelte.ts 상단 설명 참고). 그래도 안전장치로, 혹시
    // 이미 돌이 놓인 칸을 후보로 받으면(이론상 kata-analyze는 항상 그 시점의 빈 칸만
    // 후보로 주지만) 미리 걸러낸다. "pass"처럼 좌표가 없는 후보는 이 필터와 무관하므로
    // 그대로 통과시킴(아래 렌더 루프에서 fromVertex가 다시 걸러줌).
    const spots = [...result.candidates]
      .filter((c) => {
        const vertex = fromVertex(c.move, boardSize);
        return !vertex || boardStore.isEmpty(vertex.x, vertex.y);
      })
      .sort((a, b) => b.visits - a.visits)
      .slice(0, MAX_SPOTS);

    // 표시되는 후보들 기준 상대 스케일 - 전부 같은 승률이면(후보가 하나뿐이거나
    // 완전히 동률) 나눗셈이 0/0이 되지 않도록 그런 경우엔 전부 최고색(t=1)으로 고정.
    // winrate는 이론상 항상 값이 있지만(NaN/Infinity를 JSON이 표현 못 해 Rust f64가
    // 타입상 number | null로 내려옴), 실제로 null이 오는 경우는 없으므로 0으로
    // 안전하게 처리.
    const winrates = spots.map((c) => c.winrate ?? 0);
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
        winrateRange === 0
          ? 1
          : ((candidate.winrate ?? 0) - minWinrate) / winrateRange;

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
      ctx.fillText(
        `${((candidate.winrate ?? 0) * 100).toFixed(0)}%`,
        cx,
        cy - step * 0.18,
      );

      ctx.font = `${Math.round(step * 0.17)}px sans-serif`;
      ctx.fillText(formatScoreLead(candidate.scoreLead ?? 0), cx, cy + step * 0.05);

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
