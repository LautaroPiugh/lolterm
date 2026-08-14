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
    RenameTab,
    NewWorkspace,
    NextWorkspace,
    CloseWorkspace,
    LaunchAi,
    LaunchCodex,
    LaunchClaude,
    LaunchOpencode,
    LaunchCline,
    LaunchGemini,
    LaunchLazygit,
    LaunchSsh,
    LaunchTailscale,
    GitStatus,
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
        id: CommandId::RenameTab,
        slash: "tab-rename",
        hint: "renombrar tab: /tab-rename nombre",
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
        id: CommandId::LaunchCline,
        slash: "cline",
        hint: "abrir Cline",
    },
    CommandSpec {
        id: CommandId::LaunchGemini,
        slash: "gemini",
        hint: "abrir Gemini CLI",
    },
    CommandSpec {
        id: CommandId::LaunchLazygit,
        slash: "lazygit",
        hint: "abrir lazygit",
    },
    CommandSpec {
        id: CommandId::LaunchSsh,
        slash: "ssh",
        hint: "abrir ssh: /ssh user@host",
    },
    CommandSpec {
        id: CommandId::LaunchTailscale,
        slash: "tailscale",
        hint: "abrir Tailscale: /tailscale ssh host",
    },
    CommandSpec {
        id: CommandId::GitStatus,
        slash: "git-status",
        hint: "git status --short del workspace",
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

pub fn split_query(query: &str) -> (String, Vec<String>) {
    let trimmed = query.trim().trim_start_matches('/');
    let mut parts = trimmed.split_whitespace();
    let head = parts.next().unwrap_or("").to_ascii_lowercase();
    let args = parts.map(ToString::to_string).collect();
    (head, args)
}

pub fn filter_commands(query: &str) -> Vec<&'static CommandSpec> {
    let (needle, _) = split_query(query);
    if needle.is_empty() {
        return COMMANDS.iter().collect();
    }
    let mut scored: Vec<(u32, &'static CommandSpec)> = COMMANDS
        .iter()
        .filter_map(|command| {
            let slash = fuzzy_score(&needle, command.slash)?;
            let hint = fuzzy_score(&needle, command.hint).unwrap_or(0);
            Some((slash.max(hint / 2), command))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.slash.len().cmp(&b.1.slash.len()))
    });
    scored.into_iter().map(|(_, command)| command).collect()
}

pub fn fuzzy_score(needle: &str, haystack: &str) -> Option<u32> {
    if needle.is_empty() {
        return Some(0);
    }
    let hay: Vec<char> = haystack.chars().map(|ch| ch.to_ascii_lowercase()).collect();
    let mut start = 0;
    let mut score = 0u32;
    let mut prev = None;
    for (index, needle_ch) in needle.chars().map(|ch| ch.to_ascii_lowercase()).enumerate() {
        let found = hay
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, ch)| **ch == needle_ch);
        let (at, _) = found?;
        score += 16;
        if index == 0 && at == 0 {
            score += 32;
        }
        if prev.is_some_and(|prev| at == prev + 1) {
            score += 24;
        }
        prev = Some(at);
        start = at + 1;
    }
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_query_keeps_ssh_args() {
        let (head, args) = split_query("/ssh user@host -t tmux");
        assert_eq!(head, "ssh");
        assert_eq!(args, vec!["user@host", "-t", "tmux"]);
    }

    #[test]
    fn fuzzy_ranks_ssh_for_full_token() {
        let matches = filter_commands("/ssh");
        assert_eq!(matches[0].slash, "ssh");
        assert!(
            filter_commands("/sh")
                .iter()
                .any(|command| command.slash == "ssh")
        );
    }

    #[test]
    fn substring_still_matches() {
        assert!(fuzzy_score("tab", "tab-rename").is_some());
        assert!(fuzzy_score("xyz", "tab-rename").is_none());
    }
}
