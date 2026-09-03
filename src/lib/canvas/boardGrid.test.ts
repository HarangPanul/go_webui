import { describe, expect, it } from "vitest";
import { cellCenter, gridMetrics, nearestCell } from "./boardGrid";

describe("gridMetrics", () => {
  it("computes margin/step for a 19-line board on a 600px canvas", () => {
    const { margin, step } = gridMetrics(600, 19);
    expect(margin).toBeCloseTo(30, 6); // 600 / 20
    expect(step).toBeCloseTo((600 - 30 * 2) / 18, 6);
  });

  it("gives a bigger step for a smaller board on the same canvas", () => {
    const big = gridMetrics(600, 19);
    const small = gridMetrics(600, 9);
    expect(small.step).toBeGreaterThan(big.step);
  });
});

describe("cellCenter", () => {
  it("places (0, 0) exactly at the margin", () => {
    const metrics = gridMetrics(600, 19);
    expect(cellCenter(0, 0, metrics)).toEqual({ cx: metrics.margin, cy: metrics.margin });
  });

  it("places the last row/col exactly at canvasSize - margin", () => {
    const canvasSize = 600;
    const boardSize = 19;
    const metrics = gridMetrics(canvasSize, boardSize);
    const { cx, cy } = cellCenter(boardSize - 1, boardSize - 1, metrics);
    expect(cx).toBeCloseTo(canvasSize - metrics.margin, 6);
    expect(cy).toBeCloseTo(canvasSize - metrics.margin, 6);
  });
});

describe("nearestCell", () => {
  const canvasSize = 600;
  const boardSize = 19;
  const metrics = gridMetrics(canvasSize, boardSize);

  it("round-trips through cellCenter for every intersection", () => {
    for (let y = 0; y < boardSize; y++) {
      for (let x = 0; x < boardSize; x++) {
        const { cx, cy } = cellCenter(x, y, metrics);
        expect(nearestCell(cx, cy, metrics, boardSize)).toEqual({ x, y });
      }
    }
  });

  it("snaps to the nearer intersection when between two grid lines", () => {
    const { cx: x0 } = cellCenter(0, 0, metrics);
    const { cx: x1 } = cellCenter(1, 0, metrics);
    const justPastMidpoint = x0 + (x1 - x0) * 0.6;
    expect(nearestCell(justPastMidpoint, metrics.margin, metrics, boardSize)).toEqual({
      x: 1,
      y: 0,
    });
  });

  it("clamps negative coordinates (pointer past the top-left edge) to (0, 0)", () => {
    expect(nearestCell(-500, -500, metrics, boardSize)).toEqual({ x: 0, y: 0 });
  });

  it("clamps coordinates past the bottom-right edge to the last intersection", () => {
    expect(nearestCell(canvasSize + 500, canvasSize + 500, metrics, boardSize)).toEqual({
      x: boardSize - 1,
      y: boardSize - 1,
    });
  });
});
