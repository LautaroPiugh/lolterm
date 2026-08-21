use std::env;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use crate::VERSION;
use crate::config::{self, AppConfig, Machine, PendingLaunch};
use crate::context::{self, ContextPane, ContextView};
use crate::ctl;
use crate::files;
use crate::git;
use crate::registry;
use crate::session::{self, SavedWorkspace, Session};
use crate::ssh;
use crate::workspaces;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Status,
    WorkspaceList,
    WorkspaceCurrent,
    Ensure(String),
    Open(String),
    Forget(String),
    Ssh(Option<String>),
    Run(Option<String>),
    Launch,
    Context,
    Panes,
    Processes,
    Machines,
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

#[derive(Debug, PartialEq, Eq)]
pub struct PaneRow {
    pub tab: usize,
    pub tab_name: String,
    pub program: String,
    pub cwd: String,
}

pub fn run(args: &[String]) -> Result<i32, String> {
    match parse(args)? {
        Command::Help => {
            print!("{}", help_text());
            Ok(0)
        }
        Command::Launch => {
            let view = load_status();
            print!("{}", format_status(&view));
            handoff_to_desktop(
                PendingLaunch {
                    open: session_active_root(),
                    ..PendingLaunch::default()
                },
                format!("workspace {}", view.workspace),
            )
        }
        Command::Version => {
            println!("{VERSION}");
            Ok(0)
        }
        Command::Status => {
            print!("{}", format_status(&load_status()));
            Ok(0)
        }
        Command::Context => {
            print!("{}", context::format_context(&load_context()));
            Ok(0)
        }
        Command::WorkspaceList => {
            print!("{}", format_workspace_list(&load_workspace_rows()));
            Ok(0)
        }
        Command::WorkspaceCurrent => {
            print!("{}", format_status(&load_status()));
            Ok(0)
        }
        Command::Panes => {
            print!("{}", format_pane_list(&load_pane_rows()));
            Ok(0)
        }
        Command::Processes => {
            print!("{}", format_process_list(&load_process_names()));
            Ok(0)
        }
        Command::Machines => {
            print!("{}", format_machine_list(&config::load().machines));
            Ok(0)
        }
        Command::Ensure(raw) => {
            let root = resolve_dir(&raw)?;
            let view = ensure_workspace(&root)?;
            print!("{}", format_status(&view));
            handoff_to_desktop(
                PendingLaunch {
                    open: session_active_root(),
                    ..PendingLaunch::default()
                },
                format!("workspace {}", view.workspace),
            )
        }
        Command::Open(name) => {
            let view = open_workspace(&name)?;
            print!("{}", format_status(&view));
            handoff_to_desktop(
                PendingLaunch {
                    open: session_active_root(),
                    ..PendingLaunch::default()
                },
                format!("workspace {}", view.workspace),
            )
        }
        Command::Forget(name) => {
            forget_workspace(&name)?;
            print!("{}", format_workspace_list(&load_workspace_rows()));
            Ok(0)
        }
        Command::Ssh(None) => {
            print!("{}", format_machine_list(&config::load().machines));
            Ok(0)
        }
        Command::Ssh(Some(key)) => run_ssh(&key),
        Command::Run(None) => {
            print!("{}", format_run_list());
            Ok(0)
        }
        Command::Run(Some(program)) => run_program(&program),
    }
}

pub fn parse(args: &[String]) -> Result<Command, String> {
    let mut args = args
        .iter()
        .map(String::as_str)
        .filter(|arg| !arg.is_empty());
    let Some(first) = args.next() else {
        return Ok(Command::Launch);
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
        "context" | "ctx" => {
            if args.next().is_some() {
                return Err("context no admite argumentos".into());
            }
            Ok(Command::Context)
        }
        "panes" | "pane" => {
            if args.next().is_some() {
                return Err("panes no admite argumentos".into());
            }
            Ok(Command::Panes)
        }
        "processes" | "procs" => {
            if args.next().is_some() {
                return Err("processes no admite argumentos".into());
            }
            Ok(Command::Processes)
        }
        "machines" => {
            if args.next().is_some() {
                return Err(
                    "machines no admite argumentos; para conectar: lolterm ssh <máquina>".into(),
                );
            }
            Ok(Command::Machines)
        }
        "ssh" => {
            let key = args.next().map(str::to_string);
            if args.next().is_some() {
                return Err("lolterm ssh admite un solo destino".into());
            }
            Ok(Command::Ssh(key))
        }
        "run" => {
            let program = args.next().map(str::to_string);
            if args.next().is_some() {
                return Err("lolterm run admite un solo programa por ahora".into());
            }
            Ok(Command::Run(program))
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
            Some("current") | Some("now") => {
                if args.next().is_some() {
                    return Err("workspace current no admite argumentos".into());
                }
                Ok(Command::WorkspaceCurrent)
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
                "subcomando desconocido: workspace {other}\nprobá: lolterm workspace list | current | open <nombre>"
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
  lolterm
  lolterm .
  lolterm ~/dev/api
  lolterm status
  lolterm context
  lolterm workspace list
  lolterm workspace current
  lolterm workspace open <nombre>
  lolterm workspace forget <nombre>
  lolterm panes
  lolterm processes
  lolterm machines
  lolterm ssh
  lolterm ssh <máquina>
  lolterm run
  lolterm run <programa>
  lolterm -h | --help
  lolterm -V | --version

Sin argumentos abre o enfoca el Desktop en el workspace activo. `context`
imprime JSON en vivo si el Desktop está abierto (si no, la última sesión
guardada). Los panes reciben `LOLTERM_CONTEXT`. Un agente abre en un
git worktree (`LOLTERM_WORKTREE`) para no pisar nvim. Sin secretos ni
valores de env. `.`, `workspace open`, `ssh` y `run` abren el Desktop.
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

pub fn format_pane_list(rows: &[PaneRow]) -> String {
    if rows.is_empty() {
        return "ningún pane (Desktop cerrado y sin layout guardado)\n".into();
    }
    let name_w = rows
        .iter()
        .map(|row| row.tab_name.len())
        .max()
        .unwrap_or(0)
        .min(16);
    let prog_w = rows
        .iter()
        .map(|row| row.program.len())
        .max()
        .unwrap_or(0)
        .min(16);
    let mut out = String::new();
    for row in rows {
        out.push_str(&format!(
            "{tab}  {name:<name_w$}  {prog:<prog_w$}  {cwd}\n",
            tab = row.tab,
            name = truncate_name(&row.tab_name, name_w),
            prog = truncate_name(&row.program, prog_w),
            cwd = row.cwd,
        ));
    }
    out
}

pub fn format_process_list(names: &[String]) -> String {
    if names.is_empty() {
        return "ningún proceso (solo shells, o el Desktop no está abierto)\n".into();
    }
    let mut out = String::new();
    for name in names {
        out.push_str(name);
        out.push('\n');
    }
    out
}

pub struct SshPlan {
    pub dest: String,
    pub args: Vec<String>,
}

pub fn plan_ssh(cfg: &AppConfig, workspace: &str, key: &str) -> Result<SshPlan, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("hace falta una máquina: lolterm ssh <nombre>".into());
    }
    let dest = if let Some(machine) = find_machine(&cfg.machines, key) {
        let dest = machine.dest(cfg.remote.user.as_deref());
        if dest.is_empty() {
            return Err(format!("destino vacío para {key}"));
        }
        dest
    } else if config::machine_target_ok(key) {
        if let Some((user, host)) = key.split_once('@') {
            if !ssh::ssh_user_ok(user) || host.is_empty() {
                return Err(format!("destino inválido: {key}"));
            }
            key.to_string()
        } else {
            match cfg.remote.user.as_deref() {
                Some(user) => format!("{user}@{key}"),
                None => key.to_string(),
            }
        }
    } else {
        return Err(format!("máquina desconocida: {key}\nprobá: lolterm ssh"));
    };
    let tmux = ssh::tmux_session_name(&cfg.remote.tmux, workspace);
    Ok(SshPlan {
        dest: dest.clone(),
        args: ssh::ssh_args(&dest, &tmux),
    })
}

fn find_machine<'a>(machines: &'a [Machine], key: &str) -> Option<&'a Machine> {
    let slug = workspaces::slug(key);
    machines.iter().find(|machine| {
        machine.name == key || machine.target == key || workspaces::slug(&machine.name) == slug
    })
}

pub fn format_machine_list(machines: &[Machine]) -> String {
    if machines.is_empty() {
        return "ninguna máquina (conectá una desde Remoto en el Desktop)\n".into();
    }
    let width = machines
        .iter()
        .map(|machine| machine.name.len())
        .max()
        .unwrap_or(0)
        .min(24);
    let mut out = String::new();
    for machine in machines {
        out.push_str(&format!(
            "{:<width$}  {:<9}  {}\n",
            machine.name,
            machine.kind.as_str(),
            machine.target
        ));
    }
    out
}

fn run_ssh(key: &str) -> Result<i32, String> {
    let cfg = config::load();
    let view = load_status();
    let _plan = plan_ssh(&cfg, &view.workspace, key)?;
    handoff_to_desktop(
        PendingLaunch {
            ssh: Some(key.to_string()),
            ..PendingLaunch::default()
        },
        format!("ssh {key}"),
    )
}

fn run_program(program: &str) -> Result<i32, String> {
    if !files::program_ok(program) {
        return Err(format!("programa inválido: {program}"));
    }
    handoff_to_desktop(
        PendingLaunch {
            run: Some(program.to_string()),
            ..PendingLaunch::default()
        },
        format!("run {program}"),
    )
}

fn format_run_list() -> String {
    let mut out = String::new();
    for tool in registry::TOOLS {
        let name = tool.name;
        let mark = if files::command_on_path(name) {
            "  "
        } else {
            "? "
        };
        out.push_str(&format!("{mark}{name}\n"));
    }
    out.push_str("\nlolterm run nvim  →  abre LoLTerm y lanza el programa en un pane\n");
    out
}

fn session_active_root() -> Option<String> {
    let session = loaded_session();
    session
        .workspaces
        .get(session.active_workspace)
        .map(|ws| ws.root.to_string_lossy().into_owned())
}

fn handoff_to_desktop(pending: PendingLaunch, summary: String) -> Result<i32, String> {
    config::write_pending(&pending)?;
    match launch_desktop() {
        Ok(()) => println!("abriendo LoLTerm → {summary}"),
        Err(err) => {
            eprintln!("{err}");
            eprintln!("la acción quedó en cola: abrí LoLTerm cuando puedas");
        }
    }
    Ok(0)
}

fn desktop_app_root() -> Result<PathBuf, String> {
    let from_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop");
    if from_manifest.join("package.json").is_file() {
        return from_manifest
            .canonicalize()
            .map_err(|err| format!("no pude resolver apps/desktop: {err}"));
    }
    Err("no encontré apps/desktop (compilá lolterm desde el repo)".into())
}

fn vite_dev_up() -> bool {
    let Ok(addr) = "127.0.0.1:5173".parse::<SocketAddr>() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

fn launch_desktop() -> Result<(), String> {
    let app = desktop_app_root()?;
    let electron = app.join("node_modules/.bin/electron");
    if !electron.is_file() {
        return Err(format!(
            "no encontré Electron en {}\ncd apps/desktop && npm install",
            electron.display()
        ));
    }
    let dist = app.join("dist/index.html");
    let mut cmd = std::process::Command::new(&electron);
    cmd.arg(".");
    cmd.current_dir(&app);
    cmd.args(["--no-sandbox"]);
    if vite_dev_up() {
        cmd.env("LOLTERM_DEV", "1");
        cmd.env("LOLTERM_URL", "http://127.0.0.1:5173");
    } else if !dist.is_file() {
        return Err(
            "no hay Vite en :5173 ni apps/desktop/dist. Corré `npm run dev` en apps/desktop".into(),
        );
    }
    if let Ok(exe) = env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let core = dir.join("lolterm-core");
        if core.is_file() {
            cmd.env("LOLTERM_CORE", core);
        }
    }
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    cmd.spawn()
        .map_err(|err| format!("no pude abrir LoLTerm: {err}"))?;
    Ok(())
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

fn load_context() -> ContextView {
    if let Some(value) = ctl::query("context")
        && let Ok(view) = serde_json::from_value::<ContextView>(value)
    {
        return view;
    }
    saved_context()
}

fn saved_context() -> ContextView {
    let session = loaded_session();
    let cfg = config::load();
    let (name, root, processes, panes) = match session.workspaces.get(session.active_workspace) {
        Some(ws) => (
            Some(ws.name.as_str()),
            ws.root.clone(),
            saved_processes(ws),
            saved_panes(ws),
        ),
        None => {
            let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            (None, cwd, Vec::new(), Vec::new())
        }
    };
    let status = status_for(name, &root);
    let env = match session.workspaces.get(session.active_workspace) {
        Some(ws) => context::env_keys_public(ws.env.iter().map(|item| item.key.as_str())),
        None => Vec::new(),
    };
    ContextView {
        version: status.version,
        live: false,
        workspace: status.workspace,
        cwd: status.root,
        machine: "local".into(),
        git: context::ContextGit {
            branch: status.branch,
            remote: git::origin_label(&root),
        },
        tmux: status.tmux_session,
        processes,
        focused_process: panes
            .iter()
            .find(|pane| pane.focused)
            .and_then(|pane| (pane.program != "shell").then(|| pane.program.clone())),
        focused_file: panes
            .iter()
            .find(|pane| pane.focused)
            .and_then(|pane| pane.file.clone()),
        panes,
        env,
        machines: cfg
            .machines
            .iter()
            .map(|machine| machine.name.clone())
            .collect(),
        worktrees: Vec::new(),
        extra: crate::ext::extra_context(&root),
    }
}

fn saved_processes(ws: &SavedWorkspace) -> Vec<String> {
    let mut names = Vec::new();
    for tab in &ws.tabs {
        for spec in tab.tree.leaf_specs() {
            let Some(program) = spec.program.filter(|name| !name.is_empty()) else {
                continue;
            };
            if !names.iter().any(|seen| seen == &program) {
                names.push(program);
            }
        }
    }
    names
}

fn saved_panes(ws: &SavedWorkspace) -> Vec<ContextPane> {
    let mut rows: Vec<ContextPane> = Vec::new();
    for (tab_idx, tab) in ws.tabs.iter().enumerate() {
        let tab_name = tab.name.clone().unwrap_or_else(|| format!("#{tab_idx}"));
        for spec in tab.tree.leaf_specs() {
            let program = spec
                .program
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "shell".into());
            let cwd = spec
                .cwd
                .as_deref()
                .map(workspaces::compact_root)
                .unwrap_or_else(|| workspaces::compact_root(&ws.root));
            let focused = tab_idx == ws.active_tab && !rows.iter().any(|row| row.focused);
            rows.push(ContextPane {
                tab: tab_idx,
                tab_name: tab_name.clone(),
                program,
                cwd,
                remote: None,
                focused,
                worktree: None,
                file: None,
            });
        }
    }
    rows
}

fn load_pane_rows() -> Vec<PaneRow> {
    if let Some(value) = ctl::query("panes")
        && let Ok(rows) = serde_json::from_value::<Vec<ContextPane>>(value)
    {
        return rows.into_iter().map(pane_row_from_context).collect();
    }
    let session = loaded_session();
    let Some(ws) = session.workspaces.get(session.active_workspace) else {
        return Vec::new();
    };
    saved_panes(ws)
        .into_iter()
        .map(pane_row_from_context)
        .collect()
}

fn pane_row_from_context(pane: ContextPane) -> PaneRow {
    PaneRow {
        tab: pane.tab,
        tab_name: pane.tab_name,
        program: pane.program,
        cwd: pane.cwd,
    }
}

fn load_process_names() -> Vec<String> {
    if let Some(value) = ctl::query("processes")
        && let Ok(names) = serde_json::from_value::<Vec<String>>(value)
    {
        return names;
    }
    let session = loaded_session();
    session
        .workspaces
        .get(session.active_workspace)
        .map(saved_processes)
        .unwrap_or_default()
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
        assert_eq!(parse(&[]).unwrap(), Command::Launch);
        assert_eq!(parse(&["status".into()]).unwrap(), Command::Status);
        assert_eq!(parse(&["context".into()]).unwrap(), Command::Context);
        assert_eq!(parse(&["ctx".into()]).unwrap(), Command::Context);
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
        assert_eq!(parse(&["ssh".into()]).unwrap(), Command::Ssh(None));
        assert_eq!(
            parse(&["ssh".into(), "chae".into()]).unwrap(),
            Command::Ssh(Some("chae".into()))
        );
        assert_eq!(parse(&["run".into()]).unwrap(), Command::Run(None));
        assert_eq!(
            parse(&["run".into(), "nvim".into()]).unwrap(),
            Command::Run(Some("nvim".into()))
        );
        assert_eq!(
            parse(&["workspace".into(), "current".into()]).unwrap(),
            Command::WorkspaceCurrent
        );
        assert_eq!(parse(&["panes".into()]).unwrap(), Command::Panes);
        assert_eq!(parse(&["pane".into()]).unwrap(), Command::Panes);
        assert_eq!(parse(&["processes".into()]).unwrap(), Command::Processes);
        assert_eq!(parse(&["procs".into()]).unwrap(), Command::Processes);
        assert_eq!(parse(&["machines".into()]).unwrap(), Command::Machines);
        assert_eq!(
            parse(&["workspace".into(), "forget".into(), "desktop".into()]).unwrap(),
            Command::Forget("desktop".into())
        );
        assert!(parse(&["nope".into()]).is_err());
        assert!(parse(&["workspace".into(), "open".into()]).is_err());
    }

    #[test]
    fn plan_ssh_uses_registry_and_workspace_tmux() {
        let cfg = AppConfig {
            machines: vec![Machine {
                name: "home".into(),
                target: "home.example.ts.net".into(),
                user: Some("dev".into()),
                kind: crate::config::MachineKind::Tailscale,
            }],
            remote: crate::config::RemoteConfig {
                user: Some("dev".into()),
                tmux: "lolterm".into(),
            },
            ..AppConfig::default()
        };
        let plan = plan_ssh(&cfg, "lolterm", "home").unwrap();
        assert_eq!(plan.dest, "dev@home.example.ts.net");
        assert!(
            plan.args
                .last()
                .unwrap()
                .contains("tmux new-session -A -s lolterm-lolterm")
        );
        assert!(!plan.args.iter().any(|arg| arg.contains("password")));
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
    fn format_pane_list_marks_shell_and_program() {
        let text = format_pane_list(&[
            PaneRow {
                tab: 0,
                tab_name: "nvim".into(),
                program: "nvim".into(),
                cwd: "~/Projects/lolterm".into(),
            },
            PaneRow {
                tab: 1,
                tab_name: "#1".into(),
                program: "shell".into(),
                cwd: "~/Projects/lolterm".into(),
            },
        ]);
        assert!(text.contains("nvim"));
        assert!(text.contains("shell"));
        assert!(!text.contains("TOKEN"));
    }

    #[test]
    fn format_pane_list_empty_explains() {
        let text = format_pane_list(&[]);
        assert!(text.contains("ningún pane"));
    }

    #[test]
    fn format_process_list_empty_explains() {
        let text = format_process_list(&[]);
        assert!(text.contains("ningún proceso"));
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
