use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    Core,
    Ui,
}

#[derive(Clone, Copy, Debug)]
pub struct CommandSpec {
    pub id: &'static str,
    pub slash: &'static str,
    pub hint: &'static str,
    pub kind: CommandKind,
}

pub const REGISTRY: &[CommandSpec] = &[
    CommandSpec {
        id: "tab.new",
        slash: "tab-new",
        hint: "nueva tab (default de +)",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "tab.close",
        slash: "tab-close",
        hint: "cerrar tab",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "tab.duplicate",
        slash: "tab-dup",
        hint: "duplicar tab (nuevos PTYs, mismo layout)",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "tab.next",
        slash: "tab-next",
        hint: "siguiente tab",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "tab.prev",
        slash: "tab-prev",
        hint: "tab anterior",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.splitRight",
        slash: "split-right",
        hint: "partir el pane a la derecha",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.splitDown",
        slash: "split-down",
        hint: "partir el pane abajo",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.zoom",
        slash: "zoom",
        hint: "maximizar o restaurar el pane",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.focusLeft",
        slash: "pane-left",
        hint: "foco al pane de la izquierda",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.focusRight",
        slash: "pane-right",
        hint: "foco al pane de la derecha",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.focusUp",
        slash: "pane-up",
        hint: "foco al pane de arriba",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.focusDown",
        slash: "pane-down",
        hint: "foco al pane de abajo",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.swapLeft",
        slash: "swap-left",
        hint: "intercambiar con el pane de la izquierda",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.swapRight",
        slash: "swap-right",
        hint: "intercambiar con el pane de la derecha",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.swapUp",
        slash: "swap-up",
        hint: "intercambiar con el pane de arriba",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.swapDown",
        slash: "swap-down",
        hint: "intercambiar con el pane de abajo",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.close",
        slash: "pane-close",
        hint: "cerrar pane",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "pane.restart",
        slash: "pane-restart",
        hint: "reiniciar el proceso del pane",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "run.lazygit",
        slash: "lazygit",
        hint: "abrir lazygit",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "run.codex",
        slash: "codex",
        hint: "abrir Codex (worktree + contexto)",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "run.claude",
        slash: "claude",
        hint: "abrir Claude Code (worktree + contexto)",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "run.opencode",
        slash: "opencode",
        hint: "abrir OpenCode (worktree + contexto)",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "run.gemini",
        slash: "gemini",
        hint: "abrir Gemini CLI (worktree + contexto)",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "run.cline",
        slash: "cline",
        hint: "abrir Cline (worktree + contexto)",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "workspace.next",
        slash: "ws-next",
        hint: "siguiente workspace del catálogo",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "workspace.prev",
        slash: "ws-prev",
        hint: "workspace anterior del catálogo",
        kind: CommandKind::Core,
    },
    CommandSpec {
        id: "ui.palette",
        slash: "palette",
        hint: "abrir la paleta",
        kind: CommandKind::Ui,
    },
    CommandSpec {
        id: "ui.run",
        slash: "run",
        hint: "abrir una CLI en un pane o tab",
        kind: CommandKind::Ui,
    },
    CommandSpec {
        id: "ui.files",
        slash: "files",
        hint: "buscar archivos",
        kind: CommandKind::Ui,
    },
    CommandSpec {
        id: "ui.ssh",
        slash: "ssh",
        hint: "hosts de ~/.ssh/config",
        kind: CommandKind::Ui,
    },
    CommandSpec {
        id: "ui.tsSsh",
        slash: "ts-ssh",
        hint: "máquina Tailscale, después usuario",
        kind: CommandKind::Ui,
    },
    CommandSpec {
        id: "ui.sidebar",
        slash: "sidebar",
        hint: "mostrar u ocultar explorer",
        kind: CommandKind::Ui,
    },
    CommandSpec {
        id: "ui.theme",
        slash: "theme",
        hint: "sage, dusk, mono o un tema de ~/.config/lolterm/themes",
        kind: CommandKind::Ui,
    },
    CommandSpec {
        id: "ui.tabRename",
        slash: "tab-rename",
        hint: "renombrar la tab activa",
        kind: CommandKind::Ui,
    },
    CommandSpec {
        id: "ui.commands",
        slash: "commands",
        hint: "editar comandos custom y atajos",
        kind: CommandKind::Ui,
    },
];

#[derive(Serialize)]
pub struct CommandHit {
    pub id: String,
    pub slash: String,
    pub hint: String,
}

pub fn lookup(name: &str) -> Option<&'static CommandSpec> {
    let needle = name.trim().trim_start_matches('/');
    REGISTRY
        .iter()
        .find(|spec| spec.id == needle || spec.slash == needle)
}

pub fn search(query: &str) -> Vec<CommandHit> {
    let needle = query.trim().trim_start_matches('/');
    REGISTRY
        .iter()
        .filter(|spec| {
            needle.is_empty()
                || spec.id.contains(needle)
                || spec.slash.contains(needle)
                || spec
                    .hint
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase())
        })
        .map(|spec| CommandHit {
            id: spec.id.to_string(),
            slash: spec.slash.to_string(),
            hint: spec.hint.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_accepts_id_or_slash() {
        assert_eq!(lookup("tab.new").map(|s| s.slash), Some("tab-new"));
        assert_eq!(lookup("tab-next").map(|s| s.id), Some("tab.next"));
        assert_eq!(lookup("tab-prev").map(|s| s.id), Some("tab.prev"));
        assert_eq!(lookup("/zoom").map(|s| s.id), Some("pane.zoom"));
        assert_eq!(lookup("opencode").map(|s| s.id), Some("run.opencode"));
        assert_eq!(lookup("ws-next").map(|s| s.id), Some("workspace.next"));
        assert!(lookup("nope").is_none());
    }
}
