use serde::{Deserialize, Serialize};

use crate::config;
use crate::layout::SplitDir;
use crate::session::{SavedNode, SavedTab};

#[derive(Clone, Debug, Serialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub hint: String,
}

#[derive(Clone, Debug)]
pub struct PresetTab {
    pub meta: Preset,
    pub tab: SavedTab,
}

pub fn all() -> Vec<PresetTab> {
    let mut list = builtins();
    for extra in load_file() {
        if let Some(existing) = list.iter_mut().find(|item| item.meta.id == extra.meta.id) {
            *existing = extra;
        } else {
            list.push(extra);
        }
    }
    list
}

pub fn get(id: &str) -> Option<PresetTab> {
    all().into_iter().find(|item| item.meta.id == id)
}

pub fn summaries() -> Vec<Preset> {
    all().into_iter().map(|item| item.meta).collect()
}

fn builtins() -> Vec<PresetTab> {
    vec![
        preset(
            "shell",
            "Shell",
            "una terminal",
            SavedTab {
                focused: 0,
                name: Some("shell".into()),
                zoomed: None,
                tree: shell(),
            },
        ),
        preset(
            "split",
            "Split",
            "dos shells lado a lado",
            SavedTab {
                focused: 0,
                name: Some("split".into()),
                zoomed: None,
                tree: split(SplitDir::Columns, 50, shell(), shell()),
            },
        ),
        preset(
            "nvim-shell",
            "nvim + shell",
            "editor a la izquierda, shell a la derecha",
            SavedTab {
                focused: 0,
                name: Some("nvim".into()),
                zoomed: None,
                tree: split(SplitDir::Columns, 62, prog("nvim"), shell()),
            },
        ),
        preset(
            "stack",
            "Stack",
            "nvim arriba, shell abajo",
            SavedTab {
                focused: 0,
                name: Some("stack".into()),
                zoomed: None,
                tree: split(SplitDir::Rows, 70, prog("nvim"), shell()),
            },
        ),
        preset(
            "ide",
            "IDE",
            "nvim | CLI sobre una shell ancha",
            SavedTab {
                focused: 0,
                name: Some("ide".into()),
                zoomed: None,
                tree: split(
                    SplitDir::Rows,
                    72,
                    split(SplitDir::Columns, 62, prog("nvim"), shell()),
                    shell(),
                ),
            },
        ),
    ]
}

fn preset(id: &str, name: &str, hint: &str, tab: SavedTab) -> PresetTab {
    PresetTab {
        meta: Preset {
            id: id.into(),
            name: name.into(),
            hint: hint.into(),
        },
        tab,
    }
}

fn shell() -> SavedNode {
    SavedNode::Leaf {
        cwd: None,
        program: None,
        args: Vec::new(),
    }
}

fn prog(name: &str) -> SavedNode {
    SavedNode::Leaf {
        cwd: None,
        program: Some(name.into()),
        args: Vec::new(),
    }
}

fn split(dir: SplitDir, percent: u16, first: SavedNode, second: SavedNode) -> SavedNode {
    SavedNode::Split {
        dir,
        percent,
        first: Box::new(first),
        second: Box::new(second),
    }
}

fn load_file() -> Vec<PresetTab> {
    let Ok(text) = std::fs::read_to_string(path()) else {
        return Vec::new();
    };
    let parsed: FilePresets = toml::from_str(&text).unwrap_or_default();
    parsed
        .preset
        .into_iter()
        .filter(|item| !item.id.trim().is_empty())
        .map(|item| PresetTab {
            meta: Preset {
                id: item.id,
                name: if item.name.trim().is_empty() {
                    "preset".into()
                } else {
                    item.name
                },
                hint: if item.hint.trim().is_empty() {
                    "archivo presets.toml".into()
                } else {
                    item.hint
                },
            },
            tab: SavedTab {
                focused: item.focused,
                name: Some(item.tab_name.unwrap_or_else(|| "preset".into())),
                zoomed: None,
                tree: item.tree,
            },
        })
        .collect()
}

pub fn path() -> std::path::PathBuf {
    config::config_dir().join("presets.toml")
}

#[derive(Default, Deserialize)]
struct FilePresets {
    #[serde(default)]
    preset: Vec<FilePreset>,
}

#[derive(Deserialize)]
struct FilePreset {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    hint: String,
    #[serde(default)]
    focused: usize,
    #[serde(default)]
    tab_name: Option<String>,
    tree: SavedNode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_have_unique_ids_and_leaves() {
        let list = builtins();
        let mut ids = std::collections::HashSet::new();
        for item in &list {
            assert!(ids.insert(item.meta.id.clone()), "{}", item.meta.id);
            assert!(!item.tab.tree.leaf_specs().is_empty());
        }
        assert!(get("nvim-shell").is_some());
        assert!(get("missing").is_none());
    }
}
