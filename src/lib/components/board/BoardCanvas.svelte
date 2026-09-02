<script lang="ts">
  // 배경/격자/실제 돌 렌더링 + 2단계 착수 인터랙션(임시 선택 -> 확정)
  import { boardStore } from "../../stores/board.svelte";
  import { keybindingsStore } from "../../stores/keybindings.svelte";
  // 바둑판 배경: 실제 나무 질감 이미지로 교체 가능하도록 파일에서 로드
  // (현재는 단순 갈색 placeholder, src-tauri/icons와 마찬가지로 추후 실제 에셋으로 교체 예정)
  import boardBackgroundUrl from "../../../assets/board-background.png";
  // 바둑돌도 이미지 에셋 기반 렌더링 (현재는 간단한 그라디언트 원 placeholder)
  import stoneBlackUrl from "../../../assets/stone-black.png";
  import stoneWhiteUrl from "../../../assets/stone-white.png";

  // 19x19 기준 화점(hoshi) 좌표 (0-indexed)
  const STAR_POINTS_19 = [3, 9, 15];

  // 맞닿은 두 돌의 테두리 사이 간격 = 격자 간격(step)의 5%
  // 2r = step - 0.05*step  ->  r = step * 0.475
  const STONE_GAP_RATIO = 0.05;
  const STONE_RADIUS_RATIO = (1 - STONE_GAP_RATIO) / 2;

  let canvasEl: HTMLCanvasElement | undefined = $state();
  let backgroundReady = $state(false);

  const backgroundImage = new Image();
  backgroundImage.src = boardBackgroundUrl;
  backgroundImage.onload = () => {
    backgroundReady = true;
  };

  const stoneImages: Record<"black" | "white", HTMLImageElement> = {
    black: new Image(),
    white: new Image(),
  };
  let stoneReady = $state({ black: false, white: false });
  stoneImages.black.src = stoneBlackUrl;
  stoneImages.white.src = stoneWhiteUrl;
  stoneImages.black.onload = () => {
    stoneReady.black = true;
  };
  stoneImages.white.onload = () => {
    stoneReady.white = true;
  };

  // 캔버스(backing store) 좌표 기준 격자 치수 계산
  function metrics(canvasSize: number, boardSize: number) {
    const margin = canvasSize / (boardSize + 1);
    const step = (canvasSize - margin * 2) / (boardSize - 1);
    return { margin, step };
  }

  function drawStone(
    ctx: CanvasRenderingContext2D,
    cx: number,
    cy: number,
    radius: number,
    color: "black" | "white",
    alpha = 1,
  ) {
    ctx.save();
    ctx.globalAlpha = alpha;

    if (stoneReady[color]) {
      ctx.drawImage(
        stoneImages[color],
        cx - radius,
        cy - radius,
        radius * 2,
        radius * 2,
      );
    } else {
      // 이미지 로딩 전 fallback: 단순 단색 원
      ctx.beginPath();
      ctx.arc(cx, cy, radius, 0, Math.PI * 2);
      ctx.fillStyle = color === "black" ? "#111111" : "#f2f2f2";
      ctx.fill();
    }

    ctx.restore();
  }

  // 마지막으로 착수된 돌 표시: 돌과 반대 색의 작은 동그라미 (돌 크기의 30%)
  const LAST_MOVE_MARKER_RATIO = 0.3;

  function drawLastMoveMarker(
    ctx: CanvasRenderingContext2D,
    cx: number,
    cy: number,
    stoneRadius: number,
    stoneColor: "black" | "white",
  ) {
    const markerRadius = stoneRadius * LAST_MOVE_MARKER_RATIO;
    ctx.save();
    ctx.beginPath();
    ctx.arc(cx, cy, markerRadius, 0, Math.PI * 2);
    ctx.fillStyle = stoneColor === "black" ? "#ffffff" : "#111111";
    ctx.fill();
    ctx.restore();
  }

  // 현재 게임 트리 노드에서 갈라지는 다음 수 후보(자식 노드) 위치 표시: 작고
  // 반투명한 동그라미 (돌 크기의 35%, 알파 40%)
  const CHILD_MARKER_RATIO = 0.35;
  const CHILD_MARKER_ALPHA = 0.4;

  function drawChildMarker(
    ctx: CanvasRenderingContext2D,
    cx: number,
    cy: number,
    stoneRadius: number,
    color: "black" | "white",
  ) {
    const markerRadius = stoneRadius * CHILD_MARKER_RATIO;
    ctx.save();
    ctx.globalAlpha = CHILD_MARKER_ALPHA;
    ctx.beginPath();
    ctx.arc(cx, cy, markerRadius, 0, Math.PI * 2);
    ctx.fillStyle = color === "black" ? "#111111" : "#f2f2f2";
    ctx.fill();
    ctx.restore();
  }

  // 임시 선택 위치를 지나는 행 전체 + 열 전체에 십자선을 그림 (격자선보다 약간 두껍게)
  function drawCrosshair(
    ctx: CanvasRenderingContext2D,
    cx: number,
    cy: number,
    margin: number,
    canvasSize: number,
    gridLineWidth: number,
  ) {
    ctx.save();
    ctx.strokeStyle = "#d64545";
    ctx.lineWidth = gridLineWidth * 3.5;
    ctx.beginPath();
    ctx.moveTo(margin, cy);
    ctx.lineTo(canvasSize - margin, cy);
    ctx.moveTo(cx, margin);
    ctx.lineTo(cx, canvasSize - margin);
    ctx.stroke();
    ctx.restore();
  }

  function draw() {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;

    const size = canvasEl.width; // 캔버스는 항상 정사각형
    const boardSize = boardStore.size;
    const { margin, step } = metrics(size, boardSize);
    const stoneRadius = step * STONE_RADIUS_RATIO;

    ctx.clearRect(0, 0, size, size);

    if (backgroundReady) {
      ctx.drawImage(backgroundImage, 0, 0, size, size);
    } else {
      // 이미지 로딩 전 fallback 색상
      ctx.fillStyle = "#d2a564";
      ctx.fillRect(0, 0, size, size);
    }

    const gridLineWidth = Math.max(1, size / 600);
    ctx.strokeStyle = "#2b1a0e";
    ctx.lineWidth = gridLineWidth;
    ctx.beginPath();
    for (let i = 0; i < boardSize; i++) {
      const pos = margin + i * step;
      ctx.moveTo(pos, margin);
      ctx.lineTo(pos, size - margin);
      ctx.moveTo(margin, pos);
      ctx.lineTo(size - margin, pos);
    }
    ctx.stroke();

    if (boardSize === 19) {
      ctx.fillStyle = "#2b1a0e";
      const dotRadius = Math.max(2, size / 200);
      for (const row of STAR_POINTS_19) {
        for (const col of STAR_POINTS_19) {
          const cx = margin + col * step;
          const cy = margin + row * step;
          ctx.beginPath();
          ctx.arc(cx, cy, dotRadius, 0, Math.PI * 2);
          ctx.fill();
        }
      }
    }

    // 실제 착수된 돌
    const pending = boardStore.pendingMove;
    const pendingOnStone = boardStore.pendingOnStone;
    for (let y = 0; y < boardSize; y++) {
      for (let x = 0; x < boardSize; x++) {
        const stone = boardStore.stones[y][x];
        if (!stone) continue;
        const cx = margin + x * step;
        const cy = margin + y * step;
        drawStone(ctx, cx, cy, stoneRadius, stone);
      }
    }

    // 가장 마지막으로 착수된 돌의 중앙에 반대 색 동그라미 표시
    const lastMove = boardStore.lastMove;
    if (lastMove) {
      const lastStone = boardStore.stones[lastMove.y][lastMove.x];
      if (lastStone) {
        const cx = margin + lastMove.x * step;
        const cy = margin + lastMove.y * step;
        drawLastMoveMarker(ctx, cx, cy, stoneRadius, lastStone);
      }
    } else if (boardStore.lastMoveIsPass) {
      // 직전 차례가 pass였으면(좌표가 없어 동그라미로 표시할 자리가 없으므로) 보드
      // 위쪽 가운데에 텍스트로 안내 - 누가 pass했는지는 지금 차례(currentTurn)의
      // 반대쪽이므로 그걸로 색을 표기함.
      const passedColor = boardStore.currentTurn === "black" ? "white" : "black";
      ctx.save();
      ctx.font = `bold ${Math.round(step * 0.32)}px sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillStyle = passedColor === "black" ? "#111111" : "#f2f2f2";
      ctx.strokeStyle = passedColor === "black" ? "#f2f2f2" : "#111111";
      ctx.lineWidth = Math.max(1, step * 0.03);
      const cx = size / 2;
      const cy = margin * 0.65;
      ctx.strokeText("PASS", cx, cy);
      ctx.fillText("PASS", cx, cy);
      ctx.restore();
    }

    // 현재 노드에서 갈라지는 다음 수 후보(게임 트리 자식) 지점 표시
    for (const child of boardStore.currentChildren) {
      const cx = margin + child.x * step;
      const cy = margin + child.y * step;
      drawChildMarker(ctx, cx, cy, stoneRadius, child.color);
    }

    // 임시 선택(확정 전) 미리보기 + 십자선
    if (pending) {
      const cx = margin + pending.x * step;
      const cy = margin + pending.y * step;
      if (!pendingOnStone) {
        // 빈 칸을 선택한 경우: 착수될 돌을 반투명 미리보기로 표시
        drawStone(ctx, cx, cy, stoneRadius, boardStore.currentTurn, 0.45);
      }
      drawCrosshair(ctx, cx, cy, margin, size, gridLineWidth);
    }
  }

  // 포인터 좌표 -> 가장 가까운 교차점(x=열, y=행) 변환. 드래그 중 손가락이 보드
  // 가장자리를 살짝 벗어나도 자연스럽게 따라오도록 항상 보드 범위 안으로 clamp.
  function nearestIntersection(
    evt: PointerEvent,
  ): { x: number; y: number } | null {
    if (!canvasEl) return null;
    const rect = canvasEl.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return null;

    const scaleX = canvasEl.width / rect.width;
    const scaleY = canvasEl.height / rect.height;
    const bx = (evt.clientX - rect.left) * scaleX;
    const by = (evt.clientY - rect.top) * scaleY;

    const boardSize = boardStore.size;
    const { margin, step } = metrics(canvasEl.width, boardSize);

    const col = Math.min(
      boardSize - 1,
      Math.max(0, Math.round((bx - margin) / step)),
    );
    const row = Math.min(
      boardSize - 1,
      Math.max(0, Math.round((by - margin) / step)),
    );

    return { x: col, y: row };
  }

  // 드래그 상태: 누르는 순간부터 손가락/마우스를 따라 미리보기(빈 칸 -> 반투명 돌,
  // 이미 돌이 있는 칸 -> 제거 대상 표시)와 십자선이 계속 갱신됨.
  // 확정은 여기서 하지 않고 GameControls의 "Confirm Move"/"Remove Stone" 버튼을
  // 눌렀을 때만 이뤄짐.
  let dragging = false;

  function handlePointerDown(evt: PointerEvent) {
    const pos = nearestIntersection(evt);
    if (!pos) return;

    canvasEl?.setPointerCapture(evt.pointerId);
    dragging = true;
    boardStore.selectPending(pos.x, pos.y);
  }

  function handlePointerMove(evt: PointerEvent) {
    if (!dragging) return;
    const pos = nearestIntersection(evt);
    if (!pos) return;

    boardStore.selectPending(pos.x, pos.y);
  }

  function endDrag() {
    dragging = false;
  }

  // 방향키로도 같은 pendingMove(임시 선택)를 옮길 수 있게 함. 아직 선택된 칸이 없으면
  // 이미 진행 중인 선택(pendingMove) -> 게임 트리 상 현재 노드의 마지막 착수 지점
  // (lastMove) 순서로 시작 위치를 정함. 단, 첫 수(보드가 비어있고 이전 수도 없는 경우)는
  // 방향키를 누른 직후 바로 1,1 위치가 되어야 하므로 델타를 적용하지 않고 그 자리에만
  // 위치시킴. 확정은 마우스/터치와 마찬가지로 여기서 하지 않고 GameControls의 Confirm
  // Move 버튼 또는 Space 키로만 이뤄짐.
  function movePendingBy(dx: number, dy: number) {
    if (!boardStore.pendingMove && !boardStore.lastMove) {
      boardStore.selectPending(0, 0);
      return;
    }
    const boardSize = boardStore.size;
    const start = boardStore.pendingMove ?? boardStore.lastMove!;
    const x = Math.min(boardSize - 1, Math.max(0, start.x + dx));
    const y = Math.min(boardSize - 1, Math.max(0, start.y + dy));
    boardStore.selectPending(x, y);
  }

  function handleKeyDown(evt: KeyboardEvent) {
    // 다른 곳(입력창 등)에 포커스가 있거나 다른 단축키 조합이면 무시
    const target = evt.target as HTMLElement | null;
    if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
    if (evt.ctrlKey || evt.altKey || evt.metaKey) return;

    switch (evt.key) {
      case "ArrowUp":
        evt.preventDefault();
        movePendingBy(0, -1);
        return;
      case "ArrowDown":
        evt.preventDefault();
        movePendingBy(0, 1);
        return;
      case "ArrowLeft":
        evt.preventDefault();
        movePendingBy(-1, 0);
        return;
      case "ArrowRight":
        evt.preventDefault();
        movePendingBy(1, 0);
        return;
    }

    // 착수 확정/색 전환/뒤로 가기/마지막 수 제거는 Settings에서 재배정 가능한 키를 사용
    if (keybindingsStore.matches("confirmMove", evt.key)) {
      evt.preventDefault();
      boardStore.confirmMove();
    } else if (keybindingsStore.matches("back", evt.key)) {
      evt.preventDefault();
      boardStore.goBack();
    } else if (keybindingsStore.matches("removeLastMove", evt.key)) {
      evt.preventDefault();
      boardStore.removeLastMove();
    } else if (keybindingsStore.matches("changeColor", evt.key)) {
      evt.preventDefault();
      boardStore.toggleTurn();
    }
  }

  $effect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

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
    // 보드 크기, 돌 배치, 임시 선택, 배경 이미지 로드 상태 변경 시 다시 그림
    boardStore.size;
    boardStore.stones;
    boardStore.pendingMove;
    boardStore.currentTurn;
    boardStore.lastMove;
    boardStore.lastMoveIsPass;
    boardStore.currentChildren;
    backgroundReady;
    stoneReady.black;
    stoneReady.white;
    draw();
  });
</script>

<canvas
  bind:this={canvasEl}
  class="board-layer"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={endDrag}
  onpointercancel={endDrag}
></canvas>

<style>
  .board-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    cursor: pointer;
    /* 모바일 WebKit/Chrome 기본 탭 하이라이트(누르고 있는 동안 파랗게 번지는 효과) 제거 */
    -webkit-tap-highlight-color: transparent;
    /* 드래그 제스처를 직접 처리하므로 브라우저 기본 스크롤/줌 제스처는 막음 */
    touch-action: none;
    -webkit-user-select: none;
    user-select: none;
    outline: none;
  }
</style>
