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
    Ensure(String),
    Open(String),
    Forget(String),
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
        Command::Ensure(raw) => {
            let root = resolve_dir(&raw)?;
            print!("{}", format_status(&ensure_workspace(&root)?));
            Ok(0)
        }
        Command::Open(name) => {
            print!("{}", format_status(&open_workspace(&name)?));
            Ok(0)
        }
        Command::Forget(name) => {
            forget_workspace(&name)?;
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
            Some("forget") | Some("rm") => {
                let Some(name) = args.next() else {
                    return Err("hace falta un nombre: lolterm workspace forget <nombre>".into());
                };
                if args.next().is_some() {
                    return Err("workspace forget admite un solo nombre".into());
                }
                Ok(Command::Forget(name.to_string()))
            }
            Some("open") => {
                let Some(name) = args.next() else {
                    return Err("hace falta un nombre: lolterm workspace open <nombre>".into());
                };
                if args.next().is_some() {
                    return Err("workspace open admite un solo nombre".into());
                }
                Ok(Command::Open(name.to_string()))
            }
            Some(other) => Err(format!(
                "subcomando desconocido: workspace {other}\nprobá: lolterm workspace list | lolterm workspace open <nombre>"
            )),
        },
        other if looks_like_dir(other) => {
            if args.next().is_some() {
                return Err("lolterm . admite un solo path".into());
            }
            Ok(Command::Ensure(other.to_string()))
        }
        other => Err(format!(
            "comando desconocido: {other}\nprobá: lolterm status | lolterm . | lolterm workspace list"
        )),
    }
}

pub fn help_text() -> String {
    format!(
        "\
lolterm {VERSION} — control del mismo core que el Desktop

Uso:
  lolterm .
  lolterm ~/dev/api
  lolterm status
  lolterm workspace list
  lolterm workspace open <nombre>
  lolterm workspace forget <nombre>
  lolterm -h | --help
  lolterm -V | --version

Registra workspaces en ~/.config/lolterm (el mismo catálogo que el Desktop).
No abre la GUI: el próximo arranque de LoLTerm usa el workspace activo.
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

fn looks_like_dir(arg: &str) -> bool {
    matches!(arg, "." | "..")
        || arg.starts_with('/')
        || arg.starts_with('~')
        || arg.starts_with("./")
        || arg.starts_with("../")
        || arg.contains('/')
        || Path::new(arg).is_dir()
}

fn resolve_dir(raw: &str) -> Result<PathBuf, String> {
    let path = workspaces::expand_root(raw);
    if !path.is_dir() {
        return Err(format!("no es un directorio: {raw}"));
    }
    Ok(workspaces::canonical_root(&path))
}

pub fn activate_in_session(session: &mut Session, name: &str, root: PathBuf) {
    let root = workspaces::canonical_root(&root);
    if let Some(index) = session.workspaces.iter().position(|ws| ws.root == root) {
        session.active_workspace = index;
        return;
    }
    session.workspaces.push(SavedWorkspace {
        name: name.to_string(),
        root,
        active_tab: 0,
        tabs: Vec::new(),
        startup: Vec::new(),
        env: Vec::new(),
    });
    session.active_workspace = session.workspaces.len() - 1;
}

fn persist_active(name: &str, root: &Path) -> Result<(), String> {
    let mut catalog = workspaces::load();
    workspaces::ensure_in_catalog(&mut catalog.workspaces, name, root);
    workspaces::save(&catalog).map_err(|err| err.to_string())?;
    let mut session = session::load().unwrap_or_default();
    activate_in_session(&mut session, name, root.to_path_buf());
    session::save(&session).map_err(|err| err.to_string())?;
    Ok(())
}

fn ensure_workspace(root: &Path) -> Result<StatusView, String> {
    let session = loaded_session();
    if let Some(parent) = enclosing_workspace(&session, root) {
        let name = parent.name.clone();
        let parent_root = parent.root.clone();
        eprintln!(
            "lolterm: {} está dentro de {} ({}); no creé un workspace anidado",
            workspaces::compact_root(root),
            name,
            workspaces::compact_root(&parent_root)
        );
        persist_active(&name, &parent_root)?;
        return Ok(status_for(Some(&name), &parent_root));
    }
    let name = workspaces::name_from_root(root);
    persist_active(&name, root)?;
    Ok(status_for(Some(name.as_str()), root))
}

fn open_workspace(name: &str) -> Result<StatusView, String> {
    let needle = name.trim();
    let session = loaded_session();
    let Some(ws) = session
        .workspaces
        .iter()
        .find(|ws| ws.name == needle || workspaces::slug(&ws.name) == workspaces::slug(needle))
    else {
        return Err(format!(
            "workspace desconocido: {needle}\nprobá: lolterm workspace list"
        ));
    };
    persist_active(&ws.name, &ws.root)?;
    Ok(status_for(Some(&ws.name), &ws.root))
}

fn forget_workspace(name: &str) -> Result<(), String> {
    let needle = name.trim();
    let mut session = loaded_session();
    let Some(index) = session
        .workspaces
        .iter()
        .position(|ws| ws.name == needle || workspaces::slug(&ws.name) == workspaces::slug(needle))
    else {
        return Err(format!(
            "workspace desconocido: {needle}\nprobá: lolterm workspace list"
        ));
    };
    let removed = session.workspaces.remove(index);
    if session.active_workspace > index {
        session.active_workspace -= 1;
    }
    if session.workspaces.is_empty() {
        session.active_workspace = 0;
    } else {
        session.active_workspace = session.active_workspace.min(session.workspaces.len() - 1);
    }
    let mut catalog = workspaces::load();
    workspaces::remove_root(&mut catalog.workspaces, &removed.root);
    workspaces::save(&catalog).map_err(|err| err.to_string())?;
    session::save(&session).map_err(|err| err.to_string())?;
    Ok(())
}

fn enclosing_workspace<'a>(session: &'a Session, root: &Path) -> Option<&'a SavedWorkspace> {
    let root = workspaces::canonical_root(root);
    session
        .workspaces
        .iter()
        .filter(|ws| root.starts_with(&ws.root) && root != ws.root)
        .max_by_key(|ws| ws.root.as_os_str().len())
}

fn load_status() -> StatusView {
    let session = loaded_session();
    match session.workspaces.get(session.active_workspace) {
        Some(ws) => status_for(Some(&ws.name), &ws.root),
        None => {
            let cwd = env::current_dir().ok();
            status_for(None, cwd.as_deref().unwrap_or(Path::new(".")))
        }
    }
}

fn status_for(name: Option<&str>, root: &Path) -> StatusView {
    let cfg = config::load();
    let workspace = name
        .map(str::to_string)
        .unwrap_or_else(|| workspaces::name_from_root(root));
    StatusView {
        version: VERSION.to_string(),
        workspace: workspace.clone(),
        root: workspaces::compact_root(root),
        branch: git::branch_label(root),
        machines: cfg.machines.len(),
        tmux_session: ssh::tmux_session_name(&cfg.remote.tmux, &workspace),
    }
}

fn load_workspace_rows() -> Vec<WorkspaceRow> {
    let session = loaded_session();
    let active_root = session
        .workspaces
        .get(session.active_workspace)
        .map(|ws| ws.root.clone());
    session
        .workspaces
        .into_iter()
        .map(|ws| {
            let current = active_root.as_ref().is_some_and(|root| *root == ws.root);
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
        assert_eq!(parse(&[".".into()]).unwrap(), Command::Ensure(".".into()));
        assert_eq!(
            parse(&["workspace".into(), "open".into(), "api".into()]).unwrap(),
            Command::Open("api".into())
        );
        assert!(parse(&["nope".into()]).is_err());
        assert_eq!(
            parse(&["workspace".into(), "forget".into(), "desktop".into()]).unwrap(),
            Command::Forget("desktop".into())
        );
    }

    #[test]
    fn activate_in_session_sets_current_without_clobbering() {
        let mut session = Session::default();
        activate_in_session(&mut session, "a", PathBuf::from("/tmp/a"));
        session.workspaces[0].tabs = vec![];
        activate_in_session(&mut session, "a", PathBuf::from("/tmp/a"));
        assert_eq!(session.workspaces.len(), 1);
        activate_in_session(&mut session, "b", PathBuf::from("/tmp/b"));
        assert_eq!(session.active_workspace, 1);
        assert_eq!(session.workspaces[0].name, "a");
    }

    #[test]
    fn enclosing_workspace_picks_parent() {
        let mut session = Session::default();
        activate_in_session(&mut session, "lolterm", PathBuf::from("/home/dev/lolterm"));
        let nested = PathBuf::from("/home/dev/lolterm/apps/desktop");
        let parent = enclosing_workspace(&session, &nested).expect("parent");
        assert_eq!(parent.name, "lolterm");
        assert!(enclosing_workspace(&session, Path::new("/home/dev/lolterm")).is_none());
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
