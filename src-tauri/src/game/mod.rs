// 바둑판 상태 + 게임 트리(수순 기록) + 따내기/활로 판정 로직.
// 이 모듈이 규칙의 단일 진실 공급원(source of truth)이며, 프런트엔드는 매 동작 후
// 스냅샷(BoardSnapshot)을 받아 그리기만 함 - 착수/따내기 판정은 여기서만 이뤄짐.

use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    Black,
    White,
}

impl Color {
    fn opponent(self) -> Color {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }

    /// GTP 프로토콜에서 쓰는 색 표기("B"/"W") - play/genmove 명령 조립용.
    pub fn gtp_letter(self) -> &'static str {
        match self {
            Color::Black => "B",
            Color::White => "W",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MoveInfo {
    pub x: usize,
    pub y: usize,
    pub color: Color,
    // true면 이 노드는 실제 착수가 아니라 pass. 이 경우 x/y는 의미 없는 값(0, 0)이고
    // 보드 위 마커/좌표 계산에 절대 쓰이면 안 되므로, snapshot()의 current_children/
    // last_move는 이 플래그를 보고 pass 노드를 걸러낸다.
    pub is_pass: bool,
}

type Stones = Vec<Vec<Option<Color>>>;

// 각 진영이 그 노드까지(자기 자신 포함) 누적으로 잡은 상대 돌 수. Node에 매 수마다
// 갱신해서 들고 있으므로(부모 값 + 이번 수로 새로 잡은 만큼) 스냅샷을 만들 때 트리를
// 거슬러 올라가며 다시 셀 필요 없이 바로 읽을 수 있음.
#[derive(Debug, Clone, Copy, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Captures {
    pub black: u32,
    pub white: u32,
}

struct Node {
    parent: Option<usize>,
    children: Vec<usize>,
    // 이 노드에서 가장 최근에 이동해 들어갔던 자식(go_forward가 "[" "]" 단축키로
    // 자식 쪽으로 이동할 때 어느 가지를 고를지 결정하는 데 씀). confirm_move/
    // pass_turn으로 새 수를 두거나 기존 가지로 들어갈 때, 그리고 go_forward 자체가
    // 실행될 때 갱신되고, go_back(자식->부모)으로는 바뀌지 않는다 - 그래야
    // "뒤로 갔다가 다시 앞으로" 했을 때 원래 보던 가지로 정확히 되돌아간다.
    last_child: Option<usize>,
    mv: Option<MoveInfo>,
    stones: Stones,
    next_turn: Color,
    captures: Captures,
}

// 프런트엔드로 보내는 매 동작 후 보드 상태 스냅샷. 프런트엔드는 이 안의 필드만 보고
// 그리며, stones 안에 이미 따낸 돌까지 전부 반영되어 있음.
#[derive(Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardSnapshot {
    pub size: usize,
    pub stones: Stones,
    pub current_turn: Color,
    // 현재 노드로 이어진 수(마지막 착수 표시용). 루트거나 그 수가 pass였으면 없음
    // (pass는 좌표가 없어 보드 위에 표시할 자리가 없으므로).
    pub last_move: Option<Point>,
    // 현재 노드로 이어진 수가 pass였는지 - true면 위 last_move는 항상 None. 보드가
    // "방금 상대가 pass했다"는 걸 표시하는 데 사용(BoardCanvas의 PASS 안내 문구).
    pub last_move_is_pass: bool,
    // 현재 노드에서 갈라지는 다음 수 후보들(게임 트리 자식) - 보드 위 마커 표시용.
    // pass로 갈라지는 자식은 좌표가 없으므로 여기서 제외됨(snapshot() 참고).
    pub current_children: Vec<MoveInfo>,
    pub can_go_back: bool,
    // 현재 노드에 자식이 하나 이상 있는지("]"/앞으로 가기 버튼 활성화 여부).
    pub can_go_forward: bool,
    // 게임 트리 arena 안에서 현재 노드의 고유 인덱스. 노드는 절대 재사용되지 않으므로
    // (remove_last_move도 arena에서 실제로 지우지 않고 부모의 children 목록에서만
    // 떼어냄) 프런트엔드가 "이 노드의 kata-analyze 결과"를 캐싱하는 안정적인 키로
    // 쓸 수 있음 - analysisStore가 노드별로 분석 결과를 보관하는 데 사용.
    pub node_id: usize,
    // 현재 노드에서 루트까지 이어지는 조상 체인(자기 자신부터 시작, 루트로 갈수록
    // 뒤쪽). winrate bar가 "이 노드 자체는 아직 분석된 적이 없어도 가장 가까운
    // 조상의(자기 자신 포함) 분석 결과"를 대신 보여줄 수 있도록 프런트에 노출.
    pub ancestor_chain: Vec<usize>,
    // 현재 노드까지 누적으로 흑/백이 각각 잡은 상대 돌 수(포로 수). 뒤로 가기/마지막
    // 수 제거 시에도 그 노드에 저장된 값을 그대로 읽으므로 항상 "지금 보드에 반영된
    // 상태" 기준으로 정확함.
    pub captures: Captures,
}

fn empty_board(size: usize) -> Stones {
    vec![vec![None; size]; size]
}

fn neighbors(x: usize, y: usize, size: usize) -> Vec<(usize, usize)> {
    let mut result = Vec::with_capacity(4);
    if x > 0 {
        result.push((x - 1, y));
    }
    if x + 1 < size {
        result.push((x + 1, y));
    }
    if y > 0 {
        result.push((x, y - 1));
    }
    if y + 1 < size {
        result.push((x, y + 1));
    }
    result
}

// (x, y)의 돌과 같은 색으로 연결된 그룹 전체와, 그 그룹의 활로(인접한 빈 칸) 개수를 구함
fn collect_group(stones: &Stones, x: usize, y: usize, size: usize) -> (Vec<(usize, usize)>, usize) {
    let color = stones[y][x];
    let mut visited = HashSet::new();
    visited.insert((x, y));
    let mut liberties = HashSet::new();
    let mut group = Vec::new();
    let mut stack = vec![(x, y)];

    while let Some((cx, cy)) = stack.pop() {
        group.push((cx, cy));
        for (nx, ny) in neighbors(cx, cy, size) {
            let n_stone = stones[ny][nx];
            if n_stone.is_none() {
                liberties.insert((nx, ny));
            } else if n_stone == color && !visited.contains(&(nx, ny)) {
                visited.insert((nx, ny));
                stack.push((nx, ny));
            }
        }
    }

    (group, liberties.len())
}

// 방금 (x, y)에 color 돌을 둔 직후 호출: 사방으로 맞닿은 상대 돌 그룹 중 활로가 0인
// (완전히 둘러싸인) 그룹을 찾아 판에서 제거(따냄). 상대를 따낸 뒤에도 자신이 방금 둔
// 돌의 그룹이 활로 0이면(자충수) 그 그룹도 함께 제거. 반환값은 이번 수로 판에서 실제로
// 제거된 "상대" 돌 개수(포로 수 누적에 씀) - 자충수로 제거된 자기 돌은 상대가 잡은
// 것이 아니므로 포함하지 않음.
fn apply_captures(stones: &mut Stones, x: usize, y: usize, color: Color, size: usize) -> usize {
    let opponent = color.opponent();
    let mut checked = HashSet::new();
    let mut captured = 0;

    for (nx, ny) in neighbors(x, y, size) {
        if stones[ny][nx] != Some(opponent) {
            continue;
        }
        if checked.contains(&(nx, ny)) {
            continue;
        }

        let (group, liberties) = collect_group(stones, nx, ny, size);
        for &c in &group {
            checked.insert(c);
        }

        if liberties == 0 {
            captured += group.len();
            for (gx, gy) in group {
                stones[gy][gx] = None;
            }
        }
    }

    let (own_group, own_liberties) = collect_group(stones, x, y, size);
    if own_liberties == 0 {
        for (gx, gy) in own_group {
            stones[gy][gx] = None;
        }
    }

    captured
}

// 게임 트리: 각 노드는 그 수를 둔(따내기까지 반영된) 직후의 보드 스냅샷을 통째로
// 들고 있고, 뒤로 가기는 그 스냅샷을 그대로 복원하는 방식. 노드는 arena(Vec) 안에
// 보관하고 부모/자식은 인덱스로 참조.
pub struct GameTree {
    size: usize,
    nodes: Vec<Node>,
    current: usize,
}

impl GameTree {
    pub fn new(size: usize) -> Self {
        let root = Node {
            parent: None,
            children: Vec::new(),
            last_child: None,
            mv: None,
            stones: empty_board(size),
            next_turn: Color::Black,
            captures: Captures::default(),
        };
        GameTree {
            size,
            nodes: vec![root],
            current: 0,
        }
    }

    pub fn snapshot(&self) -> BoardSnapshot {
        let node = &self.nodes[self.current];
        let current_children = node
            .children
            .iter()
            .filter_map(|&id| self.nodes[id].mv)
            .filter(|m| !m.is_pass)
            .collect();
        BoardSnapshot {
            size: self.size,
            stones: node.stones.clone(),
            current_turn: node.next_turn,
            last_move: node.mv.filter(|m| !m.is_pass).map(|m| Point { x: m.x, y: m.y }),
            last_move_is_pass: node.mv.map(|m| m.is_pass).unwrap_or(false),
            current_children,
            can_go_back: node.parent.is_some(),
            can_go_forward: !node.children.is_empty(),
            node_id: self.current,
            ancestor_chain: self.ancestor_chain(),
            captures: node.captures,
        }
    }

    // 현재 노드부터 루트까지 부모 포인터를 따라간 노드 id 목록(자기 자신 포함, 가까운
    // 순서). 트리 깊이만큼만 순회하므로(보드 한 판 = 최대 수백 수) 매 snapshot마다
    // 계산해도 비용이 무시할 만함.
    fn ancestor_chain(&self) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut cur = Some(self.current);
        while let Some(id) = cur {
            chain.push(id);
            cur = self.nodes[id].parent;
        }
        chain
    }

    // 빈 칸을 확정하면 착수 -> 게임 트리에 새 노드를 추가(같은 자리·같은 색의 자식이
    // 이미 있으면 그 가지를 재사용). 이미 돌이 있는 칸은 착수할 수 없으므로 아무 동작도
    // 하지 않음. 반환값은 실제로 착수(또는 기존 가지로 이동)했는지 여부 - 호출자가 이
    // 값으로 GTP 엔진에 미러링할지 여부를 판단함(거부된 경우 엔진에 보내면 안 됨).
    pub fn confirm_move(&mut self, x: usize, y: usize) -> bool {
        if x >= self.size || y >= self.size {
            return false;
        }
        let node = &self.nodes[self.current];
        if node.stones[y][x].is_some() {
            return false;
        }
        let color = node.next_turn;

        let existing = node.children.iter().copied().find(
            |&id| matches!(self.nodes[id].mv, Some(m) if !m.is_pass && m.x == x && m.y == y && m.color == color),
        );

        if let Some(id) = existing {
            self.nodes[self.current].last_child = Some(id);
            self.current = id;
            return true;
        }

        let mut next_stones = self.nodes[self.current].stones.clone();
        next_stones[y][x] = Some(color);
        let captured = apply_captures(&mut next_stones, x, y, color, self.size);
        let next_turn = color.opponent();

        let mut captures = self.nodes[self.current].captures;
        match color {
            Color::Black => captures.black += captured as u32,
            Color::White => captures.white += captured as u32,
        }

        let child = Node {
            parent: Some(self.current),
            children: Vec::new(),
            last_child: None,
            mv: Some(MoveInfo { x, y, color, is_pass: false }),
            stones: next_stones,
            next_turn,
            captures,
        };
        let child_id = self.nodes.len();
        self.nodes.push(child);
        self.nodes[self.current].children.push(child_id);
        self.nodes[self.current].last_child = Some(child_id);
        self.current = child_id;
        true
    }

    // 착수 없이 차례만 넘김(pass) -> 게임 트리에 새 노드를 추가(같은 색이 이미 pass한
    // 자식이 있으면 confirm_move와 마찬가지로 그 가지를 재사용). 보드 상태(stones)는
    // 그대로 부모에서 물려받고 누적 포로 수도 변하지 않음 - 바뀌는 건 다음 차례뿐.
    // confirm_move와 달리 실패할 경우가 없으므로(칸을 고를 필요가 없어 항상 가능)
    // 반환값 없이 바로 트리를 갱신한다.
    pub fn pass_turn(&mut self) {
        let node = &self.nodes[self.current];
        let color = node.next_turn;

        let existing = node.children.iter().copied().find(
            |&id| matches!(self.nodes[id].mv, Some(m) if m.is_pass && m.color == color),
        );
        if let Some(id) = existing {
            self.nodes[self.current].last_child = Some(id);
            self.current = id;
            return;
        }

        let stones = self.nodes[self.current].stones.clone();
        let captures = self.nodes[self.current].captures;
        let next_turn = color.opponent();

        let child = Node {
            parent: Some(self.current),
            children: Vec::new(),
            last_child: None,
            mv: Some(MoveInfo { x: 0, y: 0, color, is_pass: true }),
            stones,
            next_turn,
            captures,
        };
        let child_id = self.nodes.len();
        self.nodes.push(child);
        self.nodes[self.current].children.push(child_id);
        self.nodes[self.current].last_child = Some(child_id);
        self.current = child_id;
    }

    // 게임 트리에서 부모 노드로 이동(뒤로 가기). 노드 자체는 지우지 않으므로 나중에
    // 같은 수를 두면 그 가지로 다시 들어감. 이미 루트라면 아무 동작도 하지 않음.
    pub fn go_back(&mut self) {
        if let Some(parent) = self.nodes[self.current].parent {
            self.current = parent;
        }
    }

    // 게임 트리에서 자식 노드로 이동(앞으로 가기). 이 노드에서 가장 최근에 갔었던
    // 자식(last_child)이 있으면 그쪽으로, 한 번도 자식으로 가본 적이 없으면(예:
    // 막 새로 갈라진 지점) 가장 최근에 만들어진 자식으로 이동한다. 자식이 하나도
    // 없으면(리프 노드) 아무 동작도 하지 않는다. 반환값은 실제로 이동했다면 그
    // 전이를 만든 수(색/좌표/pass 여부) - 호출자가 이 값으로 GTP 엔진에도 같은 수를
    // `play`로 재생해 로컬과 엔진 보드를 다시 맞출 수 있음.
    pub fn go_forward(&mut self) -> Option<MoveInfo> {
        let node = &self.nodes[self.current];
        let target = node
            .last_child
            .filter(|id| node.children.contains(id))
            .or_else(|| node.children.last().copied())?;

        self.nodes[self.current].last_child = Some(target);
        self.current = target;
        self.nodes[target].mv
    }

    // 현재 노드(=가장 마지막으로 둔 수) 자체를 게임 트리에서 통째로 삭제하고 그 부모로
    // 이동. goBack과 달리 그 수와 그 아래로 이어지는 모든 하위 가지(자식 노드들)가
    // (부모의 children 목록에서 제거되어) 더 이상 도달 불가능해짐. 루트에서는 아무
    // 동작도 하지 않음.
    pub fn remove_last_move(&mut self) {
        let Some(parent) = self.nodes[self.current].parent else {
            return;
        };
        let removed = self.current;
        self.nodes[parent].children.retain(|&id| id != removed);
        // last_child가 지금 지운 노드를 가리키고 있었다면 그대로 두면 go_forward가
        // 더 이상 부모의 children 목록에 없는(도달 불가능해진) 노드를 가리키는
        // 매달린 참조가 된다 - go_forward가 children.contains로 걸러내긴 하지만,
        // 굳이 무효한 값을 남겨둘 이유가 없으므로 여기서 바로 비워둠.
        if self.nodes[parent].last_child == Some(removed) {
            self.nodes[parent].last_child = None;
        }
        self.current = parent;
    }

    // 돌을 실제로 두지 않고도 다음에 둘 색을 수동으로 바꿈 (예: 상대 대신 두는 경우 등)
    pub fn toggle_turn(&mut self) {
        let node = &mut self.nodes[self.current];
        node.next_turn = node.next_turn.opponent();
    }

    /// 보드 한 변의 칸 수(19/13/9 등). GTP vertex 변환(좌표계 뒤집기)에 필요.
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Default for GameTree {
    fn default() -> Self {
        GameTree::new(19)
    }
}

// 규칙 엔진(따내기/자충수/게임 트리 탐색)에 대한 회귀 테스트. 5x5처럼 작은 보드를
// 써서 좌표를 손으로 계산하기 쉽게 함(실제 앱은 19/13/9만 쓰지만 규칙 로직 자체는
// 크기에 무관).
#[cfg(test)]
mod tests {
    use super::*;

    // ---------- 따내기(capture) ----------

    #[test]
    fn captures_single_surrounded_stone() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(2, 2)); // B
        assert!(tree.confirm_move(1, 2)); // W
        assert!(tree.confirm_move(0, 0)); // B dummy
        assert!(tree.confirm_move(3, 2)); // W
        assert!(tree.confirm_move(0, 1)); // B dummy
        assert!(tree.confirm_move(2, 1)); // W
        assert!(tree.confirm_move(0, 2)); // B dummy
        assert!(tree.confirm_move(2, 3)); // W - (2,2)의 마지막 활로를 메워 따냄

        let snap = tree.snapshot();
        assert_eq!(snap.stones[2][2], None);
        assert_eq!(snap.captures.white, 1);
        assert_eq!(snap.captures.black, 0);
    }

    #[test]
    fn captures_whole_connected_group() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(0, 0)); // B group 1번째 돌
        assert!(tree.confirm_move(4, 4)); // W dummy
        assert!(tree.confirm_move(1, 0)); // B group 2번째 돌 (0,0)과 연결
        assert!(tree.confirm_move(0, 1)); // W 포위 1
        assert!(tree.confirm_move(4, 3)); // B dummy
        assert!(tree.confirm_move(2, 0)); // W 포위 2
        assert!(tree.confirm_move(4, 2)); // B dummy
        assert!(tree.confirm_move(1, 1)); // W 포위 3 - group 전체 따냄

        let snap = tree.snapshot();
        assert_eq!(snap.stones[0][0], None);
        assert_eq!(snap.stones[0][1], None);
        assert_eq!(snap.captures.white, 2);
    }

    #[test]
    fn suicide_removes_own_stone_without_crediting_a_capture() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(4, 4)); // B dummy
        assert!(tree.confirm_move(1, 0)); // W
        assert!(tree.confirm_move(4, 3)); // B dummy
        assert!(tree.confirm_move(0, 1)); // W
        // (0,0) 모서리는 이웃이 (1,0)/(0,1) 둘뿐이라 둘 다 White면 Black이 두는 순간
        // 활로 0 - 아무것도 따내지 못한 채 자기 돌만 즉시 제거되는 자충수.
        assert!(tree.confirm_move(0, 0)); // B - 자충수

        let snap = tree.snapshot();
        assert_eq!(snap.stones[0][0], None);
        assert_eq!(snap.stones[0][1], Some(Color::White));
        assert_eq!(snap.stones[1][0], Some(Color::White));
        assert_eq!(snap.captures.black, 0);
        assert_eq!(snap.captures.white, 0);
    }

    #[test]
    fn capture_takes_priority_over_suicide() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(2, 0)); // B
        assert!(tree.confirm_move(1, 0)); // W - 활로가 결국 (0,0) 하나만 남을 group
        assert!(tree.confirm_move(1, 1)); // B
        assert!(tree.confirm_move(0, 1)); // W - 이쪽도 활로가 (0,0) 하나만 남을 group
        assert!(tree.confirm_move(0, 2)); // B
        assert!(tree.confirm_move(4, 4)); // W dummy (차례 맞추기용)
        // (0,0)에 Black을 두면 얼핏 자충수처럼 보이지만, White 두 그룹을 먼저
        // 따내면서 활로가 생기므로 실제로는 살아남는다 - apply_captures가 상대
        // 그룹부터 제거한 뒤에 자기 그룹의 활로를 판정하기 때문.
        assert!(tree.confirm_move(0, 0)); // B

        let snap = tree.snapshot();
        assert_eq!(snap.stones[0][0], Some(Color::Black));
        assert_eq!(snap.stones[0][1], None);
        assert_eq!(snap.stones[1][0], None);
        assert_eq!(snap.captures.black, 2);
    }

    #[test]
    fn confirm_move_rejects_occupied_point() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(0, 0));
        let node_count_before = tree.nodes.len();
        assert!(!tree.confirm_move(0, 0));
        assert_eq!(tree.nodes.len(), node_count_before);
        assert_eq!(tree.snapshot().node_id, 1);
    }

    // ---------- 게임 트리 탐색 ----------

    #[test]
    fn confirm_move_reuses_existing_branch() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(2, 2));
        let first_id = tree.snapshot().node_id;
        tree.go_back();
        assert!(tree.confirm_move(2, 2));
        assert_eq!(tree.snapshot().node_id, first_id);
        assert_eq!(tree.nodes.len(), 2); // 새 노드가 생기지 않고 기존 가지를 재사용해야 함
    }

    #[test]
    fn pass_turn_reuses_existing_branch() {
        let mut tree = GameTree::new(5);
        tree.pass_turn();
        let first_id = tree.snapshot().node_id;
        tree.go_back();
        tree.pass_turn();
        assert_eq!(tree.snapshot().node_id, first_id);
        assert_eq!(tree.nodes.len(), 2);
    }

    #[test]
    fn go_back_from_root_is_noop() {
        let mut tree = GameTree::new(5);
        tree.go_back();
        assert_eq!(tree.snapshot().node_id, 0);
        assert!(!tree.snapshot().can_go_back);
    }

    #[test]
    fn go_forward_from_leaf_returns_none() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(0, 0));
        assert_eq!(tree.go_forward(), None);
        assert_eq!(tree.snapshot().node_id, 1);
    }

    #[test]
    fn go_forward_prefers_last_visited_child() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(0, 0)); // 가지 A 생성
        tree.go_back();
        assert!(tree.confirm_move(4, 4)); // 가지 B 생성 (root.last_child = B)
        tree.go_back();

        let mv = tree.go_forward();
        assert_eq!(
            mv,
            Some(MoveInfo { x: 4, y: 4, color: Color::Black, is_pass: false })
        );
        assert_eq!(tree.snapshot().node_id, 2); // 먼저 만들어진 A(id1)가 아니라 마지막에 방문한 B(id2)
    }

    #[test]
    fn remove_last_move_clears_dangling_last_child() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(0, 0)); // 가지 A(id1)
        tree.go_back();
        assert!(tree.confirm_move(4, 4)); // 가지 B(id2), root.last_child = B
        tree.remove_last_move(); // B를 지움 - root.last_child가 B를 가리키다 None으로 정리돼야 함

        let mv = tree.go_forward();
        assert_eq!(
            mv,
            Some(MoveInfo { x: 0, y: 0, color: Color::Black, is_pass: false })
        );
        assert_eq!(tree.snapshot().node_id, 1); // 남은 유일한 자식 A로 이동
    }

    #[test]
    fn remove_last_move_from_root_is_noop() {
        let mut tree = GameTree::new(5);
        tree.remove_last_move();
        assert_eq!(tree.snapshot().node_id, 0);
        assert!(!tree.snapshot().can_go_back);
    }

    #[test]
    fn ancestor_chain_is_self_first_root_last() {
        let mut tree = GameTree::new(5);
        assert!(tree.confirm_move(0, 0));
        assert!(tree.confirm_move(1, 1));
        assert!(tree.confirm_move(2, 2));
        assert_eq!(tree.snapshot().ancestor_chain, vec![3, 2, 1, 0]);
    }

    // ---------- 알려진 규칙 격차(의도적으로 보존, 고치지 않음) ----------

    // 패(ko)/positional superko 판정은 아직 구현되어 있지 않음 - 이 테스트는 그
    // 사실을 고치는 게 아니라 현재 동작으로 고정해 문서화하는 용도. 나중에 ko 규칙을
    // 추가한다면 confirm_move 안, 기존 가지 재사용 판정 이후·실제 stones 변경 이전이
    // 그 판정을 넣을 위치가 됨.
    #[test]
    fn allows_immediate_recapture_no_ko_rule_yet() {
        let mut tree = GameTree::new(5);
        // P=(2,2)를 White가 가둬 따내고, 그 자리에 둔 White 돌을 Black이 바로
        // 되따내 정확히 같은 판 모양으로 되돌아오는 "패" 모양을 만든다.
        assert!(tree.confirm_move(2, 2)); // B: P
        assert!(tree.confirm_move(3, 2)); // W: P의 동쪽
        assert!(tree.confirm_move(0, 2)); // B: Q(1,2)의 서쪽
        assert!(tree.confirm_move(2, 1)); // W: P의 북쪽
        assert!(tree.confirm_move(1, 1)); // B: Q의 북쪽
        assert!(tree.confirm_move(2, 3)); // W: P의 남쪽
        assert!(tree.confirm_move(1, 3)); // B: Q의 남쪽

        let before = tree.snapshot();

        assert!(tree.confirm_move(1, 2)); // W: Q - P의 마지막 활로를 메워 따냄
        assert_eq!(tree.snapshot().stones[2][2], None);

        assert!(tree.confirm_move(2, 2)); // B: 즉시 되따냄 - ko 규칙이 없으므로 허용됨
        let after = tree.snapshot();

        assert_eq!(after.stones, before.stones); // 판이 정확히 같은 모양으로 되돌아옴
    }
}
