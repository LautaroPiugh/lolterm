use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitDir {
    Columns,
    Rows,
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
}
