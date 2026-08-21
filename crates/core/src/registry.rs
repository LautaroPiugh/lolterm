//! Catálogo de CLIs conocidas: versión en PATH e install en un PTY.
//! LoLTerm no baja binarios propios; corre el comando que ya usa cada herramienta.

use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::files;

#[derive(Clone, Copy)]
pub struct Tool {
    pub name: &'static str,
    pub version_flag: &'static str,
    pub install: &'static str,
}

pub const AGENTS: &[Tool] = &[
    Tool {
        name: "claude",
        version_flag: "--version",
        install: "npm install -g @anthropic-ai/claude-code",
    },
    Tool {
        name: "codex",
        version_flag: "--version",
        install: "npm install -g @openai/codex",
    },
    Tool {
        name: "opencode",
        version_flag: "--version",
        install: "npm install -g opencode-ai",
    },
    Tool {
        name: "gemini",
        version_flag: "--version",
        install: "npm install -g @google/gemini-cli",
    },
    Tool {
        name: "cline",
        version_flag: "--version",
        install: "npm install -g cline",
    },
    Tool {
        name: "copilot",
        version_flag: "--version",
        install: "gh extension install github/gh-copilot",
    },
];

#[derive(Clone, Debug, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub install: String,
}

struct Cache {
    at: Instant,
    rows: Vec<ToolInfo>,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

pub fn agents() -> Vec<ToolInfo> {
    if let Ok(guard) = CACHE.lock()
        && let Some(cache) = guard.as_ref()
        && cache.at.elapsed() < Duration::from_secs(45)
    {
        return cache.rows.clone();
    }
    let rows: Vec<ToolInfo> = AGENTS.iter().map(probe).collect();
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some(Cache {
            at: Instant::now(),
            rows: rows.clone(),
        });
    }
    rows
}

pub fn install_cmd(name: &str) -> Option<&'static str> {
    AGENTS
        .iter()
        .find(|tool| tool.name == name)
        .map(|tool| tool.install)
}

pub fn listing() -> Vec<ToolInfo> {
    if let Ok(guard) = CACHE.lock()
        && let Some(cache) = guard.as_ref()
    {
        return cache.rows.clone();
    }
    AGENTS
        .iter()
        .map(|tool| ToolInfo {
            name: tool.name.into(),
            available: files::command_on_path(tool.name),
            version: None,
            install: tool.install.into(),
        })
        .collect()
}

pub fn version_of(name: &str) -> Option<String> {
    if let Ok(guard) = CACHE.lock()
        && let Some(cache) = guard.as_ref()
    {
        return cache
            .rows
            .iter()
            .find(|row| row.name == name)
            .and_then(|row| row.version.clone());
    }
    None
}

fn probe(tool: &Tool) -> ToolInfo {
    let available = files::command_on_path(tool.name);
    let version = available.then(|| read_version(tool.name, tool.version_flag));
    ToolInfo {
        name: tool.name.into(),
        available,
        version: version.flatten(),
        install: tool.install.into(),
    }
}

fn read_version(bin: &str, flag: &str) -> Option<String> {
    let output = Command::new(bin).arg(flag).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    let line = text.lines().next().or_else(|| err.lines().next())?;
    let line = line.trim();
    (!line.is_empty()).then(|| line.chars().take(48).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_agents_have_install_cmd() {
        assert!(install_cmd("claude").is_some());
        assert!(install_cmd("nope").is_none());
    }
}
