// 바둑판 캔버스 격자 계산 - 순수 함수만 모아둠(캔버스/DOM 접근 없음, 그래서 유닛
// 테스트로 검증 가능). BoardCanvas.svelte/AnalysisOverlay.svelte/
// OwnershipOverlay.svelte 3곳에 글자 그대로 복사돼 있던 margin/step 계산과 칸 중심
// 좌표 계산, 그리고 BoardCanvas.svelte에만 있던 "포인터 좌표 -> 가장 가까운 교차점"
// 클램프 로직을 여기 하나로 모았다.

export interface GridMetrics {
  // 바둑판 가장자리 여백(캔버스 backing store 좌표 기준)
  margin: number;
  // 인접한 두 교차점 사이 간격(캔버스 backing store 좌표 기준)
  step: number;
}

/** 캔버스(backing store) 좌표 기준 격자 치수 계산. */
export function gridMetrics(canvasSize: number, boardSize: number): GridMetrics {
  const margin = canvasSize / (boardSize + 1);
  const step = (canvasSize - margin * 2) / (boardSize - 1);
  return { margin, step };
}

/** 열(col)/행(row) 교차점의 캔버스 좌표 중심점. */
export function cellCenter(
  col: number,
  row: number,
  metrics: GridMetrics,
): { cx: number; cy: number } {
  return { cx: metrics.margin + col * metrics.step, cy: metrics.margin + row * metrics.step };
}

/**
 * 캔버스 backing-store 좌표(bx, by)에 가장 가까운 교차점(x=열, y=행)을 반환.
 * 드래그 중 손가락/마우스가 보드 가장자리를 살짝 벗어나도 자연스럽게 따라오도록
 * 항상 보드 범위 안으로 clamp한다.
 */
export function nearestCell(
  bx: number,
  by: number,
  metrics: GridMetrics,
  boardSize: number,
): { x: number; y: number } {
  const x = Math.min(
    boardSize - 1,
    Math.max(0, Math.round((bx - metrics.margin) / metrics.step)),
  );
  const y = Math.min(
    boardSize - 1,
    Math.max(0, Math.round((by - metrics.margin) / metrics.step)),
  );
  return { x, y };
}
