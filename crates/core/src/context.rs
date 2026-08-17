use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config;
use crate::session;
use crate::workspaces;

/// JSON que consume `lolterm context` y, más adelante, agentes CLI.
/// Nunca lleva valores de env ni args de panes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextGit {
    pub branch: Option<String>,
    pub remote: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPane {
    pub tab: usize,
    pub tab_name: String,
    pub program: String,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextView {
    pub version: String,
    pub live: bool,
    pub workspace: String,
    pub cwd: String,
    pub machine: String,
    pub git: ContextGit,
    pub tmux: String,
    pub processes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_process: Option<String>,
    pub panes: Vec<ContextPane>,
    /// Solo nombres. Nunca valores (tokens, passwords, PATH completo).
    pub env: Vec<String>,
    pub machines: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worktrees: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

pub fn format_context(view: &ContextView) -> String {
    let mut text = serde_json::to_string_pretty(view).unwrap_or_else(|_| "{}".into());
    text.push('\n');
    text
}

/// Foto local para procesos dentro de un PTY (`LOLTERM_CONTEXT`).
/// Mismo JSON que `lolterm context`; no va en config sincronizable.
pub fn live_file_path() -> PathBuf {
    config::runtime_dir().join("context.json")
}

pub fn write_live_file(view: &ContextView) -> std::io::Result<PathBuf> {
    write_live_file_at(&live_file_path(), view)
}

fn write_live_file_at(path: &Path, view: &ContextView) -> std::io::Result<PathBuf> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, format_context(view))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(path.to_path_buf())
}

pub fn env_keys_public<'a>(keys: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut out = Vec::new();
    for key in keys {
        if !session::env_key_ok(key) || looks_secret(key) {
            continue;
        }
        if !out.iter().any(|seen| seen == key) {
            out.push(key.to_string());
        }
    }
    out
}

pub fn looks_secret(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "APIKEY",
        "API_KEY",
        "CREDENTIAL",
        "PRIVATE",
        "AUTH",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

pub fn compact_cwd(path: &std::path::Path) -> String {
    workspaces::compact_root(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_keys_are_dropped() {
        let keys = env_keys_public(["FOO", "GITHUB_TOKEN", "OPENAI_API_KEY", "PATH"]);
        assert_eq!(keys, vec!["FOO", "PATH"]);
    }

    #[test]
    fn format_omits_env_values() {
        let text = format_context(&ContextView {
            version: "0.6.0".into(),
            live: true,
            workspace: "lolterm".into(),
            cwd: "~/Projects/lolterm".into(),
            machine: "local".into(),
            git: ContextGit {
                branch: Some("master".into()),
                remote: Some("github.com/LautaroPiugh/lolterm".into()),
            },
            tmux: "lolterm-lolterm".into(),
            processes: vec!["nvim".into()],
            focused_process: Some("nvim".into()),
            panes: vec![ContextPane {
                tab: 0,
                tab_name: "nvim".into(),
                program: "nvim".into(),
                cwd: "~/Projects/lolterm".into(),
                remote: None,
                focused: true,
                worktree: None,
            }],
            env: vec!["FOO".into()],
            machines: vec!["chae".into()],
            worktrees: Vec::new(),
            extra: std::collections::BTreeMap::new(),
        });
        assert!(text.contains("\"live\": true"));
        assert!(text.contains("nvim"));
        assert!(text.contains("FOO"));
        assert!(!text.contains("TOKEN"));
        assert!(!text.contains("password"));
        assert!(!text.contains("sk-"));
    }

    #[test]
    fn live_file_is_outside_portable_config() {
        let path = live_file_path();
        let config = crate::config::config_dir();
        assert!(
            !path.starts_with(&config),
            "context.json no debe ir en config sincronizable: {}",
            path.display()
        );
        assert!(path.ends_with("context.json"));
    }

    #[test]
    fn write_live_file_roundtrip_and_mode() {
        let dir = std::env::temp_dir().join(format!("lolterm-context-{}", std::process::id()));
        let path = dir.join("context.json");
        let view = ContextView {
            version: "0.6.0".into(),
            live: true,
            workspace: "lolterm".into(),
            cwd: "~/Projects/lolterm".into(),
            machine: "local".into(),
            git: ContextGit {
                branch: Some("master".into()),
                remote: None,
            },
            tmux: String::new(),
            processes: vec!["codex".into()],
            focused_process: Some("codex".into()),
            panes: Vec::new(),
            env: vec!["TERM".into()],
            machines: Vec::new(),
            worktrees: Vec::new(),
            extra: std::collections::BTreeMap::new(),
        };
        write_live_file_at(&path, &view).expect("write context.json");
        let text = std::fs::read_to_string(&path).expect("read context.json");
        assert!(text.contains("\"live\": true"));
        assert!(text.contains("codex"));
        assert!(!text.contains("sk-"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
