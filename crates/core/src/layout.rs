use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitDir {
    Columns,
    Rows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavDir {
    Left,
    Right,
    Up,
    Down,
}

impl NavDir {
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "left" | "h" => Some(Self::Left),
            "right" | "l" => Some(Self::Right),
            "up" | "k" => Some(Self::Up),
            "down" | "j" => Some(Self::Down),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LayoutNode {
    Leaf {
        pane: u64,
    },
    Split {
        dir: SplitDir,
        percent: u16,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

impl LayoutNode {
    pub fn leaf(pane: u64) -> Self {
        Self::Leaf { pane }
    }

    pub fn ids(&self) -> Vec<u64> {
        let mut ids = Vec::new();
        collect(self, &mut ids);
        ids
    }

    pub fn split_pane(&mut self, focused: u64, dir: SplitDir, new_id: u64) -> bool {
        match self {
            Self::Leaf { pane } if *pane == focused => {
                *self = Self::Split {
                    dir,
                    percent: 50,
                    first: Box::new(Self::Leaf { pane: focused }),
                    second: Box::new(Self::Leaf { pane: new_id }),
                };
                true
            }
            Self::Split { first, second, .. } => {
                first.split_pane(focused, dir, new_id) || second.split_pane(focused, dir, new_id)
            }
            Self::Leaf { .. } => false,
        }
    }

    pub fn remove_pane(&mut self, id: u64) -> bool {
        match remove(self.clone(), id) {
            Some(next) => {
                *self = next;
                true
            }
            None => false,
        }
    }

    pub fn first_leaf(&self) -> Option<u64> {
        match self {
            Self::Leaf { pane } => Some(*pane),
            Self::Split { first, .. } => first.first_leaf(),
        }
    }

    pub fn last_leaf(&self) -> Option<u64> {
        match self {
            Self::Leaf { pane } => Some(*pane),
            Self::Split { second, .. } => second.last_leaf(),
        }
    }

    pub fn swap_ids(&mut self, first: u64, second: u64) -> bool {
        if first == second {
            return false;
        }
        let ids = self.ids();
        if !ids.contains(&first) || !ids.contains(&second) {
            return false;
        }
        let sentinel = u64::MAX;
        self.replace_id(first, sentinel);
        self.replace_id(second, first);
        self.replace_id(sentinel, second)
    }

    pub fn replace_id(&mut self, old: u64, new: u64) -> bool {
        match self {
            Self::Leaf { pane } if *pane == old => {
                *pane = new;
                true
            }
            Self::Split { first, second, .. } => {
                first.replace_id(old, new) || second.replace_id(old, new)
            }
            Self::Leaf { .. } => false,
        }
    }

    pub fn neighbor(&self, focused: u64, dir: NavDir) -> Option<u64> {
        walk_neighbor(self, focused, dir)
    }

    pub fn set_percent(&mut self, first_leaf: u64, second_leaf: u64, percent: u16) -> bool {
        let percent = percent.clamp(15, 85);
        match self {
            Self::Split {
                first,
                second,
                percent: slot,
                ..
            } => {
                if first.first_leaf() == Some(first_leaf)
                    && second.first_leaf() == Some(second_leaf)
                {
                    *slot = percent;
                    true
                } else {
                    first.set_percent(first_leaf, second_leaf, percent)
                        || second.set_percent(first_leaf, second_leaf, percent)
                }
            }
            Self::Leaf { .. } => false,
        }
    }
}

fn walk_neighbor(node: &LayoutNode, focused: u64, dir: NavDir) -> Option<u64> {
    match node {
        LayoutNode::Leaf { .. } => None,
        LayoutNode::Split {
            dir: axis,
            first,
            second,
            ..
        } => {
            let in_first = first.ids().contains(&focused);
            let in_second = second.ids().contains(&focused);
            if !in_first && !in_second {
                return None;
            }
            let child = if in_first {
                first.as_ref()
            } else {
                second.as_ref()
            };
            if let Some(found) = walk_neighbor(child, focused, dir) {
                return Some(found);
            }
            match (*axis, dir, in_first) {
                (SplitDir::Columns, NavDir::Right, true) => second.first_leaf(),
                (SplitDir::Columns, NavDir::Left, false) => first.last_leaf(),
                (SplitDir::Rows, NavDir::Down, true) => second.first_leaf(),
                (SplitDir::Rows, NavDir::Up, false) => first.last_leaf(),
                _ => None,
            }
        }
    }
}

fn collect(node: &LayoutNode, out: &mut Vec<u64>) {
    match node {
        LayoutNode::Leaf { pane } => out.push(*pane),
        LayoutNode::Split { first, second, .. } => {
            collect(first, out);
            collect(second, out);
        }
    }
}

fn remove(node: LayoutNode, id: u64) -> Option<LayoutNode> {
    match node {
        LayoutNode::Leaf { pane } if pane == id => None,
        LayoutNode::Leaf { pane } => Some(LayoutNode::Leaf { pane }),
        LayoutNode::Split {
            dir,
            percent,
            first,
            second,
        } => match (remove(*first, id), remove(*second, id)) {
            (None, None) => None,
            (None, Some(kept)) | (Some(kept), None) => Some(kept),
            (Some(a), Some(b)) => Some(LayoutNode::Split {
                dir,
                percent,
                first: Box::new(a),
                second: Box::new(b),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_then_remove_restores_leaf() {
        let mut node = LayoutNode::leaf(1);
        assert!(node.split_pane(1, SplitDir::Columns, 2));
        assert_eq!(node.ids(), vec![1, 2]);
        assert!(node.remove_pane(2));
        assert_eq!(node.ids(), vec![1]);
    }

    #[test]
    fn set_percent_clamps_and_targets_nested_split() {
        let mut node = LayoutNode::leaf(1);
        assert!(node.split_pane(1, SplitDir::Columns, 2));
        assert!(node.split_pane(2, SplitDir::Rows, 3));
        assert!(node.set_percent(1, 2, 30));
        match &node {
            LayoutNode::Split { percent, first, .. } => {
                assert_eq!(*percent, 30);
                match first.as_ref() {
                    LayoutNode::Leaf { pane } => assert_eq!(*pane, 1),
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
        assert!(node.set_percent(2, 3, 10));
        match &node {
            LayoutNode::Split { second, .. } => match second.as_ref() {
                LayoutNode::Split { percent, .. } => assert_eq!(*percent, 15),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
        assert!(!node.set_percent(9, 9, 40));
    }

    #[test]
    fn neighbor_walks_columns_then_nested_rows() {
        let mut node = LayoutNode::leaf(1);
        assert!(node.split_pane(1, SplitDir::Columns, 2));
        assert!(node.split_pane(2, SplitDir::Rows, 3));
        assert_eq!(node.neighbor(1, NavDir::Right), Some(2));
        assert_eq!(node.neighbor(2, NavDir::Down), Some(3));
        assert_eq!(node.neighbor(3, NavDir::Up), Some(2));
        assert_eq!(node.neighbor(3, NavDir::Left), Some(1));
        assert_eq!(node.neighbor(1, NavDir::Left), None);
    }

    #[test]
    fn replace_id_updates_leaf() {
        let mut node = LayoutNode::leaf(1);
        assert!(node.split_pane(1, SplitDir::Columns, 2));
        assert!(node.replace_id(2, 9));
        assert_eq!(node.ids(), vec![1, 9]);
    }

    #[test]
    fn swap_ids_exchanges_leaves() {
        let mut node = LayoutNode::leaf(1);
        assert!(node.split_pane(1, SplitDir::Columns, 2));
        assert!(node.swap_ids(1, 2));
        assert_eq!(node.ids(), vec![2, 1]);
        assert!(!node.swap_ids(1, 1));
        assert!(!node.swap_ids(1, 9));
    }
}
