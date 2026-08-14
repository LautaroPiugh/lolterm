use std::collections::HashMap;

use color_eyre::Result;
use portable_pty::PtySize;
use ratatui::layout::{Constraint, Layout, Rect};

use serde::{Deserialize, Serialize};

use crate::pane::Pane;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitDir {
    Columns,
    Rows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusDir {
    Left,
    Right,
    Up,
    Down,
}

pub enum Node {
    Leaf(u64),
    Split {
        dir: SplitDir,
        percent: u16,
        first: Box<Node>,
        second: Box<Node>,
    },
}

pub struct PaneTree {
    pub root: Node,
    pub panes: HashMap<u64, Pane>,
    pub focused: u64,
}

impl PaneTree {
    pub fn leaf_ids(&self) -> Vec<u64> {
        let mut ids = Vec::new();
        collect_ids(&self.root, &mut ids);
        ids
    }

    pub fn from_parts(root: Node, panes: HashMap<u64, Pane>, focused: u64) -> Self {
        Self {
            root,
            panes,
            focused,
        }
    }

    pub fn new(pane: Pane) -> Self {
        let focused = pane.id;
        let mut panes = HashMap::new();
        panes.insert(focused, pane);
        Self {
            root: Node::Leaf(focused),
            panes,
            focused,
        }
    }

    pub fn grow_focused(&mut self, amount: i16) {
        grow_leaf(&mut self.root, self.focused, amount);
    }

    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn split_focused(&mut self, dir: SplitDir, pane: Pane) -> bool {
        let id = pane.id;
        let focused = self.focused;
        if insert_split(&mut self.root, focused, dir, id) {
            self.panes.insert(id, pane);
            self.focused = id;
            true
        } else {
            false
        }
    }

    pub fn close_focused(&mut self) -> bool {
        self.close_id(self.focused)
    }

    fn close_id(&mut self, id: u64) -> bool {
        self.panes.remove(&id);
        match remove_leaf(std::mem::replace(&mut self.root, Node::Leaf(0)), id) {
            Some(root) => {
                self.root = root;
                if self.focused == id {
                    self.focused = first_leaf(&self.root).unwrap_or(id);
                }
                true
            }
            None => {
                self.root = Node::Leaf(0);
                self.panes.clear();
                false
            }
        }
    }

    pub fn focus_next(&mut self) {
        let mut ids = Vec::new();
        collect_ids(&self.root, &mut ids);
        if ids.is_empty() {
            return;
        }
        let Some(pos) = ids.iter().position(|id| *id == self.focused) else {
            self.focused = ids[0];
            return;
        };
        self.focused = ids[(pos + 1) % ids.len()];
    }

    pub fn focus_dir(&mut self, dir: FocusDir, area: Rect) {
        let leaves = self.areas(area);
        let Some((_, current)) = leaves.iter().find(|(id, _)| *id == self.focused) else {
            return;
        };
        let current = *current;
        let mut best: Option<(u64, u32)> = None;

        for (id, rect) in &leaves {
            if *id == self.focused {
                continue;
            }
            if let Some(score) = neighbor_score(current, *rect, dir)
                && best.is_none_or(|(_, best_score)| score < best_score)
            {
                best = Some((*id, score));
            }
        }

        if let Some((id, _)) = best {
            self.focused = id;
        }
    }

    pub fn areas(&self, area: Rect) -> Vec<(u64, Rect)> {
        let mut out = Vec::new();
        collect_areas(&self.root, area, &mut out);
        out
    }

    pub fn reap(&mut self) -> Result<bool> {
        let mut dead = Vec::new();
        for (id, pane) in &mut self.panes {
            pane.shell.poll_exit()?;
            if pane.shell.child_exited() {
                dead.push(*id);
            }
        }

        for id in dead {
            if !self.close_id(id) {
                return Ok(false);
            }
        }

        Ok(!self.panes.is_empty())
    }

    pub fn sync_sizes(&self, area: Rect) -> Result<()> {
        for (id, rect) in self.areas(area) {
            if let Some(pane) = self.panes.get(&id) {
                pane.shell.resize(pty_size_from_rect(rect))?;
            }
        }
        Ok(())
    }

    pub fn focused_shell_mut(&mut self) -> Option<&mut crate::terminal::Shell> {
        self.panes
            .get_mut(&self.focused)
            .map(|pane| &mut pane.shell)
    }
}

fn insert_split(node: &mut Node, focused: u64, dir: SplitDir, new_id: u64) -> bool {
    match node {
        Node::Leaf(id) if *id == focused => {
            *node = Node::Split {
                dir,
                percent: 50,
                first: Box::new(Node::Leaf(focused)),
                second: Box::new(Node::Leaf(new_id)),
            };
            true
        }
        Node::Split { first, second, .. } => {
            insert_split(first, focused, dir, new_id) || insert_split(second, focused, dir, new_id)
        }
        Node::Leaf(_) => false,
    }
}

fn remove_leaf(node: Node, id: u64) -> Option<Node> {
    match node {
        Node::Leaf(leaf) if leaf == id => None,
        Node::Leaf(leaf) => Some(Node::Leaf(leaf)),
        Node::Split {
            dir,
            percent,
            first,
            second,
        } => match (remove_leaf(*first, id), remove_leaf(*second, id)) {
            (None, None) => None,
            (None, Some(kept)) | (Some(kept), None) => Some(kept),
            (Some(a), Some(b)) => Some(Node::Split {
                dir,
                percent,
                first: Box::new(a),
                second: Box::new(b),
            }),
        },
    }
}

fn first_leaf(node: &Node) -> Option<u64> {
    match node {
        Node::Leaf(id) => Some(*id),
        Node::Split { first, .. } => first_leaf(first),
    }
}

fn collect_ids(node: &Node, out: &mut Vec<u64>) {
    match node {
        Node::Leaf(id) => out.push(*id),
        Node::Split { first, second, .. } => {
            collect_ids(first, out);
            collect_ids(second, out);
        }
    }
}

fn collect_areas(node: &Node, area: Rect, out: &mut Vec<(u64, Rect)>) {
    match node {
        Node::Leaf(id) => out.push((*id, area)),
        Node::Split {
            dir,
            percent,
            first,
            second,
        } => {
            let (a, b) = split_rect(area, *dir, *percent);
            collect_areas(first, a, out);
            collect_areas(second, b, out);
        }
    }
}

fn split_rect(area: Rect, dir: SplitDir, percent: u16) -> (Rect, Rect) {
    let percent = percent.clamp(15, 85);
    let constraints = [
        Constraint::Percentage(percent),
        Constraint::Percentage(100 - percent),
    ];
    let chunks = match dir {
        SplitDir::Columns => Layout::horizontal(constraints).split(area),
        SplitDir::Rows => Layout::vertical(constraints).split(area),
    };
    (chunks[0], chunks[1])
}

fn grow_leaf(node: &mut Node, id: u64, amount: i16) -> bool {
    match node {
        Node::Split {
            first,
            second,
            percent,
            ..
        } => {
            if matches!(first.as_ref(), Node::Leaf(leaf) if *leaf == id) {
                *percent = (*percent as i16 + amount).clamp(15, 85) as u16;
                return true;
            }
            if matches!(second.as_ref(), Node::Leaf(leaf) if *leaf == id) {
                *percent = (*percent as i16 - amount).clamp(15, 85) as u16;
                return true;
            }
            grow_leaf(first, id, amount) || grow_leaf(second, id, amount)
        }
        Node::Leaf(_) => false,
    }
}

fn neighbor_score(current: Rect, other: Rect, dir: FocusDir) -> Option<u32> {
    let (primary, aligned) = match dir {
        FocusDir::Left => {
            if other.x >= current.x {
                return None;
            }
            (
                current.x.saturating_sub(other.right()) as u32,
                overlap(current.y, current.height, other.y, other.height),
            )
        }
        FocusDir::Right => {
            if other.right() <= current.x {
                return None;
            }
            (
                other.x.saturating_sub(current.right()) as u32,
                overlap(current.y, current.height, other.y, other.height),
            )
        }
        FocusDir::Up => {
            if other.y >= current.y {
                return None;
            }
            (
                current.y.saturating_sub(other.bottom()) as u32,
                overlap(current.x, current.width, other.x, other.width),
            )
        }
        FocusDir::Down => {
            if other.bottom() <= current.y {
                return None;
            }
            (
                other.y.saturating_sub(current.bottom()) as u32,
                overlap(current.x, current.width, other.x, other.width),
            )
        }
    };

    Some(primary.saturating_mul(20) + (1000u32.saturating_sub(u32::from(aligned))))
}

fn overlap(a: u16, a_len: u16, b: u16, b_len: u16) -> u16 {
    let start = a.max(b);
    let end = (a + a_len).min(b + b_len);
    end.saturating_sub(start)
}

pub fn pty_size_from_rect(area: Rect) -> PtySize {
    PtySize {
        rows: area.height.saturating_sub(2).max(1),
        cols: area.width.saturating_sub(2).max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}
