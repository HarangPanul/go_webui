<script lang="ts">
  // kata-analyze의 ownership 필드(GameControls가 "ownership true"를 항상 붙여
  // 요청함)를 바둑판 위에 흑/백 그레이스케일로 오버레이. analysisStore.showOwnership
  // 토글이 켜져 있을 때만 그림 - Analysis/Ownership 두 오버레이가 항상 같은 데이터를
  // 받고 있어도 표시 여부는 독립적으로 켜고 끌 수 있음.
  //
  // ownership 값은 kata-analyze의 winrate/scoreLead와 같은 기준(분석 요청 당시
  // 둘 차례였던 색, analysisStore.current.forColor)으로 [-1, 1] - 그대로 쓰면 같은 흑 집이
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

  function draw() {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;

    const size = canvasEl.width;
    ctx.clearRect(0, 0, size, size);

    if (!analysisStore.showOwnership) return;

    const current = analysisStore.current;
    const ownership = current?.result.ownership;
    const forColor = current?.forColor;
    if (!ownership || !forColor) return;

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

        // blackOwnership: 1 -> 검정(0,0,0), -1 -> 흰색(255,255,255), 0 -> 중간 회색
        const v = Math.round((1 - blackOwnership) * 127.5);
        const cx = margin + x * step;
        const cy = margin + y * step;

        ctx.fillStyle = `rgb(${v}, ${v}, ${v})`;
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
    analysisStore.current;
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
