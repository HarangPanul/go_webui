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
  // "kata-analyze ... ownership true"로 요청했을 때만 채워짐. 보드 전체 지점을
  // row-major(위쪽 줄부터, 각 줄은 왼쪽부터)로 이어붙인 값, [-1, 1] 범위 -
  // winrate/scoreLead와 같은 기준(분석 요청 당시 둘 차례였던 색)으로 1에
  // 가까울수록 그 색 소유, -1에 가까울수록 상대 소유.
  ownership?: number[] | null;
}

// Rust process.rs::KataAnalyzeEvent와 대응(serde flatten으로 KataAnalyzeResult 필드가
// nodeId/forColor와 같은 레벨에 나란히 실림). "kata-analyze" Tauri 이벤트의 실제 payload.
export interface KataAnalyzeEvent extends KataAnalyzeResult {
  // 이 결과가 어느 게임 트리 노드(board.svelte.ts::nodeId) 기준인지 - analysisStore가
  // 노드별로 결과를 캐싱하는 키.
  nodeId: number;
  // 이 결과의 winrate/scoreLead/ownership이 어느 색 기준으로 계산됐는지(분석 요청 당시
  // 둘 차례였던 색). Rust가 명령을 보내는 시점에 직접 기록해두므로 프런트에서 따로
  // 추적할 필요가 없다.
  forColor: "black" | "white";
}
