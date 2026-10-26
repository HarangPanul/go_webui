// SGF(Smart Game Format) 파서 + 게임 트리 변환. 파일 내용을 문자열로 받아
// game::GameTree로 만들어 돌려주는 순수 함수라 엔진/상태와 무관하게 테스트할 수 있다
// (실제 로드와 엔진 재동기화는 services::game_service::GameService::import_sgf).
//
// 지원 범위: 첫 번째 대국(여러 대국이 든 컬렉션이어도 첫 판만), 정사각형 19로 이하 보드
// (SZ), 덤(KM), 흑/백 착수(B/W, 패스 포함), 변화도(variation), 루트의 놓은 돌(AB/AW)과
// 선 차례(PL). 루트가 아닌 노드의 AB/AW/AE는 로컬 게임 트리가 표현할 수 없어 에러로
// 거부한다(조용히 무시하면 보드가 기보와 어긋나므로). 그 밖의 속성(주석, 마커, 대국자
// 이름 등)은 읽고 버린다.
use crate::game::{Color, GameTree};

/// 파일 크기 상한 - 실제 기보는 수백 KB를 넘지 않는다. 터무니없이 큰 입력을 받아
/// 메모리를 잡아먹지 않도록 막는 안전장치.
const MAX_SGF_BYTES: usize = 8 * 1024 * 1024;
/// 변화도 괄호 중첩 깊이 상한 - 재귀 파싱이 스택을 넘지 않게 막는다.
const MAX_VARIATION_DEPTH: usize = 256;
/// GTP 열 표기(A~T, I 제외)가 19줄까지만 정의돼 있다(gtp::coords).
const MAX_BOARD_SIZE: usize = 19;

pub struct SgfGame {
    /// 본 줄기 끝에 커서가 놓인 게임 트리.
    pub tree: GameTree,
    /// KM이 있으면 그 값. 없으면 None(호출자가 현재 덤을 그대로 유지).
    pub komi: Option<f64>,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SgfError {
    #[error("SGF 파일이 너무 큽니다")]
    TooLarge,
    #[error("SGF 형식이 아닙니다 (대국 정보를 찾지 못했습니다)")]
    NotSgf,
    #[error("SGF 문법 오류: {0}")]
    Syntax(String),
    #[error("지원하지 않는 보드 크기입니다: {0} (정사각형 2~19줄만 지원)")]
    UnsupportedSize(String),
    #[error("잘못된 좌표입니다: {0}")]
    BadPoint(String),
    #[error("{0}번째 수가 이미 돌이 놓인 자리입니다")]
    IllegalMove(usize),
    #[error("{0}")]
    Unsupported(String),
}

type Prop = (String, Vec<String>);

// 변화도 하나: 노드 열(;...;...) 뒤에 갈라지는 하위 가지들이 이어진다.
struct Sequence {
    nodes: Vec<Vec<Prop>>,
    variations: Vec<Sequence>,
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    // '(' 로 시작하는 게임 트리 하나를 읽는다.
    fn parse_tree(&mut self, depth: usize) -> Result<Sequence, SgfError> {
        if depth > MAX_VARIATION_DEPTH {
            return Err(SgfError::Syntax("변화도 중첩이 너무 깊습니다".into()));
        }
        self.skip_ws();
        if self.peek() != Some(b'(') {
            return Err(SgfError::Syntax("'('가 필요합니다".into()));
        }
        self.pos += 1;

        let mut seq = Sequence {
            nodes: Vec::new(),
            variations: Vec::new(),
        };
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b';') => {
                    // 변화도 뒤에 노드가 오면 문법 위반(FF4) - 관대하게 받지 않는다.
                    if !seq.variations.is_empty() {
                        return Err(SgfError::Syntax("변화도 뒤에 노드가 올 수 없습니다".into()));
                    }
                    self.pos += 1;
                    seq.nodes.push(self.parse_node()?);
                }
                Some(b'(') => {
                    let child = self.parse_tree(depth + 1)?;
                    seq.variations.push(child);
                }
                Some(b')') => {
                    self.pos += 1;
                    return Ok(seq);
                }
                Some(c) => {
                    return Err(SgfError::Syntax(format!(
                        "예상치 못한 문자 '{}'",
                        c as char
                    )))
                }
                None => return Err(SgfError::Syntax("')'가 없습니다".into())),
            }
        }
    }

    // ';' 다음의 속성들을 읽는다.
    fn parse_node(&mut self) -> Result<Vec<Prop>, SgfError> {
        let mut props = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(c) if c.is_ascii_alphabetic() => {
                    // FF3 이전 파일은 속성 이름에 소문자가 섞여 있다 - 대문자만 남긴다.
                    let mut ident = String::new();
                    while let Some(c) = self.peek().filter(u8::is_ascii_alphabetic) {
                        if c.is_ascii_uppercase() {
                            ident.push(c as char);
                        }
                        self.pos += 1;
                    }
                    let mut values = Vec::new();
                    loop {
                        self.skip_ws();
                        if self.peek() != Some(b'[') {
                            break;
                        }
                        values.push(self.parse_value()?);
                    }
                    if values.is_empty() {
                        return Err(SgfError::Syntax(format!("속성 {ident}에 값이 없습니다")));
                    }
                    props.push((ident, values));
                }
                _ => return Ok(props),
            }
        }
    }

    // '[' ... ']' 한 개. 백슬래시는 다음 한 글자를 이스케이프한다.
    fn parse_value(&mut self) -> Result<String, SgfError> {
        self.pos += 1; // '['
        let start = self.pos;
        let mut escaped = false;
        while let Some(c) = self.peek() {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b']' {
                let raw = &self.bytes[start..self.pos];
                self.pos += 1;
                return Ok(String::from_utf8_lossy(raw).into_owned());
            }
            self.pos += 1;
        }
        Err(SgfError::Syntax("']'가 없습니다".into()))
    }
}

fn first_value<'a>(props: &'a [Prop], ident: &str) -> Option<&'a str> {
    props
        .iter()
        .find(|(k, _)| k == ident)
        .and_then(|(_, v)| v.first())
        .map(|s| s.trim())
}

// "aa" 형식 좌표 -> (x, y). 빈 값이거나 "tt"(19줄 이하 보드의 관례)면 패스(None).
fn parse_point(value: &str, size: usize) -> Result<Option<(usize, usize)>, SgfError> {
    let v = value.trim();
    if v.is_empty() || (v == "tt" && size <= 19) {
        return Ok(None);
    }
    let bad = || SgfError::BadPoint(v.to_string());
    let b = v.as_bytes();
    if b.len() != 2 || !b.iter().all(u8::is_ascii_lowercase) {
        return Err(bad());
    }
    let (x, y) = ((b[0] - b'a') as usize, (b[1] - b'a') as usize);
    if x >= size || y >= size {
        return Err(bad());
    }
    Ok(Some((x, y)))
}

// AB/AW 값은 점 하나이거나 "aa:cc" 형태의 직사각형 범위일 수 있다.
fn expand_points(values: &[String], size: usize) -> Result<Vec<(usize, usize)>, SgfError> {
    let mut points = Vec::new();
    for value in values {
        match value.split_once(':') {
            Some((a, b)) => {
                let (Some((x1, y1)), Some((x2, y2))) =
                    (parse_point(a, size)?, parse_point(b, size)?)
                else {
                    return Err(SgfError::BadPoint(value.clone()));
                };
                for y in y1.min(y2)..=y1.max(y2) {
                    for x in x1.min(x2)..=x1.max(x2) {
                        points.push((x, y));
                    }
                }
            }
            None => {
                if let Some(p) = parse_point(value, size)? {
                    points.push(p);
                }
            }
        }
    }
    Ok(points)
}

fn parse_size(root: &[Prop]) -> Result<usize, SgfError> {
    let Some(raw) = first_value(root, "SZ") else {
        return Ok(19);
    };
    // "SZ[19]" 또는 직사각형 "SZ[19:19]" - 가로세로가 같은 경우만 허용.
    let (w, h) = raw.split_once(':').unwrap_or((raw, raw));
    let unsupported = || SgfError::UnsupportedSize(raw.to_string());
    let w: usize = w.trim().parse().map_err(|_| unsupported())?;
    let h: usize = h.trim().parse().map_err(|_| unsupported())?;
    if w != h || !(2..=MAX_BOARD_SIZE).contains(&w) {
        return Err(unsupported());
    }
    Ok(w)
}

fn color_of(letter: &str) -> Option<Color> {
    match letter {
        "B" => Some(Color::Black),
        "W" => Some(Color::White),
        _ => None,
    }
}

pub fn parse(content: &str) -> Result<SgfGame, SgfError> {
    if content.len() > MAX_SGF_BYTES {
        return Err(SgfError::TooLarge);
    }
    // UTF-8 BOM과 앞쪽 잡글자(일부 사이트가 붙이는 헤더)는 첫 "(;" 이전까지 버린다.
    let Some(start) = content.find("(;").or_else(|| content.find('(')) else {
        return Err(SgfError::NotSgf);
    };
    let mut parser = Parser {
        bytes: content.as_bytes(),
        pos: start,
    };
    let seq = parser.parse_tree(0)?;
    let Some(root) = seq.nodes.first() else {
        return Err(SgfError::NotSgf);
    };

    let size = parse_size(root)?;
    let komi = first_value(root, "KM").and_then(|v| v.parse::<f64>().ok());

    let mut setup_black = Vec::new();
    let mut setup_white = Vec::new();
    for (ident, values) in root {
        match ident.as_str() {
            "AB" => setup_black.extend(expand_points(values, size)?),
            "AW" => setup_white.extend(expand_points(values, size)?),
            _ => {}
        }
    }
    // 선 차례: PL이 있으면 그것, 없으면 치수 바둑(흑 돌만 놓여 있음)은 백이 먼저.
    let next_turn = match first_value(root, "PL").and_then(color_of) {
        Some(c) => Some(c),
        None if !setup_black.is_empty() && setup_white.is_empty() => Some(Color::White),
        None => None,
    };
    let mut tree =
        GameTree::with_setup(size, &setup_black, &setup_white, next_turn).ok_or_else(|| {
            SgfError::Unsupported("놓은 돌(AB/AW)이 판 밖이거나 서로 겹칩니다".into())
        })?;

    let mut move_number = 0;
    apply_sequence(&mut tree, &seq, true, size, &mut move_number)?;
    tree.finish_import();

    Ok(SgfGame { tree, komi })
}

// 노드 열을 트리에 이어 붙이고, 끝난 지점에서 각 변화도를 차례로 같은 분기점에서
// 이어 붙인다. 재귀 깊이는 변화도 중첩 깊이(MAX_VARIATION_DEPTH 이하)로만 늘어난다.
fn apply_sequence(
    tree: &mut GameTree,
    seq: &Sequence,
    is_root_sequence: bool,
    size: usize,
    move_number: &mut usize,
) -> Result<(), SgfError> {
    for (index, props) in seq.nodes.iter().enumerate() {
        let is_root = is_root_sequence && index == 0;
        if !is_root
            && props
                .iter()
                .any(|(k, _)| matches!(k.as_str(), "AB" | "AW" | "AE"))
        {
            return Err(SgfError::Unsupported(
                "대국 중간에 돌을 놓거나 지우는 기보(AB/AW/AE)는 지원하지 않습니다".into(),
            ));
        }

        let black = first_value(props, "B");
        let white = first_value(props, "W");
        let (color, value) = match (black, white) {
            (Some(_), Some(_)) => {
                return Err(SgfError::Syntax(
                    "한 노드에 흑과 백의 수가 함께 있습니다".into(),
                ))
            }
            (Some(v), None) => (Color::Black, v),
            (None, Some(v)) => (Color::White, v),
            (None, None) => continue, // 주석/마커만 있는 노드
        };

        *move_number += 1;
        match parse_point(value, size)? {
            Some((x, y)) => {
                if !tree.play_as(color, x, y) {
                    return Err(SgfError::IllegalMove(*move_number));
                }
            }
            None => tree.pass_as(color),
        }
    }

    let branch_point = tree.cursor();
    for variation in &seq.variations {
        // 수 번호는 변화도 사이에서도 이어서 센다 - 에러 메시지에는 "파일에 나온
        // 순서"만 알려주면 충분하다.
        apply_sequence(tree, variation, false, size, move_number)?;
        tree.set_cursor(branch_point);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(sgf: &str) -> SgfGame {
        parse(sgf).expect("SGF 파싱 실패")
    }

    #[test]
    fn parses_main_line_size_and_komi() {
        let game = load("(;GM[1]FF[4]SZ[9]KM[5.5]PB[a]PW[b];B[ee];W[ce];B[ec])");
        assert_eq!(game.komi, Some(5.5));
        let snap = game.tree.snapshot();
        assert_eq!(snap.size, 9);
        assert_eq!(game.tree.move_history().len(), 3);
        assert_eq!(snap.stones[4][4], Some(Color::Black)); // ee
        assert_eq!(snap.stones[4][2], Some(Color::White)); // ce
        assert_eq!(snap.current_turn, Color::White);
        assert!(!snap.can_go_forward); // 본 줄기 끝
    }

    #[test]
    fn defaults_to_19_and_no_komi() {
        let game = load("(;GM[1];B[pd])");
        assert_eq!(game.tree.snapshot().size, 19);
        assert_eq!(game.komi, None);
    }

    #[test]
    fn passes_via_empty_value_and_tt() {
        let game = load("(;SZ[9];B[ee];W[];B[tt])");
        let history = game.tree.move_history();
        assert_eq!(history.len(), 3);
        assert!(!history[0].is_pass);
        assert!(history[1].is_pass && history[1].color == Color::White);
        assert!(history[2].is_pass && history[2].color == Color::Black);
    }

    #[test]
    fn same_color_may_move_twice() {
        let game = load("(;SZ[9];B[aa];B[bb])");
        let history = game.tree.move_history();
        assert_eq!(history[1].color, Color::Black);
        assert_eq!(game.tree.snapshot().current_turn, Color::White);
    }

    #[test]
    fn captures_are_applied() {
        // 흑이 (0,0) 백 한 점을 (1,0)/(0,1)로 따냄
        let game = load("(;SZ[5];B[ba];W[aa];B[ab])");
        let snap = game.tree.snapshot();
        assert_eq!(snap.stones[0][0], None);
        assert_eq!(snap.captures.black, 1);
    }

    #[test]
    fn variations_become_branches_and_main_line_is_followed() {
        let mut game = load("(;SZ[9];B[ee](;W[ce];B[cc])(;W[gg]))");
        // 본 줄기: ee -> ce -> cc (3수), 끝에 위치
        assert_eq!(game.tree.move_history().len(), 3);

        // ee 시점으로 돌아가면 갈래가 2개
        game.tree.go_back();
        game.tree.go_back();
        assert_eq!(game.tree.snapshot().current_children.len(), 2);

        // 앞으로 가기는 본 줄기(첫 변화도)를 따라간다
        let mv = game.tree.go_forward().unwrap();
        assert_eq!((mv.x, mv.y), (2, 4)); // ce
    }

    #[test]
    fn handicap_stones_default_to_white_first() {
        let game = load("(;SZ[9]HA[2]AB[cc][gg];W[ee])");
        let snap = game.tree.snapshot();
        assert_eq!(snap.stones[2][2], Some(Color::Black));
        assert_eq!(snap.stones[6][6], Some(Color::Black));
        assert_eq!(game.tree.setup_stones().len(), 2);
        // 백이 먼저 뒀으므로 이제 흑 차례
        assert_eq!(snap.current_turn, Color::Black);
        assert_eq!(game.tree.move_history().len(), 1);
    }

    #[test]
    fn pl_overrides_default_turn_and_ranges_expand() {
        let game = load("(;SZ[9]AB[aa:bb]PL[B])");
        let snap = game.tree.snapshot();
        assert_eq!(game.tree.setup_stones().len(), 4);
        assert_eq!(snap.current_turn, Color::Black);
    }

    #[test]
    fn handles_escapes_comments_and_collections() {
        let game = load("junk\n(;SZ[9]C[a \\] b ; (not a tree)]GN[x];B[ee]C[hi])(;SZ[13];B[aa])");
        assert_eq!(game.tree.snapshot().size, 9); // 첫 대국만
        assert_eq!(game.tree.move_history().len(), 1);
    }

    #[test]
    fn tolerates_old_style_lowercase_identifiers_and_comment_only_nodes() {
        let game = load("(;GaMe[1]SZ[9];;B[ee];C[note])");
        assert_eq!(game.tree.move_history().len(), 1);
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(parse("hello").err(), Some(SgfError::NotSgf));
        assert!(matches!(parse("(;SZ[9];B[ee]"), Err(SgfError::Syntax(_))));
        assert!(matches!(
            parse("(;SZ[9];B[zz])"),
            Err(SgfError::BadPoint(_))
        ));
        assert!(matches!(
            parse("(;SZ[25];B[aa])"),
            Err(SgfError::UnsupportedSize(_))
        ));
        assert!(matches!(
            parse("(;SZ[9:5];B[aa])"),
            Err(SgfError::UnsupportedSize(_))
        ));
        assert_eq!(
            parse("(;SZ[9];B[ee];W[ee])").err(),
            Some(SgfError::IllegalMove(2))
        );
        assert!(matches!(
            parse("(;SZ[9];B[ee];AB[aa])"),
            Err(SgfError::Unsupported(_))
        ));
        assert!(matches!(
            parse("(;SZ[9];B[ee]W[aa])"),
            Err(SgfError::Syntax(_))
        ));
    }

    #[test]
    fn rejects_overly_deep_variation_nesting() {
        let sgf = format!("(;SZ[9]{}{}", "(;B[aa]".repeat(1000), ")".repeat(1000));
        assert!(matches!(parse(&sgf), Err(SgfError::Syntax(_))));
    }

    #[test]
    fn rejects_oversized_input() {
        let sgf = " ".repeat(MAX_SGF_BYTES + 1);
        assert_eq!(parse(&sgf).err(), Some(SgfError::TooLarge));
    }
}
