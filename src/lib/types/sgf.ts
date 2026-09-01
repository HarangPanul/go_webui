// SGF import 결과 타입
import type { Move } from "./board";

export interface SgfGame {
  boardSize: number;
  komi: number;
  moves: Move[];
}
