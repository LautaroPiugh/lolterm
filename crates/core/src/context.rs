use serde::{Deserialize, Serialize};

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
    pub panes: Vec<ContextPane>,
    /// Solo nombres. Nunca valores (tokens, passwords, PATH completo).
    pub env: Vec<String>,
    pub machines: Vec<String>,
}

pub fn format_context(view: &ContextView) -> String {
    let mut text = serde_json::to_string_pretty(view).unwrap_or_else(|_| "{}".into());
    text.push('\n');
    text
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
            panes: vec![ContextPane {
                tab: 0,
                tab_name: "nvim".into(),
                program: "nvim".into(),
                cwd: "~/Projects/lolterm".into(),
                remote: None,
                focused: true,
            }],
            env: vec!["FOO".into()],
            machines: vec!["chae".into()],
        });
        assert!(text.contains("\"live\": true"));
        assert!(text.contains("nvim"));
        assert!(text.contains("FOO"));
        assert!(!text.contains("TOKEN"));
        assert!(!text.contains("password"));
        assert!(!text.contains("sk-"));
    }
}
