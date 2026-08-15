use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::Result;
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};

use crate::layout::{LayoutNode, SplitDir};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Session {
    pub active_workspace: usize,
    #[serde(default)]
    pub workspaces: Vec<SavedWorkspace>,
    #[serde(default)]
    pub recents: Vec<String>,
    #[serde(default)]
    pub recent_projects: Vec<PathBuf>,
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
        #[serde(default)]
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
    pub fn from_layout(node: &LayoutNode, keep: &HashSet<u64>, cwds: &[(u64, PathBuf)]) -> Self {
        match node {
            LayoutNode::Leaf { pane } => Self::Leaf {
                cwd: cwds
                    .iter()
                    .find(|(id, _)| keep.contains(id) && *id == *pane)
                    .map(|(_, cwd)| cwd.clone()),
            },
            LayoutNode::Split {
                dir,
                percent,
                first,
                second,
            } => Self::Split {
                dir: *dir,
                percent: *percent,
                first: Box::new(Self::from_layout(first, keep, cwds)),
                second: Box::new(Self::from_layout(second, keep, cwds)),
            },
        }
    }

    pub fn leaf_cwds(&self) -> Vec<Option<PathBuf>> {
        let mut out = Vec::new();
        collect_cwds(self, &mut out);
        out
    }
}

fn collect_cwds(node: &SavedNode, out: &mut Vec<Option<PathBuf>>) {
    match node {
        SavedNode::Leaf { cwd } => out.push(cwd.clone()),
        SavedNode::Split { first, second, .. } => {
            collect_cwds(first, out);
            collect_cwds(second, out);
        }
    }
}

pub fn push_unique_path(list: &mut Vec<PathBuf>, value: PathBuf, max: usize) {
    list.retain(|item| item != &value);
    list.insert(0, value);
    list.truncate(max);
}

pub fn push_unique(list: &mut Vec<String>, value: String, max: usize) {
    list.retain(|item| item != &value);
    list.insert(0, value);
    list.truncate(max);
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
