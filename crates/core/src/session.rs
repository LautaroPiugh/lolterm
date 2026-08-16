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
    #[serde(default)]
    pub zoomed: Option<usize>,
    pub tree: SavedNode,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SavedNode {
    Leaf {
        #[serde(default)]
        cwd: Option<PathBuf>,
        #[serde(default)]
        program: Option<String>,
        #[serde(default)]
        args: Vec<String>,
    },
    Split {
        dir: SplitDir,
        percent: u16,
        first: Box<SavedNode>,
        second: Box<SavedNode>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct LeafSpec {
    pub cwd: Option<PathBuf>,
    pub program: Option<String>,
    pub args: Vec<String>,
}

impl SavedNode {
    pub fn from_layout(
        node: &LayoutNode,
        keep: &HashSet<u64>,
        specs: &std::collections::HashMap<u64, LeafSpec>,
    ) -> Self {
        match node {
            LayoutNode::Leaf { pane } => {
                let spec = keep
                    .contains(pane)
                    .then(|| specs.get(pane))
                    .flatten()
                    .cloned()
                    .unwrap_or_default();
                Self::Leaf {
                    cwd: spec.cwd,
                    program: spec.program,
                    args: spec.args,
                }
            }
            LayoutNode::Split {
                dir,
                percent,
                first,
                second,
            } => Self::Split {
                dir: *dir,
                percent: *percent,
                first: Box::new(Self::from_layout(first, keep, specs)),
                second: Box::new(Self::from_layout(second, keep, specs)),
            },
        }
    }

    pub fn leaf_specs(&self) -> Vec<LeafSpec> {
        let mut out = Vec::new();
        collect_specs(self, &mut out);
        out
    }
}

fn collect_specs(node: &SavedNode, out: &mut Vec<LeafSpec>) {
    match node {
        SavedNode::Leaf { cwd, program, args } => out.push(LeafSpec {
            cwd: cwd.clone(),
            program: program.clone(),
            args: args.clone(),
        }),
        SavedNode::Split { first, second, .. } => {
            collect_specs(first, out);
            collect_specs(second, out);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutNode;

    #[test]
    fn leaf_roundtrip_keeps_program_and_args() {
        let mut specs = std::collections::HashMap::new();
        specs.insert(
            2,
            LeafSpec {
                cwd: Some(PathBuf::from("/tmp/proj")),
                program: Some("nvim".into()),
                args: vec!["README.md".into()],
            },
        );
        let mut keep = HashSet::new();
        keep.insert(2);
        let node = SavedNode::from_layout(&LayoutNode::leaf(2), &keep, &specs);
        let specs = node.leaf_specs();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].program.as_deref(), Some("nvim"));
        assert_eq!(specs[0].args, vec!["README.md"]);
    }

    #[test]
    fn old_session_leaf_without_program_still_parses() {
        let node: SavedNode = toml::from_str("type = \"leaf\"\ncwd = \"/tmp\"\n").expect("toml");
        match node {
            SavedNode::Leaf { program, args, .. } => {
                assert!(program.is_none());
                assert!(args.is_empty());
            }
            other => panic!("{other:?}"),
        }
    }
}
