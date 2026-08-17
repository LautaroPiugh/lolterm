//! Host de agentes CLI: worktrees, historial, nombres conocidos.
//! LoLTerm no implementa el agente; solo le da cwd y contexto.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config;
use crate::git;
use crate::workspaces;

pub const NAMES: &[&str] = &["codex", "claude", "opencode", "gemini", "cline"];

const HISTORY_CAP: usize = 40;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionRecord {
    pub ts: u64,
    pub workspace: String,
    pub program: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
}

pub fn is_agent(program: Option<&str>) -> bool {
    program.is_some_and(|name| NAMES.contains(&name))
}

pub fn worktree_branch(program: &str, stamp: u64) -> String {
    format!("lolterm/{}/{}", workspaces::slug(program), stamp)
}

pub fn worktree_path(workspace: &str, program: &str, stamp: u64) -> PathBuf {
    data_worktrees_root()
        .join(workspaces::slug(workspace))
        .join(format!("{}-{stamp}", workspaces::slug(program)))
}

pub fn is_our_worktree(path: &Path) -> bool {
    let root = data_worktrees_root();
    path.starts_with(&root)
}

pub fn open_worktree(
    repo: &Path,
    workspace: &str,
    program: &str,
    stamp: u64,
) -> Result<PathBuf, String> {
    let path = worktree_path(workspace, program, stamp);
    git::worktree_add(repo, &path, &worktree_branch(program, stamp))?;
    Ok(path)
}

pub fn append_session(record: &SessionRecord) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut rows = recent_sessions(HISTORY_CAP);
    rows.retain(|row| {
        !(row.workspace == record.workspace
            && row.program == record.program
            && row.worktree == record.worktree
            && record.ts.saturating_sub(row.ts) < 2)
    });
    rows.insert(0, record.clone());
    rows.truncate(HISTORY_CAP);
    let mut text = String::new();
    for row in &rows {
        if let Ok(line) = serde_json::to_string(row) {
            text.push_str(&line);
            text.push('\n');
        }
    }
    let _ = std::fs::write(path, text);
}

pub fn recent_sessions(limit: usize) -> Vec<SessionRecord> {
    let Ok(text) = std::fs::read_to_string(history_path()) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .take(limit)
        .collect()
}

fn data_worktrees_root() -> PathBuf {
    config::data_dir().join("worktrees")
}

fn history_path() -> PathBuf {
    config::data_dir().join("agent-sessions.jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_agent_names() {
        assert!(is_agent(Some("opencode")));
        assert!(is_agent(Some("codex")));
        assert!(!is_agent(Some("nvim")));
        assert!(!is_agent(Some("ssh")));
        assert!(!is_agent(None));
    }

    #[test]
    fn branch_and_path_are_safe() {
        let branch = worktree_branch("Open Code", 99);
        assert!(branch.starts_with("lolterm/"));
        assert!(!branch.contains(' '));
        let path = worktree_path("LoL Term", "opencode", 99);
        assert!(path.ends_with("opencode-99"));
        assert!(!path.starts_with(crate::config::config_dir()));
    }
}
