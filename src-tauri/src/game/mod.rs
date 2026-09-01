// 바둑판 상태 + 게임 트리(수순 기록) + 따내기/활로 판정 로직.
// 이 모듈이 규칙의 단일 진실 공급원(source of truth)이며, 프런트엔드는 매 동작 후
// 스냅샷(BoardSnapshot)을 받아 그리기만 함 - 착수/따내기 판정은 여기서만 이뤄짐.

use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MoveInfo {
    pub x: usize,
    pub y: usize,
    pub color: Color,
}

type Stones = Vec<Vec<Option<Color>>>;

struct Node {
    parent: Option<usize>,
    children: Vec<usize>,
    mv: Option<MoveInfo>,
    stones: Stones,
    next_turn: Color,
}

// 프런트엔드로 보내는 매 동작 후 보드 상태 스냅샷. 프런트엔드는 이 안의 필드만 보고
// 그리며, stones 안에 이미 따낸 돌까지 전부 반영되어 있음.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSnapshot {
    pub size: usize,
    pub stones: Stones,
    pub current_turn: Color,
    // 현재 노드로 이어진 수(마지막 착수 표시용). 루트면 없음.
    pub last_move: Option<Point>,
    // 현재 노드에서 갈라지는 다음 수 후보들(게임 트리 자식) - 보드 위 마커 표시용.
    pub current_children: Vec<MoveInfo>,
    pub can_go_back: bool,
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
fn collect_group(
    stones: &Stones,
    x: usize,
    y: usize,
    size: usize,
) -> (Vec<(usize, usize)>, usize) {
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
// 돌의 그룹이 활로 0이면(자충수) 그 그룹도 함께 제거.
fn apply_captures(stones: &mut Stones, x: usize, y: usize, color: Color, size: usize) {
    let opponent = color.opponent();
    let mut checked = HashSet::new();

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
            mv: None,
            stones: empty_board(size),
            next_turn: Color::Black,
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
            .collect();
        BoardSnapshot {
            size: self.size,
            stones: node.stones.clone(),
            current_turn: node.next_turn,
            last_move: node.mv.map(|m| Point { x: m.x, y: m.y }),
            current_children,
            can_go_back: node.parent.is_some(),
        }
    }

    // 빈 칸을 확정하면 착수 -> 게임 트리에 새 노드를 추가(같은 자리·같은 색의 자식이
    // 이미 있으면 그 가지를 재사용). 이미 돌이 있는 칸은 착수할 수 없으므로 아무 동작도
    // 하지 않음.
    pub fn confirm_move(&mut self, x: usize, y: usize) {
        if x >= self.size || y >= self.size {
            return;
        }
        let node = &self.nodes[self.current];
        if node.stones[y][x].is_some() {
            return;
        }
        let color = node.next_turn;

        let existing = node.children.iter().copied().find(|&id| {
            matches!(self.nodes[id].mv, Some(m) if m.x == x && m.y == y && m.color == color)
        });

        if let Some(id) = existing {
            self.current = id;
            return;
        }

        let mut next_stones = self.nodes[self.current].stones.clone();
        next_stones[y][x] = Some(color);
        apply_captures(&mut next_stones, x, y, color, self.size);
        let next_turn = color.opponent();

        let child = Node {
            parent: Some(self.current),
            children: Vec::new(),
            mv: Some(MoveInfo { x, y, color }),
            stones: next_stones,
            next_turn,
        };
        let child_id = self.nodes.len();
        self.nodes.push(child);
        self.nodes[self.current].children.push(child_id);
        self.current = child_id;
    }

    // 게임 트리에서 부모 노드로 이동(뒤로 가기). 노드 자체는 지우지 않으므로 나중에
    // 같은 수를 두면 그 가지로 다시 들어감. 이미 루트라면 아무 동작도 하지 않음.
    pub fn go_back(&mut self) {
        if let Some(parent) = self.nodes[self.current].parent {
            self.current = parent;
        }
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
        self.current = parent;
    }

    // 돌을 실제로 두지 않고도 다음에 둘 색을 수동으로 바꿈 (예: 상대 대신 두는 경우 등)
    pub fn toggle_turn(&mut self) {
        let node = &mut self.nodes[self.current];
        node.next_turn = node.next_turn.opponent();
    }
}

impl Default for GameTree {
    fn default() -> Self {
        GameTree::new(19)
    }
}
