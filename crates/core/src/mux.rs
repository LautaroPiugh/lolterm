use std::collections::{HashMap, HashSet, VecDeque};
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use portable_pty::PtySize;
use serde::Serialize;

use crate::commands::{self, CommandHit};
use crate::config::{Machine, MachineKind, RemoteConfig};
use crate::files;
use crate::git;
use crate::keys::{self, Binding};
use crate::layout::{LayoutNode, NavDir, SplitDir};
use crate::pty::BytePty;
use crate::session::{self, SavedTab, SavedWorkspace, Session};
use crate::ssh;

pub const RUN_CLIS: &[&str] = &[
    "nvim",
    "lazygit",
    "btop",
    "yazi",
    "fzf",
    "gh",
    "tmux",
    "rg",
    "delta",
    "codex",
    "claude",
    "opencode",
    "hermes",
    "goose",
    "aider",
    "crush",
    "qwen",
    "openhands",
    "agy",
    "cline",
    "copilot",
    "pi",
    "omp",
    "omh",
];

fn is_run_cli(name: &str) -> bool {
    crate::registry::is_known(name) || RUN_CLIS.contains(&name)
}

#[derive(Serialize)]
pub struct Snapshot {
    pub root: PathBuf,
    pub name: String,
    pub branch: Option<String>,
    pub active_tab: usize,
    pub tabs: Vec<TabSnap>,
    pub git: Option<git::Status>,
    pub git_files: Vec<git::WorkingFile>,
    pub git_branches: Vec<String>,
    pub git_log: Vec<String>,
    pub git_worktrees: Vec<git::Worktree>,
    pub tree: Vec<files::TreeRow>,
    pub tailscale: crate::tailscale::Status,
    pub run_clis: Vec<RunCli>,
    pub tools: Vec<crate::registry::ToolInfo>,
    pub http: HttpSnap,
    pub notice: Option<String>,
    pub theme: String,
    pub ssh_user: Option<String>,
    pub ssh_tmux: String,
    pub ssh_tmux_session: String,
    pub keybindings: Vec<Binding>,
    pub version: String,
    pub presets: Vec<crate::presets::Preset>,
    pub workspaces: Vec<WorkspaceSnap>,
    pub active_projects: Vec<ProjectSnap>,
    pub startup: Vec<session::StartupCmd>,
    pub env: Vec<session::EnvVar>,
    pub api_keys: Vec<String>,
    pub meta: ProjectMeta,
    pub machines: Vec<Machine>,
    pub new_tab: String,
    pub agent_worktrees: bool,
    pub agents: Vec<AgentSnap>,
    pub agent_log: Vec<crate::agents::SessionRecord>,
    pub installs: Vec<InstallSnap>,
    pub themes: Vec<crate::ext::ThemePack>,
    pub extensions: Vec<String>,
    pub status_ext: Vec<crate::ext::StatusItem>,
    pub ext_commands: Vec<crate::ext::ExtCommand>,
    pub commands_path: PathBuf,
    pub keybindings_path: PathBuf,
    /// Panes con PTY vivo (workspace actual + estacionados). El renderer
    /// no debe `dispose` estos xterm al cambiar de proyecto.
    pub held_panes: Vec<u64>,
    /// `false` en el primer `ready` (solo chrome/PTYs). `true` cuando git, árbol y CLIs ya están.
    pub booted: bool,
}

#[derive(Serialize)]
pub struct AgentSnap {
    pub program: String,
    pub tab: usize,
    pub tab_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    pub focused: bool,
    pub attention: bool,
}

#[derive(Serialize)]
pub struct ProjectMeta {
    pub stack: Vec<String>,
    pub git_remote: Option<String>,
    pub notes: String,
}

#[derive(Serialize)]
pub struct WorkspaceSnap {
    pub name: String,
    pub root: PathBuf,
    pub root_label: String,
    pub current: bool,
}

/// Un root con trabajo vivo. Puede ser el proyecto mostrado, uno estacionado
/// con PTYs todavía en ejecución, o un worktree usado por un agente.
#[derive(Serialize)]
pub struct ProjectSnap {
    pub name: String,
    pub root: PathBuf,
    pub branch: Option<String>,
    pub current: bool,
    pub tabs: usize,
    pub agents: usize,
}

#[derive(Clone, Serialize)]
pub struct InstallSnap {
    pub pane: u64,
    pub tool: String,
    pub command: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<u32>,
    pub output: String,
}

#[derive(Serialize)]
pub struct TabSnap {
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rel: Option<String>,
    pub focused: u64,
    pub zoomed: Option<u64>,
    pub layout: LayoutNode,
    pub panes: Vec<PaneSnap>,
}

#[derive(Serialize)]
pub struct PaneSnap {
    pub id: u64,
    pub title: String,
    pub program: Option<String>,
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
}

#[derive(Serialize)]
pub struct RunCli {
    pub name: String,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Serialize)]
pub struct HttpSnap {
    pub enabled: bool,
    pub bind: String,
}

struct LivePane {
    title: String,
    program: Option<String>,
    args: Vec<String>,
    pty: BytePty,
    worktree: Option<PathBuf>,
    opened_path: Option<PathBuf>,
    /// Install panes: al terminar el proceso el pane queda visible con todo
    /// el output hasta que el usuario lo cierre manualmente.
    keep_on_exit: bool,
}

#[derive(Clone)]
struct InstallTask {
    pane: u64,
    tool: String,
    command: String,
    state: String,
    exit_code: Option<u32>,
    output: String,
}

struct Tab {
    name: Option<String>,
    kind: String,
    rel: Option<String>,
    focused: u64,
    zoomed: Option<u64>,
    layout: LayoutNode,
    panes: HashMap<u64, LivePane>,
    /// Rename manual: deja de seguir el OSC de nvim. Solo en memoria.
    title_locked: bool,
}

/// Tabs/PTYs de un workspace que no está en pantalla. Siguen vivos
/// hasta cerrar LoLTerm o olvidar el proyecto.
struct ParkedWorkspace {
    tabs: Vec<Tab>,
    active: usize,
    expanded: HashSet<String>,
    name: String,
    startup: Vec<session::StartupCmd>,
    env: Vec<session::EnvVar>,
    notes: String,
}

pub struct Mux {
    next_id: u64,
    root: PathBuf,
    name: String,
    branch: Option<String>,
    tabs: Vec<Tab>,
    active: usize,
    expanded: HashSet<String>,
    tx: Sender<(u64, Vec<u8>)>,
    remote: RemoteConfig,
    recents: Vec<String>,
    recent_projects: Vec<PathBuf>,
    saved_workspaces: Vec<SavedWorkspace>,
    parked: HashMap<PathBuf, ParkedWorkspace>,
    parked_order: VecDeque<PathBuf>,
    startup: Vec<session::StartupCmd>,
    env: Vec<session::EnvVar>,
    notes: String,
    machines: Vec<Machine>,
    notice: Option<String>,
    theme: String,
    new_tab: String,
    agent_worktrees: bool,
    installs: Vec<InstallTask>,
    editor_autowrite: bool,
    ssh_fail: HashMap<String, u8>,
}

impl Mux {
    pub fn boot(open: Option<PathBuf>, tx: Sender<(u64, Vec<u8>)>) -> Result<Self> {
        let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root = open
            .map(|path| canonicalize(&path))
            .unwrap_or_else(|| canonicalize(&start));
        let cfg = crate::config::load();
        let mut mux = Self {
            next_id: 1,
            name: workspace_name(&root),
            branch: git::branch_label(&root),
            root: root.clone(),
            tabs: Vec::new(),
            active: 0,
            expanded: files::default_expanded(),
            tx,
            remote: cfg.remote,
            recents: Vec::new(),
            recent_projects: Vec::new(),
            saved_workspaces: Vec::new(),
            parked: HashMap::new(),
            parked_order: VecDeque::new(),
            startup: Vec::new(),
            env: Vec::new(),
            notes: String::new(),
            machines: cfg.machines,
            notice: None,
            theme: if crate::ext::theme_known(&cfg.theme) {
                cfg.theme
            } else {
                "claro".into()
            },
            new_tab: sanitize_new_tab(&cfg.new_tab),
            agent_worktrees: cfg.agent_worktrees,
            installs: Vec::new(),
            editor_autowrite: cfg.editor_autowrite,
            ssh_fail: HashMap::new(),
        };
        let mut session = session::load().unwrap_or_default();
        crate::workspaces::merge_into_session(&mut session, &crate::workspaces::load());
        mux.recents = session.recents;
        mux.recent_projects = session.recent_projects;
        mux.saved_workspaces = session.workspaces;
        if let Some(ws) = mux
            .saved_workspaces
            .get(
                session
                    .active_workspace
                    .min(mux.saved_workspaces.len().saturating_sub(1)),
            )
            .cloned()
        {
            mux.root = canonicalize(&ws.root);
            mux.name = ws.name.clone();
            mux.branch = git::branch_label(&mux.root);
            mux.startup = ws.startup.clone();
            mux.env = ws.env.clone();
            mux.restore_tabs(&ws.tabs, ws.active_tab)?;
        }
        mux.notes = crate::workspaces::notes_for(&mux.root);
        mux.consume_pending()?;
        mux.apply_startup()?;
        mux.run_hooks("workspace.open")?;
        session::push_unique_path(&mut mux.recent_projects, mux.root.clone(), 12);
        Ok(mux)
    }

    fn restore_tabs(&mut self, saved: &[SavedTab], active: usize) -> Result<()> {
        for tab in saved {
            self.restore_one(tab)?;
        }
        if !self.tabs.is_empty() {
            self.active = active.min(self.tabs.len() - 1);
        }
        Ok(())
    }

    fn restore_one(&mut self, saved: &SavedTab) -> Result<()> {
        let specs = saved.tree.leaf_specs();
        if let Some(spec) = specs.first()
            && let Some(kind) = view_kind(spec.program.as_deref())
        {
            let rel = spec.args.first().cloned().unwrap_or_default();
            self.push_view(kind, &rel, saved.name.clone());
            return Ok(());
        }
        self.tabs.push(Tab {
            name: saved.name.clone(),
            kind: "term".into(),
            rel: None,
            focused: 0,
            zoomed: None,
            layout: LayoutNode::leaf(0),
            panes: HashMap::new(),
            title_locked: false,
        });
        self.active = self.tabs.len() - 1;
        let specs = saved.tree.leaf_specs();
        let mut ids = Vec::new();
        for spec in &specs {
            let cwd = spec.cwd.clone().unwrap_or_else(|| self.root.clone());
            // Conservar el nombre guardado: spawn_argv resuelve PATH de login.
            // Si se degradaba a shell, apply_startup duplicaba nvim/lazygit.
            let program = spec.program.as_deref().filter(|name| !name.is_empty());
            ids.push(self.spawn_pane(&cwd, program, &spec.args)?);
        }
        if ids.is_empty() {
            let root = self.root.clone();
            ids.push(self.spawn_pane(&root, None, &[])?);
        }
        let tab = self
            .tabs
            .get_mut(self.active)
            .ok_or_else(|| eyre!("no tab"))?;
        tab.layout = layout_from_saved(&saved.tree, &ids);
        tab.focused = ids.get(saved.focused).copied().unwrap_or(ids[0]);
        tab.zoomed = saved.zoomed.and_then(|index| ids.get(index).copied());
        Ok(())
    }

    fn spawn_pane(&mut self, cwd: &Path, program: Option<&str>, args: &[String]) -> Result<u64> {
        let (id, live) = self.spawn_live(cwd, program, args)?;
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.panes.insert(id, live);
            self.refresh_live_context();
            return Ok(id);
        }
        let mut panes = HashMap::new();
        panes.insert(id, live);
        self.tabs.push(Tab {
            name: program.map(ToString::to_string),
            kind: "term".into(),
            rel: None,
            focused: id,
            zoomed: None,
            layout: LayoutNode::leaf(id),
            panes,
            title_locked: false,
        });
        self.active = self.tabs.len() - 1;
        self.refresh_live_context();
        Ok(id)
    }

    fn spawn_live(
        &mut self,
        cwd: &Path,
        program: Option<&str>,
        args: &[String],
    ) -> Result<(u64, LivePane)> {
        let stamp = self.next_id;
        let id = self.next_id;
        self.next_id += 1;
        let (cwd, worktree) = self.prepare_agent_cwd(cwd, program, stamp);
        let env = self.context_env(worktree.as_deref(), crate::agents::is_agent(program));
        let (bin, spawn_args) = files::spawn_argv(program, args);
        let spawn_args = if program == Some("ssh") {
            ssh::ensure_alive_opts(&spawn_args)
        } else if program.is_some_and(files::is_vim_family) {
            let mut host = files::nvim_host_args(self.editor_autowrite);
            host.extend(spawn_args);
            host
        } else {
            spawn_args
        };
        let stored_args = if program == Some("ssh") {
            ssh::ensure_alive_opts(args)
        } else {
            args.to_vec()
        };
        let pty = BytePty::spawn(
            id,
            PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
            &cwd,
            bin.as_deref(),
            &spawn_args,
            &env,
            self.tx.clone(),
        )?;
        if crate::agents::is_agent(program) {
            crate::agents::append_session(&crate::agents::SessionRecord {
                ts: stamp,
                workspace: self.name.clone(),
                program: program.unwrap_or("").to_string(),
                worktree: worktree
                    .as_ref()
                    .map(|path| crate::workspaces::compact_root(path)),
            });
        }
        Ok((
            id,
            LivePane {
                title: program
                    .map(|name| name.to_string())
                    .unwrap_or_else(shell_label),
                program: program.map(ToString::to_string),
                args: stored_args,
                pty,
                worktree,
                opened_path: files::editor_file_arg(args).map(PathBuf::from),
                keep_on_exit: false,
            },
        ))
    }

    fn prepare_agent_cwd(
        &mut self,
        cwd: &Path,
        program: Option<&str>,
        stamp: u64,
    ) -> (PathBuf, Option<PathBuf>) {
        if crate::agents::is_our_worktree(cwd) {
            return (cwd.to_path_buf(), Some(cwd.to_path_buf()));
        }
        let Some(name) = program else {
            return (cwd.to_path_buf(), None);
        };
        if !crate::agents::is_agent(Some(name)) {
            return (cwd.to_path_buf(), None);
        }
        if !self.agent_worktrees {
            self.notice = Some(format!("{name} en el directorio real"));
            return (cwd.to_path_buf(), None);
        }
        let Some(repo) = git::toplevel(&self.root) else {
            return (cwd.to_path_buf(), None);
        };
        match crate::agents::open_worktree(&repo, &self.name, name, stamp) {
            Ok(path) => {
                self.notice = Some(format!("{name} en worktree (no pisa nvim)"));
                (path.clone(), Some(path))
            }
            Err(err) => {
                let short = err.lines().next().unwrap_or("error");
                self.notice = Some(format!("{name} sin worktree: {short}"));
                (cwd.to_path_buf(), None)
            }
        }
    }

    fn context_env(&self, worktree: Option<&Path>, agent: bool) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .env
            .iter()
            .filter(|item| session::env_key_ok(&item.key))
            .map(|item| (item.key.clone(), item.value.clone()))
            .collect();
        out.retain(|(key, _)| {
            !matches!(
                key.as_str(),
                "LOLTERM_ROOT"
                    | "LOLTERM_CONTEXT"
                    | "LOLTERM_WORKSPACE"
                    | "LOLTERM_MACHINE"
                    | "LOLTERM_WORKTREE"
            )
        });
        out.push(("LOLTERM_ROOT".into(), self.root.display().to_string()));
        out.push(("LOLTERM_WORKSPACE".into(), self.name.clone()));
        out.push(("LOLTERM_MACHINE".into(), "local".into()));
        let _ = self.context();
        out.push((
            "LOLTERM_CONTEXT".into(),
            crate::context::live_file_path()
                .to_string_lossy()
                .into_owned(),
        ));
        if let Some(path) = worktree {
            out.push((
                "LOLTERM_WORKTREE".into(),
                path.to_string_lossy().into_owned(),
            ));
        }
        if !out.iter().any(|(key, _)| key == "PATH")
            && let Some(path) = files::effective_path()
        {
            out.push(("PATH".into(), path));
        }
        if agent {
            for (key, value) in crate::secrets::all() {
                if key.starts_with("LOLTERM_") || out.iter().any(|(seen, _)| seen == &key) {
                    continue;
                }
                out.push((key, value));
            }
        }
        out
    }

    pub fn snapshot(&self) -> Snapshot {
        self.build_snapshot(true)
    }

    /// Tabs, panes y chrome. Git, árbol y versiones de CLIs van en el
    /// `snapshot` completo (`booted: true`) para no bloquear el primer `ready`.
    pub fn snapshot_shell(&self) -> Snapshot {
        self.build_snapshot(false)
    }

    fn build_snapshot(&self, heavy: bool) -> Snapshot {
        let marks = if heavy {
            git::path_marks(&self.root)
        } else {
            HashMap::new()
        };
        Snapshot {
            root: self.root.clone(),
            name: self.name.clone(),
            branch: self.branch.clone(),
            active_tab: self.active,
            tabs: self
                .tabs
                .iter()
                .map(|tab| TabSnap {
                    name: tab.name.clone().unwrap_or_else(|| tab_label(tab)),
                    kind: tab.kind.clone(),
                    rel: tab.rel.clone(),
                    focused: tab.focused,
                    zoomed: tab.zoomed,
                    layout: tab.layout.clone(),
                    panes: tab
                        .layout
                        .ids()
                        .into_iter()
                        .filter_map(|id| {
                            let pane = tab.panes.get(&id)?;
                            Some(PaneSnap {
                                id,
                                title: pane.title.clone(),
                                program: pane.program.clone(),
                                remote: pane_remote(pane.program.as_deref(), &pane.args),
                                worktree: pane
                                    .worktree
                                    .as_ref()
                                    .map(|path| crate::workspaces::compact_root(path)),
                            })
                        })
                        .collect(),
                })
                .collect(),
            git: heavy.then(|| git::status_cached(&self.root)).flatten(),
            git_files: if heavy {
                git::working_files_cached(&self.root)
            } else {
                Vec::new()
            },
            git_branches: if heavy {
                git::branches_cached(&self.root)
            } else {
                Vec::new()
            },
            git_log: if heavy {
                git::oneline_cached(&self.root, 8)
            } else {
                Vec::new()
            },
            git_worktrees: if heavy {
                git::worktrees_cached(&self.root)
            } else {
                Vec::new()
            },
            tree: if heavy {
                files::visible_tree(&self.root, &self.expanded, &marks)
            } else {
                Vec::new()
            },
            tailscale: if heavy {
                crate::tailscale::probe_cached()
            } else {
                crate::tailscale::Status::Missing
            },
            run_clis: if heavy {
                let running = self.process_names();
                let mut run_clis: Vec<RunCli> = crate::registry::TOOLS
                    .iter()
                    .map(|tool| {
                        let name = tool.name;
                        RunCli {
                            name: name.to_string(),
                            available: files::command_on_path(name)
                                || running.iter().any(|item| item == name),
                            version: crate::registry::version_of(name),
                        }
                    })
                    .collect();
                // Mantiene el orden del catálogo dentro de cada grupo, pero
                // pone primero las CLIs que el usuario puede abrir ahora.
                run_clis.sort_by_key(|cli| !cli.available);
                run_clis
            } else {
                Vec::new()
            },
            tools: if heavy {
                crate::registry::listing()
            } else {
                Vec::new()
            },
            http: self.http_snap(),
            notice: self.notice.clone(),
            theme: self.theme.clone(),
            ssh_user: self.remote.user.clone(),
            ssh_tmux: self.remote.tmux.clone(),
            ssh_tmux_session: self.active_tmux_session(),
            keybindings: keys::load(),
            version: crate::VERSION.to_string(),
            presets: crate::presets::summaries(),
            workspaces: self.workspace_snaps(),
            active_projects: self.active_project_snaps(),
            startup: self.startup.clone(),
            env: self.env.clone(),
            api_keys: crate::secrets::names(),
            meta: ProjectMeta {
                stack: files::detect_stack(&self.root),
                git_remote: heavy.then(|| git::origin_label(&self.root)).flatten(),
                notes: self.notes.clone(),
            },
            machines: self.machines.clone(),
            new_tab: self.new_tab.clone(),
            agent_worktrees: self.agent_worktrees,
            agents: self.agent_snaps(),
            agent_log: if heavy {
                crate::agents::recent_sessions(8)
            } else {
                Vec::new()
            },
            installs: self
                .installs
                .iter()
                .rev()
                .map(|task| InstallSnap {
                    pane: task.pane,
                    tool: task.tool.clone(),
                    command: task.command.clone(),
                    state: task.state.clone(),
                    exit_code: task.exit_code,
                    output: task.output.clone(),
                })
                .collect(),
            themes: crate::ext::all_themes(),
            extensions: crate::ext::load().extensions,
            status_ext: if heavy {
                crate::ext::status_items(&self.root)
            } else {
                Vec::new()
            },
            ext_commands: crate::ext::user_commands(),
            commands_path: crate::ext::commands_path(),
            keybindings_path: keys::keybindings_path(),
            held_panes: self.held_pane_ids(),
            booted: heavy,
        }
    }

    pub fn context(&self) -> crate::context::ContextView {
        let panes = self.pane_rows();
        let focused = panes.iter().find(|pane| pane.focused);
        let cwd = focused
            .map(|pane| pane.cwd.clone())
            .unwrap_or_else(|| crate::workspaces::compact_root(&self.root));
        let machine = focused
            .and_then(|pane| pane.remote.clone())
            .unwrap_or_else(|| "local".into());
        let view = crate::context::ContextView {
            version: crate::VERSION.to_string(),
            live: true,
            workspace: self.name.clone(),
            cwd,
            machine,
            git: crate::context::ContextGit {
                branch: self.branch.clone(),
                remote: git::origin_label(&self.root),
            },
            tmux: self.active_tmux_session(),
            processes: self.process_names(),
            focused_process: focused
                .and_then(|pane| (pane.program != "shell").then(|| pane.program.clone())),
            focused_file: self.focused_opened_file(),
            panes,
            env: crate::context::env_keys_public(self.env.iter().map(|item| item.key.as_str())),
            machines: self.machines.iter().map(|item| item.name.clone()).collect(),
            worktrees: self.agent_worktree_labels(),
            extra: crate::ext::extra_context(&self.root),
        };
        let _ = crate::context::write_live_file(&view);
        view
    }

    pub fn pane_rows(&self) -> Vec<crate::context::ContextPane> {
        let mut rows = Vec::new();
        for (tab_idx, tab) in self.tabs.iter().enumerate() {
            let tab_name = tab.name.clone().unwrap_or_else(|| tab_label(tab));
            for id in tab.layout.ids() {
                let Some(pane) = tab.panes.get(&id) else {
                    continue;
                };
                let program = pane
                    .program
                    .clone()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| "shell".into());
                let cwd = pane
                    .pty
                    .cwd()
                    .map(|path| crate::workspaces::compact_root(&path))
                    .unwrap_or_else(|| crate::workspaces::compact_root(&self.root));
                rows.push(crate::context::ContextPane {
                    tab: tab_idx,
                    tab_name: tab_name.clone(),
                    program,
                    cwd,
                    remote: pane_remote(pane.program.as_deref(), &pane.args),
                    focused: tab_idx == self.active && id == tab.focused,
                    worktree: pane
                        .worktree
                        .as_ref()
                        .map(|path| crate::workspaces::compact_root(path)),
                    file: pane
                        .opened_path
                        .as_ref()
                        .map(|path| crate::workspaces::compact_root(path)),
                });
            }
        }
        rows
    }

    pub fn hud_parts(&self) -> (Vec<String>, Vec<(u64, u32)>, std::path::PathBuf) {
        (self.process_names(), self.pane_pids(), self.root.clone())
    }

    pub fn hud(&self) -> crate::hud::Hud {
        let mut hud = crate::hud::snapshot(&self.process_names());
        hud.extra = crate::inspect::extra(&self.root, &self.pane_pids());
        hud
    }

    fn pane_pids(&self) -> Vec<(u64, u32)> {
        let mut out = collect_pane_pids(&self.tabs);
        for parked in self.parked.values() {
            out.extend(collect_pane_pids(&parked.tabs));
        }
        out
    }

    fn http_snap(&self) -> HttpSnap {
        let cfg = crate::http::load_config();
        HttpSnap {
            enabled: cfg.enabled,
            bind: cfg.bind(),
        }
    }

    fn focused_opened_file(&self) -> Option<String> {
        let tab = self.tabs.get(self.active)?;
        let path = tab.panes.get(&tab.focused)?.opened_path.as_ref()?;
        Some(crate::workspaces::compact_root(path))
    }

    pub fn process_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        collect_process_names(&self.tabs, &mut names);
        for parked in self.parked.values() {
            collect_process_names(&parked.tabs, &mut names);
        }
        names
    }

    fn held_pane_ids(&self) -> Vec<u64> {
        let mut ids = pane_ids_in(&self.tabs);
        for parked in self.parked.values() {
            ids.extend(pane_ids_in(&parked.tabs));
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    fn agent_snaps(&self) -> Vec<AgentSnap> {
        let mut rows = Vec::new();
        for (tab_idx, tab) in self.tabs.iter().enumerate() {
            let tab_name = tab.name.clone().unwrap_or_else(|| tab_label(tab));
            for id in tab.layout.ids() {
                let Some(pane) = tab.panes.get(&id) else {
                    continue;
                };
                if !crate::agents::is_agent(pane.program.as_deref()) {
                    continue;
                }
                rows.push(AgentSnap {
                    program: pane.program.clone().unwrap_or_default(),
                    tab: tab_idx,
                    tab_name: tab_name.clone(),
                    worktree: pane
                        .worktree
                        .as_ref()
                        .map(|path| crate::workspaces::compact_root(path)),
                    focused: tab_idx == self.active && id == tab.focused,
                    attention: pane_needs_attention(&pane.title),
                });
            }
        }
        rows
    }

    fn agent_worktree_labels(&self) -> Vec<String> {
        let mut out = Vec::new();
        for tab in &self.tabs {
            for pane in tab.panes.values() {
                let Some(path) = pane.worktree.as_ref() else {
                    continue;
                };
                let label = crate::workspaces::compact_root(path);
                if !out.iter().any(|seen| seen == &label) {
                    out.push(label);
                }
            }
        }
        out
    }

    fn workspace_snaps(&self) -> Vec<WorkspaceSnap> {
        let mut catalog = crate::workspaces::load();
        crate::workspaces::upsert_def(
            &mut catalog.workspaces,
            crate::workspaces::WorkspaceDef {
                name: self.name.clone(),
                root: crate::workspaces::compact_root(&self.root),
                startup: self.startup.clone(),
                notes: self.notes.clone(),
            },
        );
        catalog
            .workspaces
            .into_iter()
            .map(|def| {
                let root =
                    crate::workspaces::canonical_root(&crate::workspaces::expand_root(&def.root));
                WorkspaceSnap {
                    current: root == self.root,
                    name: if root == self.root {
                        self.name.clone()
                    } else {
                        def.name
                    },
                    root_label: def.root,
                    root,
                }
            })
            .collect()
    }

    fn active_project_snaps(&self) -> Vec<ProjectSnap> {
        let mut projects = Vec::new();
        push_project_snap(&mut projects, &self.root, &self.name, true, &self.tabs);
        for root in self.parked_order.iter().rev() {
            let Some(parked) = self.parked.get(root) else {
                continue;
            };
            push_project_snap(&mut projects, root, &parked.name, false, &parked.tabs);
        }

        // Cada worktree de agente es también un proyecto activo, aunque su PTY
        // pertenezca al workspace que se está mostrando actualmente.
        for tab in &self.tabs {
            push_agent_worktrees(&mut projects, &tab.panes);
        }
        for parked in self.parked.values() {
            for tab in &parked.tabs {
                push_agent_worktrees(&mut projects, &tab.panes);
            }
        }
        projects
    }

    fn capture_workspace(&self) -> SavedWorkspace {
        SavedWorkspace {
            name: self.name.clone(),
            root: self.root.clone(),
            active_tab: self.active,
            startup: self.startup.clone(),
            env: self.env.clone(),
            tabs: self
                .tabs
                .iter()
                .map(|tab| {
                    if tab.kind == "file" || tab.kind == "rest" {
                        let program = if tab.kind == "rest" {
                            "__rest__"
                        } else {
                            "__file__"
                        };
                        return SavedTab {
                            focused: 0,
                            name: tab.name.clone(),
                            zoomed: None,
                            tree: session::SavedNode::Leaf {
                                cwd: Some(self.root.clone()),
                                program: Some(program.into()),
                                args: vec![tab.rel.clone().unwrap_or_default()],
                            },
                        };
                    }
                    let keep: HashSet<u64> = tab.panes.keys().copied().collect();
                    let specs = tab
                        .panes
                        .iter()
                        .map(|(id, pane)| {
                            (
                                *id,
                                session::LeafSpec {
                                    cwd: pane.pty.cwd(),
                                    program: pane.program.clone(),
                                    args: pane.args.clone(),
                                },
                            )
                        })
                        .collect();
                    SavedTab {
                        focused: tab
                            .layout
                            .ids()
                            .iter()
                            .position(|id| *id == tab.focused)
                            .unwrap_or(0),
                        name: tab.name.clone(),
                        zoomed: tab
                            .zoomed
                            .and_then(|id| tab.layout.ids().iter().position(|pane| *pane == id)),
                        tree: session::SavedNode::from_layout(&tab.layout, &keep, &specs),
                    }
                })
                .collect(),
        }
    }

    pub fn set_theme(&mut self, name: &str) -> Result<()> {
        let name = name.trim().to_ascii_lowercase();
        if !crate::ext::theme_known(&name) {
            return Err(eyre!("tema desconocido: {name}"));
        }
        self.theme = name;
        self.write_config()
    }

    pub fn set_new_tab(&mut self, kind: &str) -> Result<()> {
        self.new_tab = sanitize_new_tab(kind);
        self.write_config()
    }

    pub fn set_agent_worktrees(&mut self, enabled: bool) -> Result<()> {
        self.agent_worktrees = enabled;
        self.write_config()
    }

    fn write_config(&self) -> Result<()> {
        let cfg = crate::config::AppConfig {
            theme: self.theme.clone(),
            remote: self.remote.clone(),
            machines: self.machines.clone(),
            new_tab: self.new_tab.clone(),
            agent_worktrees: self.agent_worktrees,
            editor_autowrite: self.editor_autowrite,
        };
        crate::config::save(&cfg).map_err(|err| eyre!("no pude guardar config: {err}"))
    }

    pub fn persist(&self) {
        let mut workspaces = self.saved_workspaces.clone();
        session::upsert_workspace(&mut workspaces, self.capture_workspace(), 12);
        let active_workspace = workspaces
            .iter()
            .position(|ws| ws.root == self.root)
            .unwrap_or(0);
        let session = Session {
            active_workspace,
            workspaces,
            recents: self.recents.clone(),
            recent_projects: self.recent_projects.clone(),
        };
        let _ = session::save(&session);
        let mut catalog = crate::workspaces::catalog_from_saved(&session.workspaces);
        crate::workspaces::upsert_def(
            &mut catalog.workspaces,
            crate::workspaces::WorkspaceDef {
                name: self.name.clone(),
                root: crate::workspaces::compact_root(&self.root),
                startup: self.startup.clone(),
                notes: self.notes.clone(),
            },
        );
        let _ = crate::workspaces::save(&catalog);
    }

    pub fn rename_workspace(&mut self, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            self.notice = Some("el workspace necesita un nombre".into());
            return Ok(());
        }
        self.name = name.to_string();
        self.persist();
        Ok(())
    }

    pub fn set_notes(&mut self, notes: &str) -> Result<()> {
        self.notes = notes.trim().to_string();
        self.persist();
        Ok(())
    }

    pub fn forget_workspace(&mut self, path: &Path) -> Result<()> {
        let root = canonicalize(path);
        if root == self.root {
            self.notice = Some("no se puede quitar el workspace actual".into());
            return Ok(());
        }
        self.saved_workspaces.retain(|ws| ws.root != root);
        self.parked.remove(&root);
        self.parked_order.retain(|path| path != &root);
        let mut catalog = crate::workspaces::load();
        crate::workspaces::remove_root(&mut catalog.workspaces, &root);
        let _ = crate::workspaces::save(&catalog);
        self.persist();
        Ok(())
    }

    fn catalog_roots(&self) -> Vec<PathBuf> {
        let snaps = self.workspace_snaps();
        if snaps.is_empty() {
            vec![self.root.clone()]
        } else {
            snaps.into_iter().map(|item| item.root).collect()
        }
    }

    pub fn cycle_workspace(&mut self, delta: i32) -> Result<()> {
        let roots = self.catalog_roots();
        if roots.len() < 2 {
            self.notice = Some("hace falta otro workspace en el catálogo".into());
            return Ok(());
        }
        let current = roots
            .iter()
            .position(|root| root == &self.root)
            .unwrap_or(0);
        let len = roots.len() as i32;
        let next = (current as i32 + delta).rem_euclid(len) as usize;
        self.open_project(&roots[next])
    }

    pub fn write(&mut self, pane: u64, bytes: &[u8]) -> Result<()> {
        let live = self.pane_mut(pane).ok_or_else(|| eyre!("unknown pane"))?;
        live.pty.write_input(bytes)
    }

    fn pane_mut(&mut self, pane: u64) -> Option<&mut LivePane> {
        for tab in &mut self.tabs {
            if let Some(live) = tab.panes.get_mut(&pane) {
                return Some(live);
            }
        }
        None
    }

    pub fn resize(&mut self, pane: u64, cols: u16, rows: u16) -> Result<()> {
        for tab in &self.tabs {
            if let Some(live) = tab.panes.get(&pane) {
                return live.pty.resize(cols, rows);
            }
        }
        Ok(())
    }

    pub fn focus(&mut self, pane: u64) {
        if let Some(tab) = self.tabs.get_mut(self.active)
            && tab.panes.contains_key(&pane)
        {
            tab.focused = pane;
        }
        self.refresh_live_context();
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
        self.refresh_live_context();
    }

    pub fn cycle_tab(&mut self, delta: i32) {
        let len = self.tabs.len() as i32;
        if len < 2 {
            return;
        }
        self.active = (self.active as i32 + delta).rem_euclid(len) as usize;
        self.refresh_live_context();
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn root_path(&self) -> &Path {
        &self.root
    }

    pub fn new_tab(
        &mut self,
        program: Option<&str>,
        cwd: Option<&Path>,
        args: &[String],
        _shell_ok: bool,
    ) -> Result<u64> {
        if let Some(name) = program
            && !files::program_ok(name)
        {
            self.notice = Some(format!("programa inválido: {name}"));
            return Ok(0);
        }
        let cwd = cwd.unwrap_or(&self.root).to_path_buf();
        let (id, live) = self.spawn_live(&cwd, program, args)?;
        let mut panes = HashMap::new();
        panes.insert(id, live);
        self.tabs.push(Tab {
            name: program.map(ToString::to_string),
            kind: "term".into(),
            rel: None,
            focused: id,
            zoomed: None,
            layout: LayoutNode::leaf(id),
            panes,
            title_locked: false,
        });
        self.active = self.tabs.len() - 1;
        self.refresh_live_context();
        Ok(id)
    }

    pub fn spawn_preferred_tab(&mut self) -> Result<u64> {
        let kind = self.new_tab.clone();
        match kind.as_str() {
            "shell" => self.new_tab(None, None, &[], true),
            "ssh" | "tailscale" => {
                self.notice = Some("Ctrl-Alt-N abre SSH: elegí un host con + o la paleta".into());
                Ok(0)
            }
            name => self.new_tab(Some(name), None, &[], false),
        }
    }

    pub fn close_tab(&mut self, index: usize) -> Result<()> {
        if index >= self.tabs.len() {
            return Ok(());
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active = 0;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if index < self.active {
            self.active -= 1;
        }
        self.persist();
        Ok(())
    }

    pub fn duplicate_tab(&mut self, index: usize) -> Result<()> {
        let Some(src) = self.tabs.get(index) else {
            return Ok(());
        };
        if src.kind != "term" {
            let kind = src.kind.clone();
            let rel = src.rel.clone().unwrap_or_default();
            let name = src.name.clone();
            self.push_view(&kind, &rel, name);
            return Ok(());
        }
        let keep: HashSet<u64> = src.panes.keys().copied().collect();
        let specs = src
            .panes
            .iter()
            .map(|(id, pane)| {
                (
                    *id,
                    session::LeafSpec {
                        cwd: pane.pty.cwd(),
                        program: pane.program.clone(),
                        args: pane.args.clone(),
                    },
                )
            })
            .collect();
        let name = src.name.clone().unwrap_or_else(|| tab_label(src));
        let saved = SavedTab {
            focused: src
                .layout
                .ids()
                .iter()
                .position(|id| *id == src.focused)
                .unwrap_or(0),
            name: Some(format!("{name} copia")),
            zoomed: src
                .zoomed
                .and_then(|id| src.layout.ids().iter().position(|pane| *pane == id)),
            tree: session::SavedNode::from_layout(&src.layout, &keep, &specs),
        };
        self.restore_one(&saved)?;
        self.refresh_live_context();
        Ok(())
    }

    pub fn split(&mut self, dir: SplitDir, program: Option<&str>, args: &[String]) -> Result<u64> {
        if self
            .tabs
            .get(self.active)
            .is_some_and(|tab| tab.kind != "term")
        {
            self.notice = Some("no se puede partir un editor/REST".into());
            return Ok(0);
        }
        if self.tabs.is_empty() {
            return self.new_tab(program, None, args, true);
        }
        let cwd = self.focused_cwd().unwrap_or_else(|| self.root.clone());
        let id = self.spawn_pane(&cwd, program, args)?;
        let tab = self
            .tabs
            .get_mut(self.active)
            .ok_or_else(|| eyre!("no tab"))?;
        let focused = tab.focused;
        if tab.layout.split_pane(focused, dir, id) {
            tab.focused = id;
        }
        self.refresh_live_context();
        Ok(id)
    }

    pub fn set_split(&mut self, first: u64, second: u64, percent: u64) -> bool {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return false;
        };
        tab.layout
            .set_percent(first, second, percent.min(u16::MAX as u64) as u16)
    }

    pub fn close_pane(&mut self, pane: u64) -> Result<()> {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return Ok(());
        };
        if tab.panes.len() <= 1 {
            return self.close_tab(self.active);
        }
        let ended = tab
            .panes
            .get(&pane)
            .and_then(|live| live.program.clone())
            .filter(|name| crate::agents::is_agent(Some(name)));
        tab.panes.remove(&pane);
        tab.layout.remove_pane(pane);
        tab.focused = tab.layout.first_leaf().unwrap_or(pane);
        if tab.zoomed == Some(pane) {
            tab.zoomed = None;
        }
        if let Some(name) = ended {
            self.notice = Some(format!("{name} cerró"));
            self.refresh_live_context();
        }
        self.refresh_live_context();
        Ok(())
    }

    pub fn run(&mut self, program: &str, args: &[String]) -> Result<u64> {
        if !files::program_ok(program) {
            self.notice = Some(format!("programa inválido: {program}"));
            return Ok(0);
        }
        if wants_own_tab(program) {
            self.new_tab(Some(program), None, args, false)
        } else {
            self.split(SplitDir::Columns, Some(program), args)
        }
    }

    pub fn open_file(&mut self, rel: &str) -> Result<u64> {
        let path = files::join_root(&self.root, rel);
        if path.is_dir() {
            if rel.is_empty() || !self.expanded.remove(rel) {
                self.expanded.insert(rel.to_string());
            }
            return Ok(0);
        }
        if crate::rest::looks_like(rel) {
            self.push_view("rest", rel, None);
            return Ok(0);
        }
        self.push_view("file", rel, None);
        Ok(0)
    }

    pub fn open_in_nvim(&mut self, rel: &str) -> Result<u64> {
        let path = files::confined(&self.root, rel).map_err(|err| eyre!(err))?;
        if path.is_dir() {
            return Ok(0);
        }
        self.open_abs(&path)
    }

    pub fn open_config(&mut self, which: &str) -> Result<u64> {
        let path = match which {
            "keybindings" => keys::keybindings_path(),
            _ => crate::ext::commands_path(),
        };
        if !path.exists() {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let stub = if which == "keybindings" {
                "# Atajos. Vacío desactiva el default.\n[keys]\n"
            } else {
                "# Comandos custom (ext.<slug>).\n# [[command]]\n# id = \"ext.htop\"\n# slash = \"htop\"\n# hint = \"monitor\"\n# run = \"htop\"\n"
            };
            std::fs::write(&path, stub)?;
        }
        self.open_abs(&path)
    }

    fn open_abs(&mut self, path: &Path) -> Result<u64> {
        let abs = canonicalize(path);
        if let Some((tab_index, pane)) = self.find_open_file(&abs) {
            self.select_tab(tab_index);
            if let Some(tab) = self.tabs.get_mut(self.active) {
                tab.focused = pane;
            }
            self.refresh_live_context();
            return Ok(pane);
        }
        let Some((editor, extra)) = files::editor() else {
            self.notice = Some("no hay $EDITOR / nvim".into());
            return Ok(0);
        };
        let mut args = extra;
        args.push(abs.display().to_string());
        let label = files::file_tab_name(&abs);
        let id = self.new_tab(Some(&editor), None, &args, false)?;
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.name = Some(label.clone());
            tab.title_locked = false;
            if let Some(pane) = tab.panes.get_mut(&id) {
                pane.title = label;
                pane.opened_path = Some(abs);
            }
        }
        Ok(id)
    }

    fn push_view(&mut self, kind: &str, rel: &str, name: Option<String>) {
        let rel = rel.trim().trim_start_matches('/').to_string();
        if let Some((index, _)) = self
            .tabs
            .iter()
            .enumerate()
            .find(|(_, tab)| tab.kind == kind && tab.rel.as_deref() == Some(rel.as_str()))
        {
            self.active = index;
            return;
        }
        let label = name.unwrap_or_else(|| {
            Path::new(&rel)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(kind)
                .to_string()
        });
        self.tabs.push(Tab {
            name: Some(label),
            kind: kind.into(),
            rel: Some(rel),
            focused: 0,
            zoomed: None,
            layout: LayoutNode::leaf(0),
            panes: HashMap::new(),
            title_locked: true,
        });
        self.active = self.tabs.len() - 1;
    }

    pub fn read_file(&self, rel: &str) -> Result<String, String> {
        Ok(files::read_text(&self.root, rel)?.text)
    }

    pub fn write_file(&mut self, rel: &str, text: &str) -> Result<(), String> {
        files::write_text(&self.root, rel, text)?;
        git::invalidate_cache();
        self.notice = Some(format!("guardado {rel}"));
        Ok(())
    }

    /// Crear / renombrar / borrar dentro del workspace. `rel` es el padre (create)
    /// o el path actual (rename/delete).
    pub fn fs_op(&mut self, op: &str, rel: &str, name: Option<&str>) -> Result<(), String> {
        match op {
            "createFile" | "createDir" => {
                let name = name.ok_or_else(|| "falta nombre".to_string())?;
                let created = files::create_entry(&self.root, rel, name, op == "createDir")?;
                if !rel.is_empty() {
                    self.expanded.insert(rel.to_string());
                }
                git::invalidate_cache();
                self.notice = Some(format!("creado {created}"));
                if op == "createFile" {
                    let _ = self.open_file(&created);
                }
                Ok(())
            }
            "rename" => {
                let name = name.ok_or_else(|| "falta nombre".to_string())?;
                let dest = files::rename_entry(&self.root, rel, name)?;
                self.retarget_docs(rel, Some(&dest));
                git::invalidate_cache();
                self.notice = Some(format!("{rel} → {dest}"));
                Ok(())
            }
            "delete" => {
                files::delete_entry(&self.root, rel)?;
                self.retarget_docs(rel, None);
                git::invalidate_cache();
                self.notice = Some(format!("borrado {rel}"));
                Ok(())
            }
            "refresh" => {
                git::invalidate_cache();
                self.notice = Some("árbol actualizado".into());
                Ok(())
            }
            _ => Err("operación desconocida".into()),
        }
    }

    fn retarget_docs(&mut self, from: &str, to: Option<&str>) {
        let mut expanded = HashSet::new();
        for rel in &self.expanded {
            if !files::under_rel(rel, from) {
                expanded.insert(rel.clone());
                continue;
            }
            if let Some(dest) = to {
                let mapped = if rel == from {
                    dest.to_string()
                } else {
                    format!("{dest}{}", &rel[from.len()..])
                };
                expanded.insert(mapped);
            }
        }
        self.expanded = expanded;
        let mut drop = Vec::new();
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            if tab.kind != "file" && tab.kind != "rest" {
                continue;
            }
            let Some(rel) = tab.rel.as_deref() else {
                continue;
            };
            if !files::under_rel(rel, from) {
                continue;
            }
            match to {
                None => drop.push(index),
                Some(dest) => {
                    let next = if rel == from {
                        dest.to_string()
                    } else {
                        format!("{dest}{}", &rel[from.len()..])
                    };
                    if rel == from {
                        tab.name = Some(files::file_tab_name(Path::new(&next)));
                    }
                    tab.rel = Some(next);
                }
            }
        }
        for index in drop.into_iter().rev() {
            let _ = self.close_tab(index);
        }
    }

    pub fn git_op(
        &mut self,
        op: &str,
        path: Option<&str>,
        message: Option<&str>,
    ) -> Result<(), String> {
        git::run_op(&self.root, op, path, message)?;
        git::invalidate_cache();
        self.branch = git::branch_label(&self.root);
        self.notice = Some(format!("git {op}"));
        Ok(())
    }

    pub fn rest_send(&self, rel: &str) -> Result<crate::rest::RestResult, String> {
        let text = files::read_text(&self.root, rel)?.text;
        let env_text = files::read_text(&self.root, ".env")
            .map(|file| file.text)
            .unwrap_or_default();
        let pairs = crate::rest::dotenv_pairs(&env_text);
        let env: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        crate::rest::send(&text, &env)
    }

    pub fn install_agent(&mut self, name: &str) -> Result<u64> {
        let cmd = crate::registry::install_cmd(name)
            .ok_or_else(|| eyre!("herramienta desconocida: {name}"))?;
        crate::registry::invalidate();
        self.notice = Some(format!("instalando {name}"));
        let id = self.new_tab(Some("bash"), None, &["-lc".into(), cmd.clone()], false)?;
        if let Some(tab) = self.tabs.last_mut() {
            tab.name = Some(format!("install: {name}"));
            if let Some(pane) = tab.panes.get_mut(&id) {
                pane.keep_on_exit = true;
            }
        }
        self.installs.push(InstallTask {
            pane: id,
            tool: name.to_string(),
            command: cmd,
            state: "running".into(),
            exit_code: None,
            output: String::new(),
        });
        if self.installs.len() > 12 {
            self.installs.remove(0);
        }
        Ok(id)
    }

    pub fn record_output(&mut self, pane: u64, bytes: &[u8]) {
        let Some(task) = self.installs.iter_mut().find(|task| task.pane == pane) else {
            return;
        };
        task.output.push_str(&String::from_utf8_lossy(bytes));
        const OUTPUT_LIMIT: usize = 24_000;
        if task.output.len() > OUTPUT_LIMIT {
            let start = task.output.len() - OUTPUT_LIMIT;
            let start = task
                .output
                .char_indices()
                .find_map(|(index, _)| (index >= start).then_some(index))
                .unwrap_or(0);
            task.output.drain(..start);
        }
    }

    pub fn set_http(&mut self, enabled: bool, password: Option<&str>) -> Result<(), String> {
        if enabled {
            crate::http::set_password(password.unwrap_or(""))?;
        } else if let Some(password) = password.filter(|p| !p.is_empty()) {
            crate::http::set_password(password)?;
        }
        let mut text = std::fs::read_to_string(crate::config::config_path()).unwrap_or_default();
        if !text.contains("[http]") {
            text.push_str("\n[http]\nenabled = false\nhost = \"127.0.0.1\"\nport = 47832\n");
        }
        let enabled_line = if enabled {
            "enabled = true"
        } else {
            "enabled = false"
        };
        let mut out = String::new();
        let mut in_http = false;
        for line in text.lines() {
            if line.trim() == "[http]" {
                in_http = true;
                out.push_str(line);
                out.push('\n');
                continue;
            }
            if line.trim().starts_with('[') {
                in_http = false;
            }
            if in_http && line.trim().starts_with("enabled") {
                out.push_str(enabled_line);
                out.push('\n');
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        std::fs::write(crate::config::config_path(), out).map_err(|err| err.to_string())?;
        self.notice = Some(if enabled {
            "HTTP LAN on · reiniciá LoLTerm para escuchar".into()
        } else {
            "HTTP LAN off".into()
        });
        Ok(())
    }

    fn find_open_file(&self, path: &Path) -> Option<(usize, u64)> {
        for (index, tab) in self.tabs.iter().enumerate() {
            for (id, pane) in &tab.panes {
                if files::pane_holds_file(pane.opened_path.as_deref(), &pane.args, path) {
                    return Some((index, *id));
                }
            }
        }
        None
    }

    pub fn set_pane_title(&mut self, pane: u64, title: &str) {
        let title = files::sanitize_pane_title(title);
        if title.is_empty() {
            return;
        }
        for tab in &mut self.tabs {
            if let Some(live) = tab.panes.get_mut(&pane) {
                live.title = title.clone();
                if !tab.title_locked {
                    tab.name = Some(title);
                }
                return;
            }
        }
    }

    pub fn save_ext_command(&mut self, draft: crate::ext::CommandDraft) -> Result<()> {
        match crate::ext::upsert_user_command(draft) {
            Ok(cmd) => {
                self.notice = Some(format!("comando /{} guardado", cmd.slash));
            }
            Err(err) => {
                self.notice = Some(err);
            }
        }
        Ok(())
    }

    pub fn remove_ext_command(&mut self, id: &str) -> Result<()> {
        match crate::ext::remove_user_command(id) {
            Ok(()) => self.notice = Some("comando quitado".into()),
            Err(err) => self.notice = Some(err),
        }
        Ok(())
    }

    pub fn set_keybinding(&mut self, chord: &str, command: &str) -> Result<()> {
        keys::apply(chord, command)?;
        self.notice = if command.trim().is_empty() {
            Some("atajo quitado".into())
        } else {
            Some(format!("atajo {chord}"))
        };
        Ok(())
    }

    pub fn reset_keybindings(&mut self) -> Result<()> {
        keys::reset()?;
        self.notice = Some("atajos por defecto".into());
        Ok(())
    }

    pub fn toggle_expand(&mut self, rel: &str) {
        if !self.expanded.remove(rel) {
            self.expanded.insert(rel.to_string());
        }
    }

    pub fn open_project(&mut self, path: &Path) -> Result<()> {
        git::invalidate_cache();
        let next = canonicalize(path);
        if !next.is_dir() {
            return Err(eyre!("workspace no es directorio: {}", next.display()));
        }
        if next == self.root && !self.tabs.is_empty() {
            self.notice = Some(format!(
                "{} ya es el workspace activo",
                crate::workspaces::compact_root(&self.root)
            ));
            return Ok(());
        }
        self.stash_current();
        if let Some(live) = self.take_parked(&next) {
            self.root = next;
            self.name = live.name;
            self.startup = live.startup;
            self.env = live.env;
            self.notes = live.notes;
            self.expanded = live.expanded;
            self.tabs = live.tabs;
            self.active = live.active.min(self.tabs.len().saturating_sub(1));
            self.branch = git::branch_label(&self.root);
            self.apply_startup()?;
            self.refresh_live_context();
            session::push_unique_path(&mut self.recent_projects, self.root.clone(), 12);
            self.persist();
            return Ok(());
        }
        self.tabs.clear();
        self.root = next;
        self.name = workspace_name(&self.root);
        self.branch = git::branch_label(&self.root);
        self.expanded = files::default_expanded();
        let saved = self
            .saved_workspaces
            .iter()
            .find(|ws| ws.root == self.root)
            .cloned();
        if let Some(ws) = saved {
            self.name = ws.name;
            self.startup = ws.startup;
            self.env = ws.env;
            self.restore_tabs(&ws.tabs, ws.active_tab)?;
        } else {
            self.startup.clear();
            self.env.clear();
        }
        self.notes = crate::workspaces::notes_for(&self.root);
        self.apply_startup()?;
        self.refresh_live_context();
        self.run_hooks("workspace.open")?;
        session::push_unique_path(&mut self.recent_projects, self.root.clone(), 12);
        self.persist();
        Ok(())
    }

    fn stash_current(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let snapshot = self.capture_workspace();
        session::upsert_workspace(&mut self.saved_workspaces, snapshot, 12);
        let root = self.root.clone();
        self.parked_order.retain(|path| path != &root);
        self.parked.insert(
            root.clone(),
            ParkedWorkspace {
                tabs: mem::take(&mut self.tabs),
                active: self.active,
                expanded: mem::take(&mut self.expanded),
                name: self.name.clone(),
                startup: self.startup.clone(),
                env: self.env.clone(),
                notes: self.notes.clone(),
            },
        );
        self.parked_order.push_back(root);
        cap_parked(&mut self.parked, &mut self.parked_order, 12);
        self.active = 0;
    }

    fn take_parked(&mut self, root: &Path) -> Option<ParkedWorkspace> {
        let live = self.parked.remove(root)?;
        self.parked_order.retain(|path| path != root);
        Some(live)
    }

    fn has_program(&self, program: &str) -> bool {
        startup_already_open(
            self.tabs
                .iter()
                .flat_map(|tab| tab.panes.values().map(|pane| pane.program.as_deref())),
            program,
        )
    }

    pub fn apply_startup(&mut self) -> Result<()> {
        let cmds = self.startup.clone();
        for cmd in cmds {
            if !startup_needed(
                self.tabs
                    .iter()
                    .flat_map(|tab| tab.panes.values().map(|pane| pane.program.as_deref())),
                &cmd.program,
            ) {
                continue;
            }
            self.run(&cmd.program, &cmd.args)?;
        }
        Ok(())
    }

    fn run_hooks(&mut self, event: &str) -> Result<()> {
        for hook in crate::ext::hooks_for(event) {
            if self.has_program(&hook.run) {
                continue;
            }
            self.run(&hook.run, &hook.args)?;
        }
        Ok(())
    }

    pub fn add_startup(&mut self, program: &str, args: &[String]) -> Result<()> {
        let program = program.trim();
        if program.is_empty() {
            return Ok(());
        }
        if !self
            .startup
            .iter()
            .any(|cmd| cmd.program == program && cmd.args == args)
        {
            self.startup.push(session::StartupCmd {
                program: program.to_string(),
                args: args.to_vec(),
            });
        }
        if !self.has_program(program) {
            self.run(program, args)?;
        }
        self.persist();
        Ok(())
    }

    pub fn remove_startup(&mut self, program: &str) -> Result<()> {
        self.startup.retain(|cmd| cmd.program != program);
        self.persist();
        Ok(())
    }

    pub fn set_env(&mut self, key: &str, value: &str) -> Result<()> {
        let key = key.trim();
        if !session::env_key_ok(key) {
            self.notice = Some("nombre de variable inválido (letras, números y _)".into());
            return Ok(());
        }
        if crate::context::looks_secret(key) {
            self.notice = Some("esa variable parece secreta; usá .env local o el shell".into());
            return Ok(());
        }
        if let Some(existing) = self.env.iter_mut().find(|item| item.key == key) {
            existing.value = value.to_string();
        } else {
            self.env.push(session::EnvVar {
                key: key.to_string(),
                value: value.to_string(),
            });
        }
        self.persist();
        Ok(())
    }

    pub fn remove_env(&mut self, key: &str) -> Result<()> {
        self.env.retain(|item| item.key != key);
        self.persist();
        Ok(())
    }

    pub fn set_api_key(&self, key: &str, value: &str) -> Result<()> {
        crate::secrets::set(key, value).map_err(|err| eyre!(err))
    }

    pub fn remove_api_key(&self, key: &str) -> Result<()> {
        crate::secrets::remove(key).map_err(|err| eyre!(err))
    }

    pub fn ssh(&mut self, dest: &str) -> Result<u64> {
        let dest = dest.trim();
        if dest.is_empty() {
            self.notice = Some("hace falta user@host".into());
            return Ok(0);
        }
        let (user, host) = match dest.split_once('@') {
            Some((user, host)) => (Some(user), host),
            None => (None, dest),
        };
        if let Some(user) = user.filter(|name| ssh::ssh_user_ok(name)) {
            self.remember_ssh_user(user);
        }
        self.remember_connected(
            crate::config::host_label(host),
            host.to_string(),
            MachineKind::Ssh,
            user.filter(|name| ssh::ssh_user_ok(name))
                .map(str::to_string),
        );
        session::push_unique(&mut self.recents, dest.to_string(), 12);
        let id = self.new_tab(
            Some("ssh"),
            None,
            &ssh::ssh_args(dest, &self.active_tmux_session()),
            false,
        )?;
        self.name_active_tab(tab_name_from_dest(dest));
        self.refresh_live_context();
        Ok(id)
    }

    pub fn ts_ssh(&mut self, target: &str, user: Option<&str>) -> Result<u64> {
        let user = user
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToString::to_string)
            .or_else(|| self.remote.user.clone());
        let Some(user) = user else {
            self.notice = Some("hace falta un usuario ssh".into());
            return Ok(0);
        };
        if !ssh::ssh_user_ok(&user) {
            self.notice = Some("usuario ssh inválido".into());
            return Ok(0);
        }
        self.remember_ssh_user(&user);
        let args = ssh::ts_ssh_args(target, Some(&user), &self.active_tmux_session());
        let dest = ssh::ts_ssh_dest(target, Some(&user));
        if dest.is_empty() {
            self.notice = Some("máquina Tailscale vacía".into());
            return Ok(0);
        }
        let host = dest.rsplit_once('@').map(|(_, host)| host).unwrap_or(&dest);
        self.remember_connected(
            crate::config::host_label(host),
            host.to_string(),
            MachineKind::Tailscale,
            Some(user.clone()),
        );
        session::push_unique(&mut self.recents, dest.clone(), 12);
        let id = self.new_tab(Some("ssh"), None, &args, false)?;
        self.name_active_tab(tab_name_from_dest(&dest));
        self.refresh_live_context();
        Ok(id)
    }

    fn remember_connected(
        &mut self,
        name: String,
        target: String,
        kind: MachineKind,
        user: Option<String>,
    ) {
        if name.is_empty() || !crate::config::machine_target_ok(&target) {
            return;
        }
        crate::config::upsert_machine(
            &mut self.machines,
            Machine {
                name,
                target,
                user,
                kind,
            },
        );
        self.persist_machines();
    }

    pub fn set_remote_tmux(&mut self, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            self.remote.tmux.clear();
            self.persist_machines();
            return Ok(());
        }
        if !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        {
            self.notice = Some("prefijo tmux inválido (letras, números, - _)".into());
            return Ok(());
        }
        self.remote.tmux = name.to_string();
        self.persist_machines();
        Ok(())
    }

    fn active_tmux_session(&self) -> String {
        ssh::tmux_session_name(&self.remote.tmux, &self.name)
    }

    fn persist_machines(&self) {
        let _ = self.write_config();
    }

    fn remember_ssh_user(&mut self, user: &str) {
        self.remote.user = Some(user.to_string());
        self.persist_machines();
    }

    pub fn add_machine(
        &mut self,
        name: &str,
        target: &str,
        kind: &str,
        user: Option<&str>,
    ) -> Result<()> {
        let name = name.trim();
        let target = target.trim();
        if name.is_empty() {
            self.notice = Some("la máquina necesita un nombre".into());
            return Ok(());
        }
        if !crate::config::machine_target_ok(target) {
            self.notice = Some("destino inválido (host, alias o user@host)".into());
            return Ok(());
        }
        let kind = MachineKind::parse(kind).unwrap_or(MachineKind::Ssh);
        let user = user
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        if let Some(user) = user.as_deref()
            && !ssh::ssh_user_ok(user)
        {
            self.notice = Some("usuario ssh inválido".into());
            return Ok(());
        }
        let machine = Machine {
            name: name.to_string(),
            target: target.to_string(),
            user,
            kind,
        };
        crate::config::upsert_machine(&mut self.machines, machine);
        self.persist_machines();
        Ok(())
    }

    pub fn forget_machine(&mut self, target: &str) -> Result<()> {
        let target = target.trim();
        self.machines
            .retain(|item| item.target != target && item.name != target);
        self.persist_machines();
        Ok(())
    }

    /// Consume `pending.toml` de la CLI y aplica open / ssh / run.
    pub fn consume_pending(&mut self) -> Result<()> {
        let Some(pending) = crate::config::take_pending() else {
            return Ok(());
        };
        if let Some(path) = pending
            .open
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            let path = PathBuf::from(path);
            if path.is_dir() {
                self.open_project(&path)?;
            } else {
                self.notice = Some(format!("path inválido: {}", path.display()));
            }
        }
        if let Some(key) = pending
            .ssh
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            let slug = crate::workspaces::slug(key);
            if self.machines.iter().any(|item| {
                item.name == key
                    || item.target == key
                    || crate::workspaces::slug(&item.name) == slug
            }) {
                self.connect_machine(key, None)?;
            } else if crate::config::machine_target_ok(key) {
                self.ssh(key)?;
            } else {
                self.notice = Some(format!("máquina desconocida: {key}"));
            }
        }
        if let Some(program) = pending
            .run
            .as_deref()
            .map(str::trim)
            .filter(|name| files::program_ok(name))
        {
            self.run(program, &[])?;
        }
        Ok(())
    }

    pub fn connect_machine(&mut self, key: &str, user: Option<&str>) -> Result<u64> {
        let key = key.trim();
        let slug = crate::workspaces::slug(key);
        let Some(machine) = self
            .machines
            .iter()
            .find(|item| {
                item.name == key
                    || item.target == key
                    || crate::workspaces::slug(&item.name) == slug
            })
            .cloned()
        else {
            self.notice = Some(format!("máquina desconocida: {key}"));
            return Ok(0);
        };
        let user = user
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .or(machine.user.as_deref());
        match machine.kind {
            MachineKind::Tailscale => self.ts_ssh(&machine.target, user),
            MachineKind::Ssh => {
                let dest = if let Some(user) = user.filter(|_| !machine.target.contains('@')) {
                    format!("{user}@{}", machine.target)
                } else {
                    machine.dest(self.remote.user.as_deref())
                };
                self.ssh(&dest)
            }
        }
    }

    fn name_active_tab(&mut self, name: String) {
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.name = Some(name);
        }
    }

    pub fn search_files(&self, query: &str) -> Vec<files::FileEntry> {
        files::filter_files(&files::list_files(&self.root), query)
    }

    pub fn ssh_hosts(&self) -> Vec<ssh::HostItem> {
        let mut hosts: Vec<ssh::HostItem> = self
            .recents
            .iter()
            .map(|dest| ssh::HostItem {
                name: dest.clone(),
                target: dest.clone(),
                hint: "reciente".into(),
            })
            .collect();
        for name in ssh::ssh_config_hosts() {
            if hosts.iter().any(|host| host.target == name) {
                continue;
            }
            hosts.push(ssh::HostItem {
                name: name.clone(),
                target: name,
                hint: "config".into(),
            });
        }
        hosts
    }

    pub fn ts_peers(&self) -> Vec<crate::tailscale::Peer> {
        crate::tailscale::peers()
    }

    pub fn recent_projects(&self) -> Vec<PathBuf> {
        self.recent_projects.clone()
    }

    pub fn commands(&self, query: &str) -> Vec<CommandHit> {
        let mut hits = commands::search(query);
        let needle = query.trim().trim_start_matches('/');
        for preset in crate::presets::summaries() {
            let slash = format!("preset-{}", preset.id);
            let id = format!("layout.preset.{}", preset.id);
            let matches = needle.is_empty()
                || slash.contains(needle)
                || preset.id.contains(needle)
                || preset
                    .name
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase())
                || preset
                    .hint
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase());
            if matches {
                hits.push(CommandHit {
                    id,
                    slash,
                    hint: preset.hint,
                });
            }
        }
        let mut used_slugs = HashSet::new();
        for ws in self.workspace_snaps() {
            if ws.current {
                continue;
            }
            let mut slash = format!("ws-{}", crate::workspaces::slug(&ws.name));
            if !used_slugs.insert(slash.clone()) {
                slash = format!("{slash}-2");
                used_slugs.insert(slash.clone());
            }
            let label = ws.root_label.clone();
            let matches = needle.is_empty()
                || slash.contains(needle)
                || ws
                    .name
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase())
                || label.contains(needle);
            if matches {
                hits.push(CommandHit {
                    id: format!("workspace.open:{}", ws.root.display()),
                    slash,
                    hint: format!("abrir {}", ws.name),
                });
            }
        }
        let mut used_machines = HashSet::new();
        for machine in &self.machines {
            let mut slash = format!("m-{}", crate::workspaces::slug(&machine.name));
            if !used_machines.insert(slash.clone()) {
                slash = format!("{slash}-2");
                used_machines.insert(slash.clone());
            }
            let matches = needle.is_empty()
                || slash.contains(needle)
                || machine
                    .name
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase())
                || machine.target.contains(needle);
            if matches {
                hits.push(CommandHit {
                    id: format!("machine.open:{}", machine.target),
                    slash,
                    hint: format!("ssh {}", machine.name),
                });
            }
        }
        for cmd in crate::ext::load().commands {
            let matches = needle.is_empty()
                || cmd.id.contains(needle)
                || cmd.slash.contains(needle)
                || cmd
                    .hint
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase());
            if matches {
                hits.push(CommandHit {
                    id: cmd.id,
                    slash: cmd.slash,
                    hint: if cmd.hint.is_empty() {
                        format!("extensión: {}", cmd.run)
                    } else {
                        cmd.hint
                    },
                });
            }
        }
        hits
    }

    pub fn apply_preset(&mut self, id: &str) -> Result<()> {
        let Some(preset) = crate::presets::get(id) else {
            self.notice = Some(format!("preset desconocido: {id}"));
            return Ok(());
        };
        self.restore_one(&preset.tab)
    }

    pub fn toggle_zoom(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        tab.zoomed = match tab.zoomed {
            Some(id) if id == tab.focused => None,
            _ => Some(tab.focused),
        };
    }

    pub fn swap_nav(&mut self, dir: NavDir) {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        let Some(other) = tab.layout.neighbor(tab.focused, dir) else {
            return;
        };
        tab.layout.swap_ids(tab.focused, other);
    }

    pub fn focus_nav(&mut self, dir: NavDir) {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return;
        };
        let Some(next) = tab.layout.neighbor(tab.focused, dir) else {
            return;
        };
        tab.focused = next;
        if tab.zoomed.is_some() {
            tab.zoomed = Some(next);
        }
        self.refresh_live_context();
    }

    pub fn rename_tab(&mut self, index: usize, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.name = Some(name.to_string());
            tab.title_locked = true;
        }
    }

    pub fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        if self.active == from {
            self.active = to;
        } else if from < self.active && to >= self.active {
            self.active -= 1;
        } else if from > self.active && to <= self.active {
            self.active += 1;
        }
    }

    pub fn dock_tab(&mut self, from: usize, edge: NavDir) {
        if self.tabs.len() < 2 || from >= self.tabs.len() || from == self.active {
            return;
        }
        let incoming = self.tabs.remove(from);
        if from < self.active {
            self.active -= 1;
        }
        let focused = incoming.focused;
        let Some(dest) = self.tabs.get_mut(self.active) else {
            return;
        };
        dest.panes.extend(incoming.panes);
        dest.layout.wrap(incoming.layout, edge);
        dest.focused = focused;
        dest.zoomed = None;
        self.persist();
    }

    pub fn restart_pane(&mut self, pane: u64) -> Result<()> {
        let (cwd, program, args) = {
            let tab = self.tabs.get(self.active).ok_or_else(|| eyre!("no tab"))?;
            let live = tab.panes.get(&pane).ok_or_else(|| eyre!("unknown pane"))?;
            (
                live.pty.cwd().unwrap_or_else(|| self.root.clone()),
                live.program.clone(),
                live.args.clone(),
            )
        };
        let new_id = self.spawn_pane(&cwd, program.as_deref(), &args)?;
        let tab = self
            .tabs
            .get_mut(self.active)
            .ok_or_else(|| eyre!("no tab"))?;
        tab.layout.replace_id(pane, new_id);
        tab.panes.remove(&pane);
        if tab.focused == pane {
            tab.focused = new_id;
        }
        if tab.zoomed == Some(pane) {
            tab.zoomed = Some(new_id);
        }
        Ok(())
    }

    pub fn dispatch(&mut self, name: &str) -> Result<bool> {
        let name = name.trim().trim_start_matches('/');
        if let Some(id) = name
            .strip_prefix("layout.preset.")
            .or_else(|| name.strip_prefix("preset-"))
        {
            self.apply_preset(id)?;
            return Ok(true);
        }
        if let Some(raw) = name.strip_prefix("workspace.open:") {
            self.open_project(Path::new(raw))?;
            return Ok(true);
        }
        if let Some(raw) = name.strip_prefix("machine.open:") {
            self.connect_machine(raw, None)?;
            return Ok(true);
        }
        let Some(spec) = commands::lookup(name) else {
            if let Some(cmd) = crate::ext::command(name) {
                self.run(&cmd.run, &cmd.args)?;
                return Ok(true);
            }
            self.notice = Some(format!("comando desconocido: {name}"));
            return Ok(false);
        };
        if spec.kind == commands::CommandKind::Ui {
            return Ok(false);
        }
        match spec.id {
            "tab.new" => {
                self.spawn_preferred_tab()?;
            }
            "tab.close" => self.close_tab(self.active)?,
            "tab.duplicate" => self.duplicate_tab(self.active)?,
            "tab.next" => self.cycle_tab(1),
            "tab.prev" => self.cycle_tab(-1),
            "pane.splitRight" => {
                self.split(SplitDir::Columns, None, &[])?;
            }
            "pane.splitDown" => {
                self.split(SplitDir::Rows, None, &[])?;
            }
            "pane.zoom" => self.toggle_zoom(),
            "pane.focusLeft" => self.focus_nav(NavDir::Left),
            "pane.focusRight" => self.focus_nav(NavDir::Right),
            "pane.focusUp" => self.focus_nav(NavDir::Up),
            "pane.focusDown" => self.focus_nav(NavDir::Down),
            "pane.swapLeft" => self.swap_nav(NavDir::Left),
            "pane.swapRight" => self.swap_nav(NavDir::Right),
            "pane.swapUp" => self.swap_nav(NavDir::Up),
            "pane.swapDown" => self.swap_nav(NavDir::Down),
            "pane.close" => {
                let pane = self
                    .tabs
                    .get(self.active)
                    .map(|tab| tab.focused)
                    .unwrap_or(0);
                self.close_pane(pane)?;
            }
            "pane.restart" => {
                let pane = self
                    .tabs
                    .get(self.active)
                    .map(|tab| tab.focused)
                    .unwrap_or(0);
                self.restart_pane(pane)?;
            }
            id if let Some(program) = id.strip_prefix("run.") => {
                self.run(program, &[])?;
            }
            "music.playPause" => {
                let _ = crate::music::play_pause();
            }
            "music.next" => {
                let _ = crate::music::next();
            }
            "music.prev" => {
                let _ = crate::music::previous();
            }
            "workspace.next" => self.cycle_workspace(1)?,
            "workspace.prev" => self.cycle_workspace(-1)?,
            _ => {}
        }
        Ok(true)
    }

    pub fn reap(&mut self) {
        let mut poll = Vec::new();
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            drop_panes_not_in_layout(tab);
            for (id, pane) in &mut tab.panes {
                let _ = pane.pty.poll_exit();
                poll.push((
                    index,
                    *id,
                    pane.pty.child_exited(),
                    pane.pty.exit_code(),
                    pane.program.clone(),
                    pane.args.clone(),
                    pane.keep_on_exit,
                ));
            }
        }
        let mut empty_tabs = Vec::new();
        let mut ended = Vec::new();
        let mut restart_ssh = Vec::new();
        let mut dead = Vec::new();
        for (index, id, exited, exit_code, program, args, keep) in poll {
            if !exited {
                if program.as_deref() == Some("ssh")
                    && let Some(dest) = ssh::ssh_dest_from_args(&args)
                {
                    self.ssh_fail.remove(&dest);
                }
                continue;
            }
            if program.as_deref() == Some("ssh") {
                let dest = ssh::ssh_dest_from_args(&args).unwrap_or_else(|| "ssh".into());
                let fails = self.ssh_fail.entry(dest.clone()).or_insert(0);
                *fails = fails.saturating_add(1);
                if *fails <= 1 {
                    restart_ssh.push((index, id));
                    continue;
                }
                ended.push(format!("ssh {dest}"));
            } else if crate::agents::is_agent(program.as_deref())
                && let Some(name) = program
            {
                ended.push(name);
            }
            if keep {
                // Pane de instalación: registra el resultado pero no lo
                // elimina; el usuario cierra la pestaña cuando quiera.
                let was_running = self
                    .installs
                    .iter()
                    .any(|task| task.pane == id && task.state == "running");
                self.finish_install(id, exit_code);
                if was_running {
                    let code = exit_code
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "?".into());
                    self.notice = Some(format!(
                        "instalación terminó (exit {code}) · cerrá la pestaña cuando quieras"
                    ));
                }
                continue;
            }
            self.finish_install(id, exit_code);
            dead.push((index, id));
        }
        for (index, id) in dead {
            if let Some(tab) = self.tabs.get_mut(index) {
                tab.panes.remove(&id);
                tab.layout.remove_pane(id);
                tab.focused = tab.layout.first_leaf().unwrap_or(id);
                if tab.zoomed == Some(id) {
                    tab.zoomed = None;
                }
            }
        }
        for (tab, pane) in restart_ssh {
            let prev = self.active;
            if tab < self.tabs.len() {
                self.active = tab;
                self.notice = Some("ssh se cortó · reconectando".into());
                if self.restart_pane(pane).is_err()
                    && let Some(item) = self.tabs.get_mut(tab)
                {
                    item.panes.remove(&pane);
                    item.layout.remove_pane(pane);
                }
            }
            if !self.tabs.is_empty() {
                self.active = prev.min(self.tabs.len() - 1);
            }
        }
        for (index, tab) in self.tabs.iter().enumerate() {
            if tab.panes.is_empty() {
                empty_tabs.push(index);
            }
        }
        for index in empty_tabs.into_iter().rev() {
            self.tabs.remove(index);
        }
        if self.tabs.is_empty() {
            self.active = 0;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        if !ended.is_empty() {
            ended.sort();
            ended.dedup();
            self.notice = Some(format!("{} se cortó", ended.join(", ")));
        }
        self.reap_parked();
    }

    fn reap_parked(&mut self) {
        let mut finished = Vec::new();
        for parked in self.parked.values_mut() {
            finished.extend(drop_exited_in_tabs(&mut parked.tabs));
        }
        for (pane, exit_code) in finished {
            self.finish_install(pane, exit_code);
        }
    }

    fn finish_install(&mut self, pane: u64, exit_code: Option<u32>) {
        let Some(task) = self.installs.iter_mut().find(|task| task.pane == pane) else {
            return;
        };
        task.exit_code = exit_code;
        task.state = if exit_code == Some(0) {
            "installed".into()
        } else {
            "failed".into()
        };
    }

    fn refresh_live_context(&self) {
        let _ = self.context();
    }

    fn focused_cwd(&self) -> Option<PathBuf> {
        let tab = self.tabs.get(self.active)?;
        tab.panes.get(&tab.focused).and_then(|pane| pane.pty.cwd())
    }
}

fn drop_panes_not_in_layout(tab: &mut Tab) {
    let keep: HashSet<u64> = tab.layout.ids().into_iter().collect();
    tab.panes.retain(|id, _| keep.contains(id));
    if !keep.contains(&tab.focused) {
        tab.focused = tab.layout.first_leaf().unwrap_or(0);
    }
    if tab.zoomed.is_some_and(|id| !keep.contains(&id)) {
        tab.zoomed = None;
    }
}

fn drop_exited_in_tabs(tabs: &mut Vec<Tab>) -> Vec<(u64, Option<u32>)> {
    let mut empty_tabs = Vec::new();
    let mut finished = Vec::new();
    for (index, tab) in tabs.iter_mut().enumerate() {
        drop_panes_not_in_layout(tab);
        let dead: Vec<(u64, Option<u32>, bool)> = tab
            .panes
            .iter_mut()
            .filter_map(|(id, pane)| {
                let _ = pane.pty.poll_exit();
                pane.pty
                    .child_exited()
                    .then_some((*id, pane.pty.exit_code(), pane.keep_on_exit))
            })
            .collect();
        for (id, exit_code, keep) in dead {
            finished.push((id, exit_code));
            if keep {
                continue;
            }
            tab.panes.remove(&id);
            tab.layout.remove_pane(id);
            tab.focused = tab.layout.first_leaf().unwrap_or(id);
            if tab.zoomed == Some(id) {
                tab.zoomed = None;
            }
        }
        if tab.panes.is_empty() && tab.kind == "term" {
            empty_tabs.push(index);
        }
    }
    for index in empty_tabs.into_iter().rev() {
        tabs.remove(index);
    }
    finished
}

fn pane_ids_in(tabs: &[Tab]) -> Vec<u64> {
    tabs.iter()
        .flat_map(|tab| tab.panes.keys().copied())
        .collect()
}

fn collect_pane_pids(tabs: &[Tab]) -> Vec<(u64, u32)> {
    let mut out = Vec::new();
    for tab in tabs {
        for (id, pane) in &tab.panes {
            if let Some(pid) = pane.pty.process_id() {
                out.push((*id, pid));
            }
        }
    }
    out
}

fn collect_process_names(tabs: &[Tab], names: &mut Vec<String>) {
    for tab in tabs {
        for id in tab.layout.ids() {
            let Some(pane) = tab.panes.get(&id) else {
                continue;
            };
            let program = pane.program.as_deref().filter(|name| !name.is_empty());
            if let Some(program) = program
                && !names.iter().any(|seen| seen == program)
            {
                names.push(program.to_string());
            }
            if let Some(pid) = pane.pty.process_id() {
                for name in crate::agents::running_under(pid) {
                    if !names.iter().any(|seen| seen == &name) {
                        names.push(name);
                    }
                }
            }
        }
    }
}

fn push_project_snap(
    projects: &mut Vec<ProjectSnap>,
    root: &Path,
    name: &str,
    current: bool,
    tabs: &[Tab],
) {
    let root = canonicalize(root);
    let agents = tabs
        .iter()
        .flat_map(|tab| tab.panes.values())
        .filter(|pane| crate::agents::is_agent(pane.program.as_deref()))
        .count();
    if let Some(project) = projects.iter_mut().find(|project| project.root == root) {
        project.current |= current;
        project.tabs += tabs.len();
        project.agents += agents;
        return;
    }
    projects.push(ProjectSnap {
        name: name.to_string(),
        branch: git::branch_label(&root),
        root,
        current,
        tabs: tabs.len(),
        agents,
    });
}

fn push_agent_worktrees(projects: &mut Vec<ProjectSnap>, panes: &HashMap<u64, LivePane>) {
    for pane in panes.values() {
        if !crate::agents::is_agent(pane.program.as_deref()) {
            continue;
        }
        let Some(worktree) = pane.worktree.as_deref() else {
            continue;
        };
        let root = canonicalize(worktree);
        if let Some(project) = projects.iter_mut().find(|project| project.root == root) {
            project.agents += 1;
            continue;
        }
        projects.push(ProjectSnap {
            name: workspace_name(&root),
            branch: git::branch_label(&root),
            root,
            current: false,
            tabs: 0,
            agents: 1,
        });
    }
}

fn cap_parked<T>(parked: &mut HashMap<PathBuf, T>, order: &mut VecDeque<PathBuf>, cap: usize) {
    while parked.len() > cap {
        let Some(old) = order.pop_front() else {
            break;
        };
        parked.remove(&old);
    }
}

fn wants_own_tab(program: &str) -> bool {
    matches!(program, "vim" | "htop") || crate::registry::is_known(program)
}

fn sanitize_new_tab(raw: &str) -> String {
    let name = raw.trim().to_ascii_lowercase();
    match name.as_str() {
        "" | "shell" | "term" | "terminal" => "shell".into(),
        "ts" => "tailscale".into(),
        "ssh" | "tailscale" => name,
        other if is_run_cli(other) => other.into(),
        _ => "shell".into(),
    }
}

fn shell_label() -> String {
    std::env::var("SHELL")
        .ok()
        .as_deref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("sh")
        .to_string()
}

fn workspace_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace")
        .to_string()
}

fn canonicalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn tab_name_from_dest(dest: &str) -> String {
    crate::config::host_label(dest)
}

fn pane_remote(program: Option<&str>, args: &[String]) -> Option<String> {
    if program != Some("ssh") {
        return None;
    }
    ssh::ssh_dest_from_args(args).map(|dest| crate::config::host_label(&dest))
}

fn tab_label(tab: &Tab) -> String {
    if tab.kind != "term" {
        return tab
            .name
            .clone()
            .or_else(|| tab.rel.clone())
            .unwrap_or_else(|| tab.kind.clone());
    }
    tab.panes
        .get(&tab.focused)
        .map(|pane| pane.title.clone())
        .unwrap_or_else(|| "tab".into())
}

fn view_kind(program: Option<&str>) -> Option<&'static str> {
    match program {
        Some("__file__") => Some("file"),
        Some("__rest__") => Some("rest"),
        _ => None,
    }
}

fn pane_needs_attention(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    [
        "waiting",
        "awaiting",
        "permission",
        "ask",
        "yn?",
        "(y/n)",
        "approve",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn layout_from_saved(node: &session::SavedNode, ids: &[u64]) -> LayoutNode {
    fn walk(node: &session::SavedNode, ids: &mut std::slice::Iter<'_, u64>) -> LayoutNode {
        match node {
            session::SavedNode::Leaf { .. } => LayoutNode::leaf(*ids.next().unwrap_or(&0)),
            session::SavedNode::Split {
                dir,
                percent,
                first,
                second,
            } => LayoutNode::Split {
                dir: *dir,
                percent: *percent,
                first: Box::new(walk(first, ids)),
                second: Box::new(walk(second, ids)),
            },
        }
    }
    walk(node, &mut ids.iter())
}

fn startup_already_open<'a>(
    open: impl IntoIterator<Item = Option<&'a str>>,
    program: &str,
) -> bool {
    open.into_iter().any(|name| name == Some(program))
}

fn startup_needed<'a>(open: impl IntoIterator<Item = Option<&'a str>>, program: &str) -> bool {
    let program = program.trim();
    !program.is_empty() && !startup_already_open(open, program)
}

#[cfg(test)]
mod tests {
    use super::{pane_remote, sanitize_new_tab, startup_already_open, startup_needed};

    #[test]
    fn startup_skips_when_layout_already_has_program() {
        let open = [Some("nvim"), None, Some("lazygit")];
        assert!(!startup_needed(open, "nvim"));
        assert!(startup_needed(open, "btop"));
        assert!(!startup_needed(open, ""));
        assert!(startup_already_open(open, "lazygit"));
    }

    #[test]
    fn new_tab_kind_accepts_catalog_and_rejects_paths() {
        assert_eq!(sanitize_new_tab("Terminal"), "shell");
        assert_eq!(sanitize_new_tab("nvim"), "nvim");
        assert_eq!(sanitize_new_tab("copilot"), "copilot");
        assert_eq!(sanitize_new_tab("ts"), "tailscale");
        assert_eq!(sanitize_new_tab("/bin/bash"), "shell");
        assert_eq!(sanitize_new_tab("rm"), "shell");
    }

    #[test]
    fn ssh_pane_exposes_host_not_argv() {
        assert_eq!(
            pane_remote(Some("ssh"), &["chae".into()]),
            Some("chae".into())
        );
        assert_eq!(pane_remote(Some("nvim"), &["README.md".into()]), None);
    }

    #[test]
    fn find_open_file_logic_uses_args_when_no_opened_path() {
        let path = std::path::Path::new("/ws/src/App.tsx");
        assert!(crate::files::pane_holds_file(
            None,
            &["-c".into(), "set title".into(), "/ws/src/App.tsx".into()],
            path
        ));
        assert!(!crate::files::pane_holds_file(
            None,
            &["-c".into(), "set title".into(), "/ws/other.ts".into()],
            path
        ));
    }

    #[test]
    fn cap_parked_drops_oldest_roots() {
        use std::collections::{HashMap, VecDeque};
        use std::path::PathBuf;

        let mut parked = HashMap::new();
        let mut order = VecDeque::new();
        for i in 0..14 {
            let root = PathBuf::from(format!("/ws/{i}"));
            parked.insert(root.clone(), i);
            order.retain(|path| path != &root);
            order.push_back(root);
            super::cap_parked(&mut parked, &mut order, 12);
        }
        assert_eq!(parked.len(), 12);
        assert!(!parked.contains_key(&PathBuf::from("/ws/0")));
        assert!(!parked.contains_key(&PathBuf::from("/ws/1")));
        assert_eq!(parked.get(&PathBuf::from("/ws/13")), Some(&13));
    }
}
