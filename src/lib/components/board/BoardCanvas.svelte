<script lang="ts">
  // 배경/격자/실제 돌 렌더링 + 2단계 착수 인터랙션(임시 선택 -> 확정)
  import { gameTreeStore } from "../../stores/gameTree.svelte";
  import { pendingMoveStore } from "../../stores/pendingMove.svelte";
  import { instantMoveStore } from "../../stores/instantMove.svelte";
  import { cellCenter, gridMetrics, nearestCell } from "../../canvas/boardGrid";
  import { canvasLayer } from "../../canvas/canvasLayer";
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
    const boardSize = gameTreeStore.size;
    const metrics = gridMetrics(size, boardSize);
    const { margin, step } = metrics;
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
          const { cx, cy } = cellCenter(col, row, metrics);
          ctx.beginPath();
          ctx.arc(cx, cy, dotRadius, 0, Math.PI * 2);
          ctx.fill();
        }
      }
    }

    // 실제 착수된 돌
    const pending = pendingMoveStore.pendingMove;
    const pendingOnStone = pendingMoveStore.pendingOnStone;
    for (let y = 0; y < boardSize; y++) {
      for (let x = 0; x < boardSize; x++) {
        const stone = gameTreeStore.stones[y][x];
        if (!stone) continue;
        const { cx, cy } = cellCenter(x, y, metrics);
        drawStone(ctx, cx, cy, stoneRadius, stone);
      }
    }

    // 가장 마지막으로 착수된 돌의 중앙에 반대 색 동그라미 표시
    const lastMove = gameTreeStore.lastMove;
    if (lastMove) {
      const lastStone = gameTreeStore.stones[lastMove.y][lastMove.x];
      if (lastStone) {
        const { cx, cy } = cellCenter(lastMove.x, lastMove.y, metrics);
        drawLastMoveMarker(ctx, cx, cy, stoneRadius, lastStone);
      }
    } else if (gameTreeStore.lastMoveIsPass) {
      // 직전 차례가 pass였으면(좌표가 없어 동그라미로 표시할 자리가 없으므로) 보드
      // 위쪽 가운데에 텍스트로 안내 - 누가 pass했는지는 지금 차례(currentTurn)의
      // 반대쪽이므로 그걸로 색을 표기함.
      const passedColor = gameTreeStore.currentTurn === "black" ? "white" : "black";
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
    for (const child of gameTreeStore.currentChildren) {
      const { cx, cy } = cellCenter(child.x, child.y, metrics);
      drawChildMarker(ctx, cx, cy, stoneRadius, child.color);
    }

    // 임시 선택(확정 전) 미리보기 + 십자선
    if (pending) {
      const { cx, cy } = cellCenter(pending.x, pending.y, metrics);
      if (!pendingOnStone) {
        // 빈 칸을 선택한 경우: 착수될 돌을 반투명 미리보기로 표시
        drawStone(ctx, cx, cy, stoneRadius, gameTreeStore.currentTurn, 0.45);
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

    const boardSize = gameTreeStore.size;
    return nearestCell(bx, by, gridMetrics(canvasEl.width, boardSize), boardSize);
  }

  // 드래그 상태: 누르는 순간부터 손가락/마우스를 따라 미리보기(빈 칸 -> 반투명 돌,
  // 이미 돌이 있는 칸 -> 제거 대상 표시)와 십자선이 계속 갱신됨.
  // 확정은 기본적으로 여기서 하지 않고 GameControls의 "Confirm Move" 버튼(또는 키보드
  // 단축키)을 눌렀을 때 이뤄지지만, 설정에서 "확인 없이 바로 착수"가 켜져 있으면
  // 마우스는 누르는 순간(handlePointerDown), 터치스크린은 손을 떼는 순간
  // (handlePointerUp)에 곧바로 확정한다 - instantMove.svelte.ts 참고. 키보드로 두는
  // 흐름은 이 설정과 무관하게 항상 별도 확정 키가 필요함.
  let dragging = false;

  function handlePointerDown(evt: PointerEvent) {
    const pos = nearestIntersection(evt);
    if (!pos) return;

    // 이미 임시 선택된 지점을 다시 누르면 그 선택을 취소함(별도 취소 버튼 없이도
    // 취소할 수 있도록).
    const pending = pendingMoveStore.pendingMove;
    if (pending && pending.x === pos.x && pending.y === pos.y) {
      pendingMoveStore.cancelPending();
      return;
    }

    if (instantMoveStore.enabled && evt.pointerType === "mouse") {
      // 마우스는 드래그 미리보기 없이 누르는 순간 바로 확정
      pendingMoveStore.selectPending(pos.x, pos.y);
      gameTreeStore.confirmMove();
      return;
    }

    canvasEl?.setPointerCapture(evt.pointerId);
    dragging = true;
    pendingMoveStore.selectPending(pos.x, pos.y);
  }

  function handlePointerMove(evt: PointerEvent) {
    if (!dragging) return;
    const pos = nearestIntersection(evt);
    if (!pos) return;

    pendingMoveStore.selectPending(pos.x, pos.y);
  }

  function handlePointerUp(evt: PointerEvent) {
    // 터치스크린(펜 포함)에서 "확인 없이 바로 착수"가 켜져 있으면 손을 떼는 순간 확정
    if (dragging && instantMoveStore.enabled && evt.pointerType !== "mouse") {
      gameTreeStore.confirmMove();
    }
    dragging = false;
  }

  function handlePointerCancel() {
    dragging = false;
  }

  // 방향키로도 같은 pendingMove(임시 선택)를 옮길 수 있게 함. 아직 선택된 칸이 없으면
  // 이미 진행 중인 선택(pendingMove) -> 게임 트리 상 현재 노드의 마지막 착수 지점
  // (lastMove) 순서로 시작 위치를 정함. 단, 첫 수(보드가 비어있고 이전 수도 없는 경우)는
  // 방향키를 누른 직후 바로 1,1 위치가 되어야 하므로 델타를 적용하지 않고 그 자리에만
  // 위치시킴. 확정은 마우스/터치와 마찬가지로 여기서 하지 않고 GameControls의 Confirm
  // Move 버튼 또는 (KeyboardShortcuts.svelte가 처리하는) Space 키로만 이뤄짐.
  function movePendingBy(dx: number, dy: number) {
    if (!pendingMoveStore.pendingMove && !gameTreeStore.lastMove) {
      pendingMoveStore.selectPending(0, 0);
      return;
    }
    const boardSize = gameTreeStore.size;
    const start = pendingMoveStore.pendingMove ?? gameTreeStore.lastMove!;
    const x = Math.min(boardSize - 1, Math.max(0, start.x + dx));
    const y = Math.min(boardSize - 1, Math.max(0, start.y + dy));
    pendingMoveStore.selectPending(x, y);
  }

  // 방향키(임시 선택 이동)만 처리 - confirmMove/back/removeLastMove/changeColor 등
  // Settings에서 재배정 가능한 단축키는 전부 KeyboardShortcuts.svelte(앱 최상단에서
  // 한 번만 마운트)로 옮겨졌음. 방향키는 재배정 대상이 아니라 여기 그대로 둠.
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
  }

  $effect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  $effect(() => {
    // 보드 크기, 돌 배치, 임시 선택, 배경 이미지 로드 상태 변경 시 다시 그림
    gameTreeStore.size;
    gameTreeStore.stones;
    pendingMoveStore.pendingMove;
    gameTreeStore.currentTurn;
    gameTreeStore.lastMove;
    gameTreeStore.lastMoveIsPass;
    gameTreeStore.currentChildren;
    backgroundReady;
    stoneReady.black;
    stoneReady.white;
    draw();
  });
</script>

<canvas
  bind:this={canvasEl}
  class="board-layer"
  use:canvasLayer={draw}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerCancel}
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
