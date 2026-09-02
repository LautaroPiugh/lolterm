//! Catálogo de CLIs conocidas: versión en PATH e install en un PTY.
//! LoLTerm no baja binarios propios; corre el comando que ya usa cada herramienta.

use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::files;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolKind {
    Agent,
    Cli,
}

#[derive(Clone, Copy)]
pub enum Install {
    /// Comando universal: igual en cualquier distro (npm, cargo, go, curl...).
    Cmd(&'static str),
    /// Variantes por gestor de paquetes del sistema. Se elige según qué
    /// binario está en PATH; si no hay ninguno, se usa la variante apt.
    Pkg {
        apt: &'static str,
        dnf: &'static str,
        pacman: &'static str,
        zypper: &'static str,
    },
}

#[derive(Clone, Copy)]
pub struct Tool {
    pub name: &'static str,
    pub kind: ToolKind,
    pub hint: &'static str,
    pub version_flag: &'static str,
    pub install: Install,
}

pub fn is_known(name: &str) -> bool {
    TOOLS.iter().any(|tool| tool.name == name)
}

pub const TOOLS: &[Tool] = &[
    Tool {
        name: "claude",
        kind: ToolKind::Agent,
        hint: "Claude Code",
        version_flag: "--version",
        install: Install::Cmd("npm install -g @anthropic-ai/claude-code"),
    },
    Tool {
        name: "codex",
        kind: ToolKind::Agent,
        hint: "OpenAI Codex CLI",
        version_flag: "--version",
        install: Install::Cmd("npm install -g @openai/codex"),
    },
    Tool {
        name: "opencode",
        kind: ToolKind::Agent,
        hint: "OpenCode",
        version_flag: "--version",
        install: Install::Cmd("npm install -g opencode-ai"),
    },
    Tool {
        name: "pi",
        kind: ToolKind::Agent,
        hint: "Pi (pi.dev)",
        version_flag: "--version",
        install: Install::Cmd("npm install -g --ignore-scripts @earendil-works/pi-coding-agent"),
    },
    Tool {
        name: "omp",
        kind: ToolKind::Agent,
        hint: "Oh My Pi",
        version_flag: "--version",
        install: Install::Cmd("curl -fsSL https://omp.sh/install | sh"),
    },
    Tool {
        name: "omh",
        kind: ToolKind::Agent,
        hint: "Oh My Hermes",
        version_flag: "--version",
        install: Install::Cmd(
            "curl -fsSL https://raw.githubusercontent.com/rlaope/oh-my-hermes/main/install.sh | sh",
        ),
    },
    Tool {
        name: "hermes",
        kind: ToolKind::Agent,
        hint: "Hermes Agent (Nous Research)",
        version_flag: "--version",
        install: Install::Cmd("curl -fsSL https://hermes-agent.nousresearch.com/install.sh | bash"),
    },
    Tool {
        name: "goose",
        kind: ToolKind::Agent,
        hint: "Goose (Block / AAIF)",
        version_flag: "--version",
        install: Install::Cmd(
            "curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | bash",
        ),
    },
    Tool {
        name: "aider",
        kind: ToolKind::Agent,
        hint: "Aider",
        version_flag: "--version",
        install: Install::Cmd("python -m pip install aider-install && aider-install"),
    },
    Tool {
        name: "crush",
        kind: ToolKind::Agent,
        hint: "Crush (Charmbracelet)",
        version_flag: "--version",
        install: Install::Cmd("go install github.com/charmbracelet/crush@latest"),
    },
    Tool {
        name: "qwen",
        kind: ToolKind::Agent,
        hint: "Qwen Code",
        version_flag: "--version",
        install: Install::Cmd("npm install -g @qwen-code/qwen-code@latest"),
    },
    Tool {
        name: "openhands",
        kind: ToolKind::Agent,
        hint: "OpenHands CLI",
        version_flag: "--version",
        install: Install::Cmd("uv tool install openhands --python 3.12"),
    },
    Tool {
        name: "agy",
        kind: ToolKind::Agent,
        hint: "Antigravity CLI (Google)",
        version_flag: "--version",
        install: Install::Cmd("curl -fsSL https://antigravity.google/cli/install.sh | bash"),
    },
    Tool {
        name: "cline",
        kind: ToolKind::Agent,
        hint: "Cline CLI",
        version_flag: "--version",
        install: Install::Cmd("npm install -g cline"),
    },
    Tool {
        name: "copilot",
        kind: ToolKind::Agent,
        hint: "GitHub Copilot CLI",
        version_flag: "--version",
        install: Install::Cmd("gh extension install github/gh-copilot"),
    },
    Tool {
        name: "lazygit",
        kind: ToolKind::Cli,
        hint: "TUI de git",
        version_flag: "--version",
        install: Install::Pkg {
            apt: "sudo apt-get install -y lazygit || go install github.com/jesseduffield/lazygit@latest",
            dnf: "sudo dnf install -y lazygit || go install github.com/jesseduffield/lazygit@latest",
            pacman: "sudo pacman -S --noconfirm --needed lazygit || go install github.com/jesseduffield/lazygit@latest",
            zypper: "sudo zypper --non-interactive install lazygit || go install github.com/jesseduffield/lazygit@latest",
        },
    },
    Tool {
        name: "nvim",
        kind: ToolKind::Cli,
        hint: "Neovim",
        version_flag: "--version",
        install: Install::Pkg {
            apt: "sudo apt-get install -y neovim",
            dnf: "sudo dnf install -y neovim",
            pacman: "sudo pacman -S --noconfirm --needed neovim",
            zypper: "sudo zypper --non-interactive install neovim",
        },
    },
    Tool {
        name: "btop",
        kind: ToolKind::Cli,
        hint: "monitor del sistema",
        version_flag: "--version",
        install: Install::Pkg {
            apt: "sudo apt-get install -y btop",
            dnf: "sudo dnf install -y btop",
            pacman: "sudo pacman -S --noconfirm --needed btop",
            zypper: "sudo zypper --non-interactive install btop",
        },
    },
    Tool {
        name: "yazi",
        kind: ToolKind::Cli,
        hint: "file manager TUI",
        version_flag: "--version",
        install: Install::Cmd("cargo install --locked yazi-fm yazi-cli"),
    },
    Tool {
        name: "fzf",
        kind: ToolKind::Cli,
        hint: "fuzzy finder",
        version_flag: "--version",
        install: Install::Pkg {
            apt: "sudo apt-get install -y fzf",
            dnf: "sudo dnf install -y fzf",
            pacman: "sudo pacman -S --noconfirm --needed fzf",
            zypper: "sudo zypper --non-interactive install fzf",
        },
    },
    Tool {
        name: "gh",
        kind: ToolKind::Cli,
        hint: "GitHub CLI",
        version_flag: "--version",
        install: Install::Pkg {
            apt: "sudo apt-get install -y gh",
            dnf: "sudo dnf install -y gh",
            pacman: "sudo pacman -S --noconfirm --needed github-cli",
            zypper: "sudo zypper --non-interactive install gh",
        },
    },
    Tool {
        name: "tmux",
        kind: ToolKind::Cli,
        hint: "sesiones remotas",
        version_flag: "-V",
        install: Install::Pkg {
            apt: "sudo apt-get install -y tmux",
            dnf: "sudo dnf install -y tmux",
            pacman: "sudo pacman -S --noconfirm --needed tmux",
            zypper: "sudo zypper --non-interactive install tmux",
        },
    },
    Tool {
        name: "rg",
        kind: ToolKind::Cli,
        hint: "ripgrep",
        version_flag: "--version",
        install: Install::Pkg {
            apt: "sudo apt-get install -y ripgrep",
            dnf: "sudo dnf install -y ripgrep",
            pacman: "sudo pacman -S --noconfirm --needed ripgrep",
            zypper: "sudo zypper --non-interactive install ripgrep",
        },
    },
    Tool {
        name: "delta",
        kind: ToolKind::Cli,
        hint: "pager de diffs git",
        version_flag: "--version",
        install: Install::Pkg {
            apt: "sudo apt-get install -y git-delta",
            dnf: "sudo dnf install -y git-delta",
            pacman: "sudo pacman -S --noconfirm --needed git-delta",
            zypper: "sudo zypper --non-interactive install git-delta",
        },
    },
];

#[derive(Clone, Debug, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub kind: ToolKind,
    pub hint: String,
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

pub fn listing() -> Vec<ToolInfo> {
    listing_inner(false)
}

/// Completa `--version` en segundo plano. El snapshot de arranque no debe esperar eso.
pub fn refresh_versions() {
    let _ = listing_inner(true);
}

fn listing_inner(want_versions: bool) -> Vec<ToolInfo> {
    if let Ok(guard) = CACHE.lock()
        && let Some(cache) = guard.as_ref()
        && cache.at.elapsed() < Duration::from_secs(45)
        && (!want_versions || cache.rows.iter().any(|row| row.version.is_some()))
    {
        return cache.rows.clone();
    }
    let rows: Vec<ToolInfo> = TOOLS
        .iter()
        .map(|tool| probe(tool, want_versions))
        .collect();
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some(Cache {
            at: Instant::now(),
            rows: rows.clone(),
        });
    }
    rows
}

pub fn install_cmd(name: &str) -> Option<String> {
    TOOLS
        .iter()
        .find(|tool| tool.name == name)
        .map(|tool| resolve_install(tool.install))
}

/// Elige la variante del comando según el gestor de paquetes disponible.
/// El orden va de más común a menos; sin gestor conocido, apt mantiene el
/// comportamiento previo al soporte multi-distro.
fn resolve_install(spec: Install) -> String {
    match spec {
        Install::Cmd(cmd) => cmd.to_string(),
        Install::Pkg {
            apt,
            dnf,
            pacman,
            zypper,
        } => {
            if files::command_on_path("apt-get") {
                apt.to_string()
            } else if files::command_on_path("dnf") {
                dnf.to_string()
            } else if files::command_on_path("pacman") {
                pacman.to_string()
            } else if files::command_on_path("zypper") {
                zypper.to_string()
            } else {
                apt.to_string()
            }
        }
    }
}

pub fn invalidate() {
    crate::files::invalidate_path();
    if let Ok(mut guard) = CACHE.lock() {
        *guard = None;
    }
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

fn probe(tool: &Tool, versions: bool) -> ToolInfo {
    let available = files::command_on_path(tool.name);
    let version = (versions && available).then(|| read_version(tool.name, tool.version_flag));
    ToolInfo {
        name: tool.name.into(),
        kind: tool.kind,
        hint: tool.hint.into(),
        available,
        version: version.flatten(),
        install: resolve_install(tool.install),
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
        assert!(install_cmd("lazygit").is_some());
        assert!(crate::registry::is_known("nvim"));
        assert!(crate::registry::is_known("claude"));
        assert!(!crate::registry::is_known("nope"));
        assert!(install_cmd("nvim").is_some());
        assert!(install_cmd("nope").is_none());
        assert!(
            TOOLS
                .iter()
                .any(|tool| tool.name == "claude" && tool.kind == ToolKind::Agent)
        );
        assert!(
            TOOLS
                .iter()
                .any(|tool| tool.name == "lazygit" && tool.kind == ToolKind::Cli)
        );
    }

    #[test]
    fn every_tool_resolves_a_non_empty_install_cmd() {
        for tool in TOOLS {
            let cmd = install_cmd(tool.name).unwrap_or_default();
            assert!(
                !cmd.trim().is_empty(),
                "{} sin comando de install",
                tool.name
            );
        }
        assert!(install_cmd("nope").is_none());
    }

    #[test]
    fn universal_install_is_used_verbatim() {
        let cmd = resolve_install(Install::Cmd("cargo install demo"));
        assert_eq!(cmd, "cargo install demo");
    }
}
