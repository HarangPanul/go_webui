// kata-analyze 출력 파싱 결과 타입 (Rust 파서와 필드 대응)

export interface KataAnalyzeMove {
  move: string;
  visits: number;
  winrate: number;
  scoreLead: number;
  pv: string[];
}

export interface KataAnalyzeResult {
  moveNumber: number;
  candidates: KataAnalyzeMove[];
}
