use std::env;
use std::path::{Path, PathBuf};

use crate::VERSION;
use crate::config;
use crate::git;
use crate::session::{self, SavedWorkspace, Session};
use crate::ssh;
use crate::workspaces;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Status,
    WorkspaceList,
    Help,
    Version,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StatusView {
    pub version: String,
    pub workspace: String,
    pub root: String,
    pub branch: Option<String>,
    pub machines: usize,
    pub tmux_session: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct WorkspaceRow {
    pub name: String,
    pub root: String,
    pub current: bool,
}

pub fn run(args: &[String]) -> Result<i32, String> {
    match parse(args)? {
        Command::Help => {
            print!("{}", help_text());
            Ok(0)
        }
        Command::Version => {
            println!("{VERSION}");
            Ok(0)
        }
        Command::Status => {
            print!("{}", format_status(&load_status()));
            Ok(0)
        }
        Command::WorkspaceList => {
            print!("{}", format_workspace_list(&load_workspace_rows()));
            Ok(0)
        }
    }
}

pub fn parse(args: &[String]) -> Result<Command, String> {
    let mut args = args
        .iter()
        .map(String::as_str)
        .filter(|arg| !arg.is_empty());
    let Some(first) = args.next() else {
        return Ok(Command::Help);
    };
    match first {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "-V" | "--version" | "version" => Ok(Command::Version),
        "status" => {
            if args.next().is_some() {
                return Err("status no admite argumentos".into());
            }
            Ok(Command::Status)
        }
        "workspace" | "ws" => match args.next() {
            None | Some("list") | Some("ls") => {
                if args.next().is_some() {
                    return Err("workspace list no admite argumentos".into());
                }
                Ok(Command::WorkspaceList)
            }
            Some(other) => Err(format!(
                "subcomando desconocido: workspace {other}\nprobá: lolterm workspace list"
            )),
        },
        other => Err(format!(
            "comando desconocido: {other}\nprobá: lolterm status | lolterm workspace list"
        )),
    }
}

pub fn help_text() -> String {
    format!(
        "\
lolterm {VERSION} — control del mismo core que el Desktop

Uso:
  lolterm status
  lolterm workspace list
  lolterm -h | --help
  lolterm -V | --version

Todavía no abre la GUI ni PTYs: lee config y workspaces locales.
"
    )
}

pub fn format_status(view: &StatusView) -> String {
    let branch = view.branch.as_deref().unwrap_or("—");
    let tmux = if view.tmux_session.is_empty() {
        "—".to_string()
    } else {
        view.tmux_session.clone()
    };
    format!(
        "\
LoLTerm {version}
workspace  {workspace}
root       {root}
branch     {branch}
machines   {machines}
tmux       {tmux}
",
        version = view.version,
        workspace = view.workspace,
        root = view.root,
        machines = view.machines,
    )
}

pub fn format_workspace_list(rows: &[WorkspaceRow]) -> String {
    if rows.is_empty() {
        return "ningún workspace (abrí uno desde el Desktop o lolterm . más adelante)\n".into();
    }
    let width = rows
        .iter()
        .map(|row| row.name.len())
        .max()
        .unwrap_or(0)
        .min(24);
    let mut out = String::new();
    for row in rows {
        let mark = if row.current { '*' } else { ' ' };
        let name = truncate_name(&row.name, width);
        out.push_str(&format!("{mark} {name:<width$}  {}\n", row.root));
    }
    out
}

fn truncate_name(name: &str, width: usize) -> String {
    if name.len() <= width {
        return name.to_string();
    }
    let mut cut = name
        .chars()
        .take(width.saturating_sub(1))
        .collect::<String>();
    cut.push('…');
    cut
}

fn load_status() -> StatusView {
    let cfg = config::load();
    let session = loaded_session();
    let cwd = env::current_dir().ok();
    let current = current_workspace(&session, cwd.as_deref());
    let workspace = current
        .map(|ws| ws.name.clone())
        .unwrap_or_else(|| "—".into());
    let root_path = current
        .map(|ws| ws.root.clone())
        .or_else(|| cwd.clone())
        .unwrap_or_else(|| PathBuf::from("."));
    let root = workspaces::compact_root(&root_path);
    let branch = git::branch_label(&root_path);
    let tmux_session = current
        .map(|ws| ssh::tmux_session_name(&cfg.remote.tmux, &ws.name))
        .unwrap_or_default();
    StatusView {
        version: VERSION.to_string(),
        workspace,
        root,
        branch,
        machines: cfg.machines.len(),
        tmux_session,
    }
}

fn load_workspace_rows() -> Vec<WorkspaceRow> {
    let session = loaded_session();
    let cwd = env::current_dir().ok();
    let current_root = current_workspace(&session, cwd.as_deref()).map(|ws| ws.root.clone());
    session
        .workspaces
        .into_iter()
        .map(|ws| {
            let current = current_root.as_ref().is_some_and(|root| *root == ws.root);
            WorkspaceRow {
                name: ws.name,
                root: workspaces::compact_root(&ws.root),
                current,
            }
        })
        .collect()
}

fn loaded_session() -> Session {
    let mut session = session::load().unwrap_or_default();
    workspaces::merge_into_session(&mut session, &workspaces::load());
    session
}

fn current_workspace<'a>(session: &'a Session, cwd: Option<&Path>) -> Option<&'a SavedWorkspace> {
    if let Some(cwd) = cwd {
        let cwd = workspaces::canonical_root(cwd);
        if let Some(ws) = session
            .workspaces
            .iter()
            .filter(|ws| cwd == ws.root || cwd.starts_with(&ws.root))
            .max_by_key(|ws| ws.root.as_os_str().len())
        {
            return Some(ws);
        }
    }
    session.workspaces.get(session.active_workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_commands() {
        assert_eq!(parse(&[]).unwrap(), Command::Help);
        assert_eq!(parse(&["status".into()]).unwrap(), Command::Status);
        assert_eq!(
            parse(&["workspace".into(), "list".into()]).unwrap(),
            Command::WorkspaceList
        );
        assert_eq!(parse(&["ws".into()]).unwrap(), Command::WorkspaceList);
        assert_eq!(parse(&["-V".into()]).unwrap(), Command::Version);
        assert!(parse(&["nope".into()]).is_err());
        assert!(parse(&["workspace".into(), "open".into()]).is_err());
    }

    #[test]
    fn format_status_omits_secrets() {
        let text = format_status(&StatusView {
            version: "0.3.0".into(),
            workspace: "lolterm".into(),
            root: "~/Projects/lolterm".into(),
            branch: Some("master".into()),
            machines: 2,
            tmux_session: "lolterm-lolterm".into(),
        });
        assert!(text.contains("LoLTerm 0.3.0"));
        assert!(text.contains("lolterm-lolterm"));
        assert!(!text.contains("TOKEN"));
        assert!(!text.contains("password"));
    }

    #[test]
    fn format_list_marks_current() {
        let text = format_workspace_list(&[
            WorkspaceRow {
                name: "lolterm".into(),
                root: "~/Projects/lolterm".into(),
                current: true,
            },
            WorkspaceRow {
                name: "api".into(),
                root: "~/dev/api".into(),
                current: false,
            },
        ]);
        assert!(text.starts_with("* lolterm"));
        assert!(text.contains("  api"));
    }

    #[test]
    fn empty_list_explains() {
        let text = format_workspace_list(&[]);
        assert!(text.contains("ningún workspace"));
    }
}
