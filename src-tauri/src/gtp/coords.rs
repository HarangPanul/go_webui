// GTP vertex(예: "Q16") <-> 로컬 보드 좌표(x=열, y=행, 둘 다 0-indexed) 변환.
//
// 로컬 좌표계: x는 왼쪽부터 0-based 열 인덱스, y는 "위쪽"부터 0-based 행 인덱스
// (BoardCanvas가 캔버스 y좌표 증가 방향 = 아래쪽으로 그대로 그리므로 y=0이 화면
// 맨 윗줄).
// GTP 좌표계: 열은 A~T(관례상 I 제외) 왼쪽부터, 행은 1이 맨 "아래쪽" 줄, size가
// 맨 "위쪽" 줄 -> 로컬 y=0(맨 위 줄)이 GTP 행 번호로는 size, y=size-1(맨 아래 줄)이
// GTP 행 번호로는 1. 즉 GTP 행 번호 = size - y.

const COLUMN_LETTERS: &str = "ABCDEFGHJKLMNOPQRST"; // I는 관례상 건너뜀 (19개, 19줄까지 지원)

pub fn to_vertex(x: usize, y: usize, size: usize) -> String {
    debug_assert!(x < size && x < COLUMN_LETTERS.len() && y < size);
    let col = COLUMN_LETTERS.as_bytes()[x] as char;
    let row = size - y;
    format!("{col}{row}")
}

/// GTP vertex 문자열을 로컬 좌표로 파싱. 잘못된 형식/범위를 벗어나면 None
/// (`pass`/`resign`은 vertex가 아니므로 여기서 처리하지 않고 호출자가 별도로 걸러냄).
pub fn from_vertex(vertex: &str, size: usize) -> Option<(usize, usize)> {
    let vertex = vertex.trim();
    let mut chars = vertex.chars();
    let col_char = chars.next()?.to_ascii_uppercase();
    let x = COLUMN_LETTERS.find(col_char)?;
    let row: usize = chars.as_str().parse().ok()?;
    if row == 0 || row > size || x >= size {
        return None;
    }
    Some((x, size - row))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_corners_19() {
        assert_eq!(to_vertex(0, 0, 19), "A19");
        assert_eq!(to_vertex(18, 18, 19), "T1");
        assert_eq!(to_vertex(8, 3, 19), "J16"); // 9번째 열(index 8): I를 건너뛰므로 J
    }

    #[test]
    fn roundtrips_full_board() {
        for x in 0..19 {
            for y in 0..19 {
                let v = to_vertex(x, y, 19);
                assert_eq!(from_vertex(&v, 19), Some((x, y)));
            }
        }
    }

    #[test]
    fn parses_lowercase() {
        assert_eq!(from_vertex("q16", 19), from_vertex("Q16", 19));
    }

    #[test]
    fn rejects_out_of_range_or_malformed() {
        assert_eq!(from_vertex("Z1", 19), None);
        assert_eq!(from_vertex("A20", 19), None);
        assert_eq!(from_vertex("A0", 19), None);
        assert_eq!(from_vertex("", 19), None);
        assert_eq!(from_vertex("pass", 19), None);
    }
}
