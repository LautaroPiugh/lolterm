use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::Result;
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};

use crate::tree::{Node, SplitDir};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub active_workspace: usize,
    pub workspaces: Vec<SavedWorkspace>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedWorkspace {
    pub name: String,
    pub root: PathBuf,
    pub active_tab: usize,
    pub tabs: Vec<SavedTab>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedTab {
    pub focused: usize,
    #[serde(default)]
    pub name: Option<String>,
    pub tree: SavedNode,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SavedNode {
    Leaf {
        #[serde(default, skip_serializing)]
        #[allow(dead_code)]
        cwd: Option<PathBuf>,
    },
    Split {
        dir: SplitDir,
        percent: u16,
        first: Box<SavedNode>,
        second: Box<SavedNode>,
    },
}

impl SavedNode {
    pub fn from_live(node: &Node, keep: &HashSet<u64>) -> Option<Self> {
        match node {
            Node::Leaf(id) if keep.contains(id) => Some(Self::Leaf { cwd: None }),
            Node::Leaf(_) => None,
            Node::Split {
                dir,
                percent,
                first,
                second,
            } => match (Self::from_live(first, keep), Self::from_live(second, keep)) {
                (None, None) => None,
                (Some(kept), None) | (None, Some(kept)) => Some(kept),
                (Some(a), Some(b)) => Some(Self::Split {
                    dir: *dir,
                    percent: *percent,
                    first: Box::new(a),
                    second: Box::new(b),
                }),
            },
        }
    }

    pub fn leaf_count(&self) -> usize {
        match self {
            Self::Leaf { .. } => 1,
            Self::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }

    pub fn to_live(&self, ids: &mut impl Iterator<Item = u64>) -> Result<Node> {
        match self {
            Self::Leaf { .. } => {
                let id = ids
                    .next()
                    .ok_or_else(|| eyre!("session tree has more leaves than panes"))?;
                Ok(Node::Leaf(id))
            }
            Self::Split {
                dir,
                percent,
                first,
                second,
            } => Ok(Node::Split {
                dir: *dir,
                percent: *percent,
                first: Box::new(first.to_live(ids)?),
                second: Box::new(second.to_live(ids)?),
            }),
        }
    }
}

pub fn path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("lolterm").join("session.toml")
}

pub fn load() -> Result<Session> {
    let path = path();
    let text = fs::read_to_string(&path)
        .map_err(|err| eyre!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&text).map_err(|err| eyre!("failed to parse session: {err}"))
}

pub fn save(session: &Session) -> Result<()> {
    let path = path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| eyre!("failed to create {}: {err}", parent.display()))?;
    }
    let text = toml::to_string_pretty(session)
        .map_err(|err| eyre!("failed to serialize session: {err}"))?;
    fs::write(&path, text).map_err(|err| eyre!("failed to write {}: {err}", path.display()))
}

pub fn exists() -> bool {
    Path::new(&path()).is_file()
}
