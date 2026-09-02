// GTP vertex(예: "Q16") -> 로컬 보드 좌표(x=열, y=행, 둘 다 0-indexed) 변환.
// src-tauri/src/gtp/coords.rs의 from_vertex()와 반드시 같은 규칙을 따라야 함 -
// kata-analyze 결과의 candidate.move가 이 GTP 표기 그대로 넘어오므로, 여기서
// 그 값을 캔버스에 그릴 픽셀 좌표로 바꾸는 데 쓰인다.
//
// 로컬 좌표계: x는 왼쪽부터 0-based 열 인덱스, y는 "위쪽"부터 0-based 행 인덱스.
// GTP 좌표계: 열은 A~T(관례상 I 제외) 왼쪽부터, 행은 1이 맨 "아래쪽" 줄, size가
// 맨 "위쪽" 줄 -> GTP 행 번호 = size - y.
const COLUMN_LETTERS = "ABCDEFGHJKLMNOPQRST"; // I는 관례상 건너뜀 (19개, 19줄까지 지원)

/// GTP vertex 문자열을 로컬 좌표로 파싱. 잘못된 형식/범위를 벗어나거나
/// "pass"/"resign"이면 null (좌표가 없는 수이므로 오버레이에서 건너뜀).
export function fromVertex(
  vertex: string,
  size: number,
): { x: number; y: number } | null {
  const trimmed = vertex.trim();
  if (trimmed.length < 2) return null;

  const colChar = trimmed[0].toUpperCase();
  const x = COLUMN_LETTERS.indexOf(colChar);
  if (x < 0) return null;

  const row = Number(trimmed.slice(1));
  if (!Number.isInteger(row) || row <= 0 || row > size || x >= size) {
    return null;
  }

  return { x, y: size - row };
}
