#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandId {
    SplitRight,
    SplitDown,
    Grow,
    Shrink,
    FocusNext,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    ClosePane,
    NewTab,
    NextTab,
    PrevTab,
    CloseTab,
    NewWorkspace,
    NextWorkspace,
    CloseWorkspace,
    LaunchAi,
    LaunchCodex,
    LaunchClaude,
    LaunchOpencode,
    LaunchGemini,
    ScrollUp,
    ScrollDown,
    Quit,
}

#[derive(Clone, Copy)]
pub struct CommandSpec {
    pub id: CommandId,
    pub slash: &'static str,
    pub hint: &'static str,
}

pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        id: CommandId::SplitRight,
        slash: "split-right",
        hint: "partir el pane a la derecha",
    },
    CommandSpec {
        id: CommandId::SplitDown,
        slash: "split-down",
        hint: "partir el pane abajo",
    },
    CommandSpec {
        id: CommandId::Grow,
        slash: "grow",
        hint: "agrandar el pane con foco",
    },
    CommandSpec {
        id: CommandId::Shrink,
        slash: "shrink",
        hint: "achicar el pane con foco",
    },
    CommandSpec {
        id: CommandId::FocusNext,
        slash: "focus-next",
        hint: "foco al siguiente pane",
    },
    CommandSpec {
        id: CommandId::FocusLeft,
        slash: "focus-left",
        hint: "foco al pane de la izquierda",
    },
    CommandSpec {
        id: CommandId::FocusRight,
        slash: "focus-right",
        hint: "foco al pane de la derecha",
    },
    CommandSpec {
        id: CommandId::FocusUp,
        slash: "focus-up",
        hint: "foco al pane de arriba",
    },
    CommandSpec {
        id: CommandId::FocusDown,
        slash: "focus-down",
        hint: "foco al pane de abajo",
    },
    CommandSpec {
        id: CommandId::ClosePane,
        slash: "close-pane",
        hint: "cerrar el pane con foco",
    },
    CommandSpec {
        id: CommandId::NewTab,
        slash: "tab-new",
        hint: "nueva tab",
    },
    CommandSpec {
        id: CommandId::NextTab,
        slash: "tab-next",
        hint: "tab siguiente",
    },
    CommandSpec {
        id: CommandId::PrevTab,
        slash: "tab-prev",
        hint: "tab anterior",
    },
    CommandSpec {
        id: CommandId::CloseTab,
        slash: "tab-close",
        hint: "cerrar tab",
    },
    CommandSpec {
        id: CommandId::NewWorkspace,
        slash: "workspace-new",
        hint: "workspace desde el cwd del pane",
    },
    CommandSpec {
        id: CommandId::NextWorkspace,
        slash: "workspace-next",
        hint: "siguiente workspace",
    },
    CommandSpec {
        id: CommandId::CloseWorkspace,
        slash: "workspace-close",
        hint: "cerrar workspace",
    },
    CommandSpec {
        id: CommandId::LaunchAi,
        slash: "ai",
        hint: "abrir la primera CLI de IA en PATH",
    },
    CommandSpec {
        id: CommandId::LaunchCodex,
        slash: "codex",
        hint: "abrir Codex",
    },
    CommandSpec {
        id: CommandId::LaunchClaude,
        slash: "claude",
        hint: "abrir Claude Code",
    },
    CommandSpec {
        id: CommandId::LaunchOpencode,
        slash: "opencode",
        hint: "abrir OpenCode",
    },
    CommandSpec {
        id: CommandId::LaunchGemini,
        slash: "gemini",
        hint: "abrir Gemini CLI",
    },
    CommandSpec {
        id: CommandId::ScrollUp,
        slash: "scroll-up",
        hint: "subir el historial del pane",
    },
    CommandSpec {
        id: CommandId::ScrollDown,
        slash: "scroll-down",
        hint: "bajar el historial del pane",
    },
    CommandSpec {
        id: CommandId::Quit,
        slash: "quit",
        hint: "salir de LolTerm",
    },
];

pub fn filter_commands(query: &str) -> Vec<&'static CommandSpec> {
    let needle = query.trim().trim_start_matches('/').to_ascii_lowercase();
    if needle.is_empty() {
        return COMMANDS.iter().collect();
    }
    COMMANDS
        .iter()
        .filter(|command| {
            command.slash.contains(&needle) || command.hint.to_ascii_lowercase().contains(&needle)
        })
        .collect()
}
