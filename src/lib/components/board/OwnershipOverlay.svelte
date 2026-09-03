<script lang="ts">
  // kata-analyze의 ownership 필드(GameControls가 "ownership true"를 항상 붙여
  // 요청함)를 바둑판 위에 흑/백 그레이스케일로 오버레이. analysisStore.showOwnership
  // 토글이 켜져 있을 때만 그림 - Analysis/Ownership 두 오버레이가 항상 같은 데이터를
  // 받고 있어도 표시 여부는 독립적으로 켜고 끌 수 있음.
  //
  // 지금 노드 자체는 아직 한 번도 분석된 적이 없을 수 있다(막 이동한 직후 아직
  // kata-analyze의 첫 결과가 안 왔거나, 애초에 분석 없이 지나온 위치) - 이 경우
  // WinrateGraph.svelte와 같은 방식으로 현재 노드부터 조상 방향으로(자기 자신 포함,
  // boardStore.ancestorChain이 가까운 순서로 줌) 캐싱된 ownership이 있는 가장 가까운
  // 노드의 값을 대신 보여준다 - 완전히 같은 위치는 아닐 수 있지만 지형이 크게 다르지
  // 않은 한두 수 차이라 계속 뭔가 보이는 편이 매 이동마다 오버레이가 깜빡이며 사라지는
  // 것보다 낫다. AnalysisOverlay(bluespot 후보 지점)는 반대로 이런 조상 fallback 없이
  // 노드마다 개별적으로 관리한다 - 후보 수 자체가 위치마다 완전히 달라질 수 있어서,
  // 다른 노드의 후보를 그대로 보여주면 "지금 이 위치에서 그 칸에 후보가 있다"는
  // 잘못된 인상을 줄 수 있기 때문(그레이스케일 영역 표시보다 훨씬 오해하기 쉬움).
  //
  // ownership 값은 kata-analyze의 winrate/scoreLead와 같은 기준(분석 요청 당시
  // 둘 차례였던 색, 위 forColor)으로 [-1, 1] - 그대로 쓰면 같은 흑 집이
  // 분석 시작 시점이 흑 차례였는지 백 차례였는지에 따라 색이 뒤바뀌어 보이므로,
  // WinrateGraph.svelte와 같은 방식으로 항상 "흑 기준" 값으로 반전시켜 둔 뒤에
  // 흑(검정)<->백(흰색) 그레이스케일로 매핑한다. -1..1의 절대 스케일 자체가 이미
  // "얼마나 확실히 그 색 소유인가"를 의미하므로(경합 지점은 0 근처), 승률 오버레이처럼
  // 화면에 보이는 값들 기준 상대 정규화를 할 필요는 없음.
  //
  // 부호는 KataGo 소스(cpp/command/gtp.cpp의 kata-analyze "ownership" 출력부,
  // cpp/neuralnet/nninputs.h의 NNOutput::whiteOwnerMap)를 직접 확인해서 정함:
  // - NN이 내는 원값(whiteOwnerMap)은 이름 그대로 항상 "백 기준"(양수 = 백 소유).
  // - gtp.cpp가 kata-analyze로 내보내기 직전에 `perspective == P_BLACK ||
  //   (perspective 미지정 && pla == P_BLACK)`일 때만 부호를 뒤집는다 - 이건
  //   winrate/scoreLead에 걸린 조건과 완전히 같다. [player] 인자 없이 보낸 우리
  //   커맨드에서는 이 조건이 "그 순간 둘 차례인 색이 흑이면 뒤집는다"와 같으므로,
  //   최종적으로 GTP가 내보내는 ownership은 항상 "그 수를 두는 사람(pla, 즉
  //   forColor)" 기준 - 양수면 forColor 소유, 음수면 상대 소유. 그래서 아래
  //   변환은 forColor==black일 때 raw를 그대로(양수=흑), white일 때 -raw로
  //   반전시켜(양수=흑) "흑 기준" 값을 얻는다 - 추가 반전 없이 이게 정답.
  import { boardStore } from "../../stores/board.svelte";
  import { analysisStore } from "../../stores/analysis.svelte";

  let canvasEl: HTMLCanvasElement | undefined = $state();

  // BoardCanvas.svelte와 같은 격자 치수 계산(캔버스 backing store 좌표 기준)
  function metrics(canvasSize: number, boardSize: number) {
    const margin = canvasSize / (boardSize + 1);
    const step = (canvasSize - margin * 2) / (boardSize - 1);
    return { margin, step };
  }

  // 현재 노드부터 조상 방향으로 캐싱된 ownership이 있는 가장 가까운 노드를 찾음
  // (WinrateGraph.svelte의 blackWinrate/blackScoreLead와 같은 패턴).
  function nearestOwnership(): { ownership: number[]; forColor: "black" | "white" } | null {
    for (const nodeId of boardStore.ancestorChain) {
      const cached = analysisStore.forNode(nodeId);
      if (cached?.result.ownership) {
        // 각 지점 값도 이론상 항상 있지만(winrate와 같은 이유로 number | null) 실제로
        // null이 오는 경우는 없으므로 0(경합 지점과 동일하게 취급)으로 안전하게 처리.
        const ownership = cached.result.ownership.map((v) => v ?? 0);
        return { ownership, forColor: cached.forColor };
      }
    }
    return null;
  }

  function draw() {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;

    const size = canvasEl.width;
    ctx.clearRect(0, 0, size, size);

    if (!analysisStore.showOwnership) return;

    const current = nearestOwnership();
    if (!current) return;
    const { ownership, forColor } = current;

    const boardSize = boardStore.size;
    if (ownership.length !== boardSize * boardSize) return; // 보드 크기와 안 맞으면(스트림 전환 중 등) 건너뜀

    const { margin, step } = metrics(size, boardSize);
    // 이미 돌이 있는 칸은 사각형이 돌을 완전히 덮어버리면 실제 착수된 돌 색을
    // 구분할 수 없으므로, 칸을 꽉 채우던 0.88에서 절반(0.5)으로 줄여 그 밑의
    // 돌/바둑판이 계속 비쳐 보이게 함.
    const squareSide = step * 0.5;
    const half = squareSide / 2;

    ctx.save();
    ctx.globalAlpha = 0.92;
    for (let y = 0; y < boardSize; y++) {
      for (let x = 0; x < boardSize; x++) {
        // ownership은 row-major(위쪽 줄부터, 각 줄은 왼쪽부터)로, 로컬 y=0도 맨
        // 위 줄이라 인덱스 계산이 그대로 맞아떨어짐 (coords.ts 상단 설명 참고)
        const raw = ownership[y * boardSize + x];
        const blackOwnership = forColor === "black" ? raw : -raw;

        // 그레이스케일 그라데이션 대신 흑/백 중 실제로 우세한 쪽 색 하나만 칠하고,
        // 그 확신도(|blackOwnership|, 0=경합~1=확정)를 그대로 alpha로 씀 - 그래서
        // 가운데(경합 지점, 0 근처)일수록 투명해지고 확정적인 곳일수록 진하게 보임.
        const color = blackOwnership >= 0 ? "0, 0, 0" : "255, 255, 255";
        const alpha = Math.abs(blackOwnership);
        const cx = margin + x * step;
        const cy = margin + y * step;

        ctx.fillStyle = `rgba(${color}, ${alpha})`;
        ctx.fillRect(cx - half, cy - half, squareSide, squareSide);
      }
    }
    ctx.restore();
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
    boardStore.size;
    boardStore.ancestorChain;
    analysisStore.showOwnership;
    draw();
  });
</script>

<canvas bind:this={canvasEl} class="board-layer ownership-overlay"></canvas>

<style>
  .board-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
</style>
