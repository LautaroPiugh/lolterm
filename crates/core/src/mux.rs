use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use portable_pty::PtySize;
use serde::Serialize;

use crate::commands::{self, CommandHit};
use crate::config::{RemoteConfig, Theme};
use crate::files;
use crate::git;
use crate::keys::{self, Binding};
use crate::layout::{LayoutNode, NavDir, SplitDir};
use crate::pty::BytePty;
use crate::session::{self, SavedTab, SavedWorkspace, Session};
use crate::ssh;

pub const RUN_CLIS: &[&str] = &[
    "nvim", "lazygit", "btop", "yazi", "codex", "claude", "opencode", "gemini", "cline",
];

#[derive(Serialize)]
pub struct Snapshot {
    pub root: PathBuf,
    pub name: String,
    pub branch: Option<String>,
    pub active_tab: usize,
    pub tabs: Vec<TabSnap>,
    pub git: Option<git::Status>,
    pub git_log: Vec<String>,
    pub tree: Vec<files::TreeRow>,
    pub tailscale: crate::tailscale::Status,
    pub run_clis: Vec<RunCli>,
    pub notice: Option<String>,
    pub theme: Theme,
    pub ssh_user: Option<String>,
    pub keybindings: Vec<Binding>,
    pub version: String,
    pub presets: Vec<crate::presets::Preset>,
    pub workspaces: Vec<WorkspaceSnap>,
    pub startup: Vec<session::StartupCmd>,
}

#[derive(Serialize)]
pub struct WorkspaceSnap {
    pub name: String,
    pub root: PathBuf,
    pub current: bool,
}

#[derive(Serialize)]
pub struct TabSnap {
    pub name: String,
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
}

#[derive(Serialize)]
pub struct RunCli {
    pub name: String,
    pub available: bool,
}

struct LivePane {
    title: String,
    program: Option<String>,
    args: Vec<String>,
    pty: BytePty,
}

struct Tab {
    name: Option<String>,
    focused: u64,
    zoomed: Option<u64>,
    layout: LayoutNode,
    panes: HashMap<u64, LivePane>,
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
    startup: Vec<session::StartupCmd>,
    notice: Option<String>,
    theme: Theme,
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
            startup: Vec::new(),
            notice: None,
            theme: cfg.theme,
        };
        if session::exists()
            && let Ok(session) = session::load()
        {
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
                mux.restore_tabs(&ws.tabs, ws.active_tab)?;
            }
        }
        if mux.tabs.is_empty() {
            mux.new_tab(None, None, &[], true)?;
        }
        mux.apply_startup()?;
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
        self.tabs.push(Tab {
            name: saved.name.clone(),
            focused: 0,
            zoomed: None,
            layout: LayoutNode::leaf(0),
            panes: HashMap::new(),
        });
        self.active = self.tabs.len() - 1;
        let specs = saved.tree.leaf_specs();
        let mut ids = Vec::new();
        for spec in &specs {
            let cwd = spec.cwd.clone().unwrap_or_else(|| self.root.clone());
            let program = spec
                .program
                .as_deref()
                .filter(|name| files::command_on_path(name));
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
        let id = self.next_id;
        self.next_id += 1;
        let env = self.context_env();
        let pty = BytePty::spawn(
            id,
            PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
            cwd,
            program,
            args,
            &env,
            self.tx.clone(),
        )?;
        let title = program
            .map(|name| name.to_string())
            .unwrap_or_else(shell_label);
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.panes.insert(
                id,
                LivePane {
                    title,
                    program: program.map(ToString::to_string),
                    args: args.to_vec(),
                    pty,
                },
            );
        } else {
            let mut panes = HashMap::new();
            panes.insert(
                id,
                LivePane {
                    title,
                    program: program.map(ToString::to_string),
                    args: args.to_vec(),
                    pty,
                },
            );
            self.tabs.push(Tab {
                name: program.map(ToString::to_string),
                focused: id,
                zoomed: None,
                layout: LayoutNode::leaf(id),
                panes,
            });
            self.active = self.tabs.len() - 1;
            return Ok(id);
        }
        Ok(id)
    }

    fn context_env(&self) -> Vec<(String, String)> {
        vec![("LOLTERM_ROOT".into(), self.root.display().to_string())]
    }

    pub fn snapshot(&self) -> Snapshot {
        let marks = git::path_marks(&self.root);
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
                    focused: tab.focused,
                    zoomed: tab.zoomed,
                    layout: tab.layout.clone(),
                    panes: tab
                        .panes
                        .iter()
                        .map(|(id, pane)| PaneSnap {
                            id: *id,
                            title: pane.title.clone(),
                            program: pane.program.clone(),
                        })
                        .collect(),
                })
                .collect(),
            git: git::status(&self.root),
            git_log: git::oneline(&self.root, 8),
            tree: files::visible_tree(&self.root, &self.expanded, &marks),
            tailscale: crate::tailscale::probe(),
            run_clis: RUN_CLIS
                .iter()
                .map(|name| RunCli {
                    name: (*name).to_string(),
                    available: files::command_on_path(name),
                })
                .collect(),
            notice: self.notice.clone(),
            theme: self.theme,
            ssh_user: self.remote.user.clone(),
            keybindings: keys::load(),
            version: crate::VERSION.to_string(),
            presets: crate::presets::summaries(),
            workspaces: self.workspace_snaps(),
            startup: self.startup.clone(),
        }
    }

    fn workspace_snaps(&self) -> Vec<WorkspaceSnap> {
        let mut list = self.saved_workspaces.clone();
        session::upsert_workspace(&mut list, self.capture_workspace(), 12);
        list.into_iter()
            .map(|ws| WorkspaceSnap {
                current: ws.root == self.root,
                name: ws.name,
                root: ws.root,
            })
            .collect()
    }

    fn capture_workspace(&self) -> SavedWorkspace {
        SavedWorkspace {
            name: self.name.clone(),
            root: self.root.clone(),
            active_tab: self.active,
            startup: self.startup.clone(),
            tabs: self
                .tabs
                .iter()
                .map(|tab| {
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
        let theme = Theme::parse(name)
            .ok_or_else(|| eyre!("tema desconocido: {name} (sage, dusk, mono)"))?;
        self.theme = theme;
        let mut cfg = crate::config::load();
        cfg.theme = theme;
        crate::config::save(&cfg).map_err(|err| eyre!("no pude guardar config: {err}"))?;
        Ok(())
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
    }

    pub fn write(&mut self, pane: u64, bytes: &[u8]) -> Result<()> {
        let tab = self
            .tabs
            .get_mut(self.active)
            .ok_or_else(|| eyre!("no tab"))?;
        let live = tab
            .panes
            .get_mut(&pane)
            .ok_or_else(|| eyre!("unknown pane"))?;
        live.pty.write_input(bytes)
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
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn new_tab(
        &mut self,
        program: Option<&str>,
        cwd: Option<&Path>,
        args: &[String],
        _shell_ok: bool,
    ) -> Result<u64> {
        let cwd = cwd.unwrap_or(&self.root);
        let id = self.next_id;
        self.next_id += 1;
        let env = self.context_env();
        let pty = BytePty::spawn(
            id,
            PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
            cwd,
            program,
            args,
            &env,
            self.tx.clone(),
        )?;
        let title = program.map(ToString::to_string).unwrap_or_else(shell_label);
        let mut panes = HashMap::new();
        panes.insert(
            id,
            LivePane {
                title,
                program: program.map(ToString::to_string),
                args: args.to_vec(),
                pty,
            },
        );
        self.tabs.push(Tab {
            name: program.map(ToString::to_string),
            focused: id,
            zoomed: None,
            layout: LayoutNode::leaf(id),
            panes,
        });
        self.active = self.tabs.len() - 1;
        Ok(id)
    }

    pub fn close_tab(&mut self, index: usize) -> Result<()> {
        if self.tabs.len() <= 1 {
            self.notice = Some("solo hay una tab".into());
            return Ok(());
        }
        if index < self.tabs.len() {
            self.tabs.remove(index);
            if self.active >= self.tabs.len() {
                self.active = self.tabs.len() - 1;
            }
        }
        Ok(())
    }

    pub fn duplicate_tab(&mut self, index: usize) -> Result<()> {
        let Some(src) = self.tabs.get(index) else {
            return Ok(());
        };
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
        self.restore_one(&saved)
    }

    pub fn split(&mut self, dir: SplitDir, program: Option<&str>, args: &[String]) -> Result<u64> {
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
        let tab = self
            .tabs
            .get_mut(self.active)
            .ok_or_else(|| eyre!("no tab"))?;
        if tab.panes.len() <= 1 {
            return self.close_tab(self.active);
        }
        tab.panes.remove(&pane);
        tab.layout.remove_pane(pane);
        tab.focused = tab.layout.first_leaf().unwrap_or(pane);
        if tab.zoomed == Some(pane) {
            tab.zoomed = None;
        }
        Ok(())
    }

    pub fn run(&mut self, program: &str, args: &[String]) -> Result<u64> {
        if !files::command_on_path(program) {
            self.notice = Some(format!("`{program}` no está en PATH"));
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
        let Some((editor, extra)) = files::editor() else {
            self.notice = Some("no hay $EDITOR / nvim".into());
            return Ok(0);
        };
        let mut args = extra;
        args.push(path.display().to_string());
        self.new_tab(Some(&editor), None, &args, false)
    }

    pub fn toggle_expand(&mut self, rel: &str) {
        if !self.expanded.remove(rel) {
            self.expanded.insert(rel.to_string());
        }
    }

    pub fn open_project(&mut self, path: &Path) -> Result<()> {
        let next = canonicalize(path);
        if next == self.root && !self.tabs.is_empty() {
            return Ok(());
        }
        if !self.tabs.is_empty() {
            let snapshot = self.capture_workspace();
            session::upsert_workspace(&mut self.saved_workspaces, snapshot, 12);
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
            self.restore_tabs(&ws.tabs, ws.active_tab)?;
        } else {
            self.startup.clear();
        }
        if self.tabs.is_empty() {
            let root = self.root.clone();
            self.new_tab(None, Some(&root), &[], true)?;
        }
        self.apply_startup()?;
        session::push_unique_path(&mut self.recent_projects, self.root.clone(), 12);
        self.persist();
        Ok(())
    }

    fn has_program(&self, program: &str) -> bool {
        self.tabs.iter().any(|tab| {
            tab.panes
                .values()
                .any(|pane| pane.program.as_deref() == Some(program))
        })
    }

    pub fn apply_startup(&mut self) -> Result<()> {
        let cmds = self.startup.clone();
        for cmd in cmds {
            if cmd.program.trim().is_empty() || self.has_program(&cmd.program) {
                continue;
            }
            self.run(&cmd.program, &cmd.args)?;
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

    pub fn ssh(&mut self, dest: &str) -> Result<u64> {
        let dest = dest.trim();
        if dest.is_empty() {
            self.notice = Some("hace falta user@host".into());
            return Ok(0);
        }
        session::push_unique(&mut self.recents, dest.to_string(), 12);
        let id = self.new_tab(Some("ssh"), None, &["-tt".into(), dest.to_string()], false)?;
        self.name_active_tab(tab_name_from_dest(dest));
        Ok(id)
    }

    pub fn ts_ssh(&mut self, target: &str, user: Option<&str>) -> Result<u64> {
        let user = user
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .or(self.remote.user.as_deref());
        let Some(user) = user else {
            self.notice = Some("hace falta un usuario ssh".into());
            return Ok(0);
        };
        if !ssh::ssh_user_ok(user) {
            self.notice = Some("usuario ssh inválido".into());
            return Ok(0);
        }
        let args = ssh::ts_ssh_args(target, Some(user), &self.remote.tmux);
        let dest = ssh::ts_ssh_dest(target, Some(user));
        if dest.is_empty() {
            self.notice = Some("máquina Tailscale vacía".into());
            return Ok(0);
        }
        session::push_unique(&mut self.recents, dest.clone(), 12);
        let id = self.new_tab(Some("ssh"), None, &args, false)?;
        self.name_active_tab(tab_name_from_dest(&dest));
        Ok(id)
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
    }

    pub fn rename_tab(&mut self, index: usize, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.name = Some(name.to_string());
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
        let Some(spec) = commands::lookup(name) else {
            self.notice = Some(format!("comando desconocido: {name}"));
            return Ok(false);
        };
        if spec.kind == commands::CommandKind::Ui {
            return Ok(false);
        }
        match spec.id {
            "tab.new" => {
                self.new_tab(None, None, &[], true)?;
            }
            "tab.close" => self.close_tab(self.active)?,
            "tab.duplicate" => self.duplicate_tab(self.active)?,
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
            "run.lazygit" => {
                self.run("lazygit", &[])?;
            }
            _ => {}
        }
        Ok(true)
    }

    pub fn reap(&mut self) {
        let mut empty_tabs = Vec::new();
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let mut dead = Vec::new();
            for (id, pane) in &mut tab.panes {
                let _ = pane.pty.poll_exit();
                if pane.pty.child_exited() {
                    dead.push(*id);
                }
            }
            for id in dead {
                tab.panes.remove(&id);
                tab.layout.remove_pane(id);
                tab.focused = tab.layout.first_leaf().unwrap_or(id);
                if tab.zoomed == Some(id) {
                    tab.zoomed = None;
                }
            }
            if tab.panes.is_empty() {
                empty_tabs.push(index);
            }
        }
        for index in empty_tabs.into_iter().rev() {
            self.tabs.remove(index);
        }
        if self.tabs.is_empty() {
            let _ = self.new_tab(None, None, &[], true);
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }

    fn focused_cwd(&self) -> Option<PathBuf> {
        let tab = self.tabs.get(self.active)?;
        tab.panes.get(&tab.focused).and_then(|pane| pane.pty.cwd())
    }
}

fn wants_own_tab(program: &str) -> bool {
    matches!(
        program,
        "nvim"
            | "vim"
            | "lazygit"
            | "btop"
            | "htop"
            | "yazi"
            | "opencode"
            | "claude"
            | "codex"
            | "gemini"
            | "cline"
    )
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
    dest.rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(dest)
        .split('.')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(dest)
        .to_string()
}

fn tab_label(tab: &Tab) -> String {
    tab.panes
        .get(&tab.focused)
        .map(|pane| pane.title.clone())
        .unwrap_or_else(|| "tab".into())
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
