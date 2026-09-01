export type Color = "black" | "white";

export interface Point {
  x: number;
  y: number;
}

export interface Move {
  color: Color;
  point: Point | null; // null = pass
}
