use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::layout::{Constraint, Layout, Position, Rect, Size};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph, Tabs};
use ratatui::{DefaultTerminal, Frame};
use tui_term::widget::PseudoTerminal;

use crate::commands::{CommandId, CommandSpec, filter_commands, split_query};
use crate::config::Config;
use crate::event::{encode_key, encode_mouse, is_prefix_key, is_quit_key};
use crate::pane::Pane;
use crate::session::{SavedNode, SavedTab, SavedWorkspace, Session};
use crate::tree::{Divider, FocusDir, Node, PaneTree, SplitDir, pty_size_from_rect};

const MAX_PANES: usize = 8;
const MAX_TABS: usize = 8;
const MAX_WORKSPACES: usize = 6;

struct Palette {
    query: String,
    selected: usize,
}

struct Tab {
    name: Option<String>,
    tree: PaneTree,
}

struct Workspace {
    name: String,
    root: PathBuf,
    branch: Option<String>,
    tabs: Vec<Tab>,
    active: usize,
}

pub struct App {
    running: bool,
    workspaces: Vec<Workspace>,
    active_ws: usize,
    next_id: u64,
    palette: Option<Palette>,
    term_size: Size,
    start_cwd: PathBuf,
    config: Config,
    drag: Option<Divider>,
    last_git: Instant,
}

impl App {
    pub fn new(term_size: Size) -> Result<Self> {
        let mut app = Self {
            running: true,
            workspaces: Vec::new(),
            active_ws: 0,
            next_id: 1,
            palette: None,
            term_size,
            start_cwd: canonicalize_dir(
                &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            ),
            config: crate::config::load(),
            drag: None,
            last_git: Instant::now(),
        };
        if crate::session::exists()
            && let Ok(session) = crate::session::load()
        {
            app.restore_session(session)?;
        }
        if app.workspaces.is_empty() {
            app.open_default_workspace()?;
        }
        Ok(app)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while self.running {
            self.reap()?;
            if !self.running {
                break;
            }
            self.term_size = terminal.size()?;
            self.sync_sizes()?;
            if self.last_git.elapsed() >= Duration::from_secs(2) {
                self.refresh_active_branch();
                self.last_git = Instant::now();
            }
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        let _ = crate::session::save(&self.snapshot());
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let [ws_bar, tab_bar, body, status] = chrome(frame.area());

        let ws_titles: Vec<Line> = self
            .workspaces
            .iter()
            .map(|ws| Line::from(workspace_label(ws)))
            .collect();
        let theme = self.config.theme;
        frame.render_widget(
            Tabs::new(ws_titles)
                .select(self.active_ws)
                .highlight_style(Style::default().fg(theme.workspace).bg(Color::Black))
                .style(Style::default().fg(theme.inactive)),
            ws_bar,
        );

        let Some(ws) = self.workspaces.get(self.active_ws) else {
            return;
        };

        let tab_titles: Vec<Line> = ws
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| Line::from(tab_label(tab, index)))
            .collect();
        frame.render_widget(
            Tabs::new(tab_titles)
                .select(ws.active)
                .highlight_style(Style::default().fg(Color::Black).bg(theme.focus))
                .style(Style::default().fg(theme.inactive)),
            tab_bar,
        );

        let Some(tab) = ws.tabs.get(ws.active) else {
            return;
        };

        for (id, area) in tab.tree.areas(body) {
            let Some(pane) = tab.tree.panes.get(&id) else {
                continue;
            };
            let focused = id == tab.tree.focused;
            let parser = pane.shell.parser();
            let marker = if focused { "*" } else { " " };
            let title = format!(" {marker} {} ", pane.title());
            let border = if focused {
                Style::default().fg(theme.focus)
            } else {
                Style::default().fg(theme.inactive)
            };
            let title_style = if focused {
                Style::default().fg(Color::Black).bg(theme.focus)
            } else {
                Style::default().fg(theme.inactive)
            };
            frame.render_widget(
                PseudoTerminal::new(parser.screen()).block(
                    Block::bordered()
                        .border_style(border)
                        .title(Line::from(title).style(title_style)),
                ),
                area,
            );
        }

        frame.render_widget(
            Paragraph::new(Line::from(self.status_text(ws, tab)))
                .style(Style::default().fg(theme.inactive)),
            status,
        );

        self.draw_palette(frame);
    }

    fn handle_events(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(16))? {
            return Ok(());
        }

        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.handle_key(key)?,
            Event::Mouse(mouse) => self.handle_mouse(mouse)?,
            Event::Paste(text) => {
                if let Some(palette) = &mut self.palette {
                    palette.query.push_str(&text);
                    palette.selected = 0;
                } else if let Some(shell) = self.focused_shell() {
                    shell.write_input(text.as_bytes())?;
                }
            }
            Event::Resize(width, height) => {
                self.term_size = Size { width, height };
                self.sync_sizes()?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if is_quit_key(key, self.config.quit) {
            self.running = false;
            return Ok(());
        }
        if self.palette.is_some() {
            return self.handle_palette_key(key);
        }
        if is_prefix_key(key, self.config.prefix) {
            self.palette = Some(Palette {
                query: "/".to_string(),
                selected: 0,
            });
            return Ok(());
        }
        if self.config.new_tab.is_some_and(|chord| chord.matches(key)) {
            self.new_tab()?;
            return Ok(());
        }
        if self
            .config
            .split_right
            .is_some_and(|chord| chord.matches(key))
        {
            self.split(SplitDir::Columns)?;
            return Ok(());
        }
        if self
            .config
            .split_down
            .is_some_and(|chord| chord.matches(key))
        {
            self.split(SplitDir::Rows)?;
            return Ok(());
        }
        if let Some(bytes) = encode_key(key)
            && let Some(shell) = self.focused_shell()
        {
            shell.write_input(&bytes)?;
        }
        Ok(())
    }

    fn handle_palette_key(&mut self, key: KeyEvent) -> Result<()> {
        if is_prefix_key(key, self.config.prefix) || key.code == KeyCode::Esc {
            self.palette = None;
            return Ok(());
        }

        match key.code {
            KeyCode::Enter => {
                if let Some(command) = self.selected_command() {
                    let args = self
                        .palette
                        .as_ref()
                        .map(|palette| split_query(&palette.query).1)
                        .unwrap_or_default();
                    let id = command.id;
                    self.palette = None;
                    self.run_command(id, &args)?;
                }
            }
            KeyCode::Up => self.move_palette_selection(-1),
            KeyCode::Down => self.move_palette_selection(1),
            KeyCode::Backspace => {
                if let Some(palette) = &mut self.palette
                    && palette.query.len() > 1
                {
                    palette.query.pop();
                    palette.selected = 0;
                }
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(palette) = &mut self.palette {
                    palette.query.push(ch);
                    palette.selected = 0;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn selected_command(&self) -> Option<&'static CommandSpec> {
        let palette = self.palette.as_ref()?;
        let matches = filter_commands(&palette.query);
        matches.get(palette.selected).copied()
    }

    fn move_palette_selection(&mut self, delta: i32) {
        let Some(palette) = &mut self.palette else {
            return;
        };
        let count = filter_commands(&palette.query).len() as i32;
        if count == 0 {
            palette.selected = 0;
            return;
        }
        palette.selected = (palette.selected as i32 + delta).rem_euclid(count) as usize;
    }

    fn run_command(&mut self, id: CommandId, args: &[String]) -> Result<()> {
        match id {
            CommandId::SplitRight => self.split(SplitDir::Columns)?,
            CommandId::SplitDown => self.split(SplitDir::Rows)?,
            CommandId::Grow => self.grow_focused(5),
            CommandId::Shrink => self.grow_focused(-5),
            CommandId::FocusNext => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.tree.focus_next();
                }
            }
            CommandId::FocusLeft => self.focus_dir(FocusDir::Left),
            CommandId::FocusRight => self.focus_dir(FocusDir::Right),
            CommandId::FocusUp => self.focus_dir(FocusDir::Up),
            CommandId::FocusDown => self.focus_dir(FocusDir::Down),
            CommandId::ClosePane => self.close_pane(),
            CommandId::NewTab => self.new_tab()?,
            CommandId::NextTab => self.cycle_tab(1),
            CommandId::PrevTab => self.cycle_tab(-1),
            CommandId::CloseTab => self.close_tab(),
            CommandId::RenameTab => self.rename_tab(args),
            CommandId::NewWorkspace => self.new_workspace()?,
            CommandId::NextWorkspace => self.cycle_workspace(1),
            CommandId::CloseWorkspace => self.close_workspace(),
            CommandId::LaunchAi => self.launch_ai()?,
            CommandId::LaunchCodex => self.launch_named("codex")?,
            CommandId::LaunchClaude => self.launch_named("claude")?,
            CommandId::LaunchOpencode => self.launch_named("opencode")?,
            CommandId::LaunchCline => self.launch_named("cline")?,
            CommandId::LaunchGemini => self.launch_named("gemini")?,
            CommandId::LaunchLazygit => self.launch_named("lazygit")?,
            CommandId::LaunchSsh => {
                let cwd = self.start_cwd.clone();
                self.launch_program("ssh", args, &cwd)?;
            }
            CommandId::LaunchTailscale => {
                let cwd = self.start_cwd.clone();
                let args = if args.is_empty() {
                    vec!["status".to_string()]
                } else {
                    args.to_vec()
                };
                self.launch_program("tailscale", &args, &cwd)?;
            }
            CommandId::GitStatus => {
                let root = self.active_root();
                self.launch_program(
                    "git",
                    &["status".into(), "--short".into(), "--branch".into()],
                    &root,
                )?;
            }
            CommandId::ScrollUp => self.scroll_focused(5),
            CommandId::ScrollDown => self.scroll_focused(-5),
            CommandId::Quit => self.running = false,
        }
        Ok(())
    }

    fn draw_palette(&self, frame: &mut Frame) {
        let Some(palette) = &self.palette else {
            return;
        };
        let matches = filter_commands(&palette.query);
        let list_height = matches.len().min(8) as u16;
        let height = list_height.saturating_add(3).max(4);
        let full = frame.area();
        let area = Rect {
            x: full.x.saturating_add(2),
            y: full.bottom().saturating_sub(height.saturating_add(1)),
            width: full.width.saturating_sub(4).max(20),
            height,
        };
        frame.render_widget(Clear, area);
        let block = Block::bordered().title(" comandos · Enter ejecuta · Esc cierra ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let [input, list] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);
        frame.render_widget(Paragraph::new(palette.query.as_str()), input);

        let lines: Vec<Line> = matches
            .iter()
            .enumerate()
            .map(|(index, command)| {
                let text = format!(" /{}  {}", command.slash, command.hint);
                if index == palette.selected {
                    Line::from(text).style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(self.config.theme.focus),
                    )
                } else {
                    Line::from(text)
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), list);
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        let [ws_bar, tab_bar, body, _] = chrome(term_rect(self.term_size));
        let pos = Position::new(mouse.column, mouse.row);

        if matches!(mouse.kind, MouseEventKind::Up(_)) {
            let dragging = self.drag.is_some();
            self.drag = None;
            if dragging {
                return Ok(());
            }
        }

        if let Some(drag) = self.drag.clone()
            && matches!(
                mouse.kind,
                MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved
            )
        {
            if let Some(tab) = self.active_tab_mut() {
                tab.tree.drag_split(&drag.path, drag.dir, drag.parent, pos);
            }
            return Ok(());
        }

        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            if ws_bar.contains(pos) {
                if let Some(index) = index_at(mouse.column, ws_bar, self.workspaces.len()) {
                    self.active_ws = index;
                    self.refresh_active_branch();
                }
                return Ok(());
            }
            if tab_bar.contains(pos)
                && let Some(ws) = self.workspaces.get_mut(self.active_ws)
                && let Some(index) = index_at(mouse.column, tab_bar, ws.tabs.len())
            {
                ws.active = index;
                return Ok(());
            }
            if let Some(tab) = self.active_tab()
                && let Some(divider) = tab
                    .tree
                    .dividers(body)
                    .into_iter()
                    .find(|divider| divider.hit.contains(pos))
            {
                self.drag = Some(divider);
                return Ok(());
            }
        }

        let Some(tab) = self.active_tab_mut() else {
            return Ok(());
        };

        let Some((id, area)) = tab
            .tree
            .areas(body)
            .into_iter()
            .find(|(_, rect)| rect.contains(pos))
        else {
            return Ok(());
        };

        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            tab.tree.focused = id;
        }

        let inner = Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        let wants_mouse = tab
            .tree
            .panes
            .get(&id)
            .is_some_and(|pane| pane.shell.wants_mouse());

        match mouse.kind {
            MouseEventKind::ScrollUp if !wants_mouse => {
                if let Some(pane) = tab.tree.panes.get(&id) {
                    pane.shell.scroll_by(3);
                }
            }
            MouseEventKind::ScrollDown if !wants_mouse => {
                if let Some(pane) = tab.tree.panes.get(&id) {
                    pane.shell.scroll_by(-3);
                }
            }
            MouseEventKind::Moved | MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight
                if !wants_mouse => {}
            _ if wants_mouse && inner.contains(pos) => {
                let col = mouse.column.saturating_sub(inner.x).saturating_add(1);
                let row = mouse.row.saturating_sub(inner.y).saturating_add(1);
                if let Some(bytes) = encode_mouse(mouse, col, row)
                    && let Some(pane) = tab.tree.panes.get_mut(&id)
                {
                    pane.shell.write_input(&bytes)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn split(&mut self, dir: SplitDir) -> Result<()> {
        let area = self.content_area();
        let cwd = self.start_cwd.clone();
        let spawn_area = {
            let Some(tab) = self.active_tab() else {
                return Ok(());
            };
            if tab.tree.pane_count() >= MAX_PANES {
                return Ok(());
            }
            let focused = tab.tree.focused;
            tab.tree
                .areas(area)
                .iter()
                .find(|(id, _)| *id == focused)
                .map(|(_, rect)| half_rect(*rect, dir))
                .unwrap_or(area)
        };

        let pane = self.spawn_pane(spawn_area, &cwd)?;
        if let Some(tab) = self.active_tab_mut() {
            tab.tree.split_focused(dir, pane);
        }
        Ok(())
    }

    fn close_pane(&mut self) {
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        if !tab.tree.close_focused() || tab.tree.pane_count() == 0 {
            self.close_tab();
        }
    }

    fn new_tab(&mut self) -> Result<()> {
        let cwd = self.start_cwd.clone();
        let area = self.content_area();
        if self
            .workspaces
            .get(self.active_ws)
            .is_none_or(|ws| ws.tabs.len() >= MAX_TABS)
        {
            return Ok(());
        }
        let pane = self.spawn_pane(area, &cwd)?;
        if let Some(ws) = self.workspaces.get_mut(self.active_ws) {
            ws.tabs.push(Tab {
                name: None,
                tree: PaneTree::new(pane),
            });
            ws.active = ws.tabs.len() - 1;
        }
        Ok(())
    }

    fn cycle_tab(&mut self, delta: i32) {
        let Some(ws) = self.workspaces.get_mut(self.active_ws) else {
            return;
        };
        if ws.tabs.is_empty() {
            return;
        }
        let len = ws.tabs.len() as i32;
        ws.active = (ws.active as i32 + delta).rem_euclid(len) as usize;
    }

    fn close_tab(&mut self) {
        let Some(ws) = self.workspaces.get_mut(self.active_ws) else {
            return;
        };
        if ws.tabs.is_empty() {
            self.close_workspace();
            return;
        }
        ws.tabs.remove(ws.active);
        if ws.tabs.is_empty() {
            self.close_workspace();
            return;
        }
        if ws.active >= ws.tabs.len() {
            ws.active = ws.tabs.len() - 1;
        }
    }

    fn new_workspace(&mut self) -> Result<()> {
        if self.workspaces.len() >= MAX_WORKSPACES {
            return Ok(());
        }
        let root = canonicalize_dir(&self.focused_project_root());
        if let Some(index) = self
            .workspaces
            .iter()
            .position(|ws| canonicalize_dir(&ws.root) == root)
        {
            self.active_ws = index;
            self.refresh_active_branch();
            return Ok(());
        }
        let name = workspace_name(&root);
        let branch = crate::git::branch_label(&root);
        let pane = self.spawn_pane(self.content_area(), &root)?;
        self.workspaces.push(Workspace {
            name,
            root,
            branch,
            tabs: vec![Tab {
                name: None,
                tree: PaneTree::new(pane),
            }],
            active: 0,
        });
        self.active_ws = self.workspaces.len() - 1;
        Ok(())
    }

    fn cycle_workspace(&mut self, delta: i32) {
        if self.workspaces.is_empty() {
            return;
        }
        let len = self.workspaces.len() as i32;
        self.active_ws = (self.active_ws as i32 + delta).rem_euclid(len) as usize;
        self.refresh_active_branch();
    }

    fn close_workspace(&mut self) {
        if self.workspaces.is_empty() {
            self.running = false;
            return;
        }
        self.workspaces.remove(self.active_ws);
        if self.workspaces.is_empty() {
            self.running = false;
            return;
        }
        if self.active_ws >= self.workspaces.len() {
            self.active_ws = self.workspaces.len() - 1;
        }
    }

    fn focus_dir(&mut self, dir: FocusDir) {
        let area = self.content_area();
        if let Some(tab) = self.active_tab_mut() {
            tab.tree.focus_dir(dir, area);
        }
    }

    fn grow_focused(&mut self, amount: i16) {
        if let Some(tab) = self.active_tab_mut() {
            tab.tree.grow_focused(amount);
        }
    }

    fn scroll_focused(&mut self, delta: i32) {
        if let Some(shell) = self.focused_shell() {
            shell.scroll_by(delta);
        }
    }

    fn reap(&mut self) -> Result<()> {
        let mut ws_index = 0;
        while ws_index < self.workspaces.len() {
            let mut tab_index = 0;
            while tab_index < self.workspaces[ws_index].tabs.len() {
                if self.workspaces[ws_index].tabs[tab_index].tree.reap()? {
                    tab_index += 1;
                } else {
                    self.workspaces[ws_index].tabs.remove(tab_index);
                    let active = &mut self.workspaces[ws_index].active;
                    if *active > tab_index {
                        *active = active.saturating_sub(1);
                    }
                }
            }

            if self.workspaces[ws_index].tabs.is_empty() {
                self.workspaces.remove(ws_index);
                if self.active_ws > ws_index {
                    self.active_ws = self.active_ws.saturating_sub(1);
                }
            } else {
                let ws = &mut self.workspaces[ws_index];
                if ws.active >= ws.tabs.len() {
                    ws.active = ws.tabs.len() - 1;
                }
                ws_index += 1;
            }
        }

        if self.workspaces.is_empty() {
            self.running = false;
        } else if self.active_ws >= self.workspaces.len() {
            self.active_ws = self.workspaces.len() - 1;
        }

        Ok(())
    }

    fn sync_sizes(&mut self) -> Result<()> {
        let area = self.content_area();
        if let Some(tab) = self.active_tab_mut() {
            tab.tree.sync_sizes(area)?;
        }
        Ok(())
    }

    fn focused_shell(&mut self) -> Option<&mut crate::terminal::Shell> {
        self.active_tab_mut()?.tree.focused_shell_mut()
    }

    fn spawn_pane(&mut self, area: Rect, cwd: &Path) -> Result<Pane> {
        let id = self.next_id;
        self.next_id += 1;
        Pane::spawn(id, pty_size_from_rect(area), cwd)
    }

    fn spawn_program_pane(
        &mut self,
        area: Rect,
        cwd: &Path,
        program: &str,
        args: &[String],
    ) -> Result<Pane> {
        let id = self.next_id;
        self.next_id += 1;
        Pane::spawn_program(id, pty_size_from_rect(area), cwd, program, args)
    }

    fn launch_ai(&mut self) -> Result<()> {
        match first_ai_cli() {
            Some(program) => self.launch_named(program),
            None => Ok(()),
        }
    }

    fn launch_named(&mut self, program: &str) -> Result<()> {
        self.launch_program(program, &[], &self.start_cwd.clone())
    }

    fn launch_program(&mut self, program: &str, args: &[String], cwd: &Path) -> Result<()> {
        if !command_exists(program) {
            return Ok(());
        }
        let area = self.content_area();
        let cwd = cwd.to_path_buf();
        let spawn_area = {
            let Some(tab) = self.active_tab() else {
                return Ok(());
            };
            if tab.tree.pane_count() >= MAX_PANES {
                return Ok(());
            }
            let focused = tab.tree.focused;
            tab.tree
                .areas(area)
                .iter()
                .find(|(id, _)| *id == focused)
                .map(|(_, rect)| half_rect(*rect, SplitDir::Columns))
                .unwrap_or(area)
        };
        let pane = self.spawn_program_pane(spawn_area, &cwd, program, args)?;
        if let Some(tab) = self.active_tab_mut() {
            tab.tree.split_focused(SplitDir::Columns, pane);
        }
        Ok(())
    }

    fn rename_tab(&mut self, args: &[String]) {
        let name = args.join(" ");
        let Some(tab) = self.active_tab_mut() else {
            return;
        };
        tab.name = if name.is_empty() { None } else { Some(name) };
    }

    fn focused_project_root(&self) -> PathBuf {
        self.active_tab()
            .and_then(|tab| tab.tree.panes.get(&tab.tree.focused))
            .and_then(|pane| pane.cwd())
            .map(|cwd| workspace_root_from(&cwd))
            .unwrap_or_else(|| self.active_root())
    }

    fn refresh_active_branch(&mut self) {
        if let Some(ws) = self.workspaces.get_mut(self.active_ws) {
            ws.branch = crate::git::branch_label(&ws.root);
        }
    }

    fn open_default_workspace(&mut self) -> Result<()> {
        let root = canonicalize_dir(&workspace_root_from(&self.start_cwd));
        let name = workspace_name(&root);
        let branch = crate::git::branch_label(&root);
        let cwd = self.start_cwd.clone();
        let pane = self.spawn_pane(self.content_area(), &cwd)?;
        self.workspaces.push(Workspace {
            name,
            root,
            branch,
            tabs: vec![Tab {
                name: None,
                tree: PaneTree::new(pane),
            }],
            active: 0,
        });
        self.active_ws = self.workspaces.len() - 1;
        Ok(())
    }

    fn snapshot(&self) -> Session {
        Session {
            active_workspace: self.active_ws,
            workspaces: self
                .workspaces
                .iter()
                .map(|ws| SavedWorkspace {
                    name: ws.name.clone(),
                    root: ws.root.clone(),
                    active_tab: ws.active,
                    tabs: ws
                        .tabs
                        .iter()
                        .map(|tab| {
                            let keep: HashSet<u64> = tab
                                .tree
                                .panes
                                .iter()
                                .filter(|(_, pane)| pane.is_shell())
                                .map(|(id, _)| *id)
                                .collect();
                            let persistable: Vec<u64> = tab
                                .tree
                                .leaf_ids()
                                .into_iter()
                                .filter(|id| keep.contains(id))
                                .collect();
                            let focused = persistable
                                .iter()
                                .position(|id| *id == tab.tree.focused)
                                .unwrap_or(0);
                            SavedTab {
                                focused,
                                name: tab.name.clone(),
                                tree: SavedNode::from_live(&tab.tree.root, &keep)
                                    .unwrap_or(SavedNode::Leaf { cwd: None }),
                            }
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    fn restore_session(&mut self, session: Session) -> Result<()> {
        let area = self.content_area();
        for saved_ws in session.workspaces {
            if !saved_ws.root.is_dir() {
                continue;
            }
            let root = canonicalize_dir(&saved_ws.root);

            let mut tabs = Vec::new();
            for saved_tab in saved_ws.tabs {
                let Some(tab) = self.restore_tab(&root, area, saved_tab)? else {
                    continue;
                };
                tabs.push(tab);
            }
            if tabs.is_empty() {
                continue;
            }
            let active = saved_ws.active_tab.min(tabs.len() - 1);
            let branch = crate::git::branch_label(&root);
            self.workspaces.push(Workspace {
                name: saved_ws.name,
                root,
                branch,
                tabs,
                active,
            });
        }

        if !self.workspaces.is_empty() {
            self.active_ws = session.active_workspace.min(self.workspaces.len() - 1);
            self.refresh_active_branch();
        }
        Ok(())
    }

    fn restore_tab(
        &mut self,
        _root: &Path,
        area: Rect,
        saved_tab: SavedTab,
    ) -> Result<Option<Tab>> {
        let count = saved_tab.tree.leaf_count().max(1);
        let cwd = self.start_cwd.clone();
        let mut ordered = Vec::with_capacity(count);
        let mut panes = HashMap::new();
        for _ in 0..count {
            let pane = self.spawn_pane(area, &cwd)?;
            ordered.push(pane.id);
            panes.insert(pane.id, pane);
        }

        let mut ids = ordered.iter().copied();
        let live = match saved_tab.tree.to_live(&mut ids) {
            Ok(node) => node,
            Err(_) => Node::Leaf(ordered[0]),
        };
        let focused = ordered
            .get(saved_tab.focused.min(ordered.len() - 1))
            .copied()
            .unwrap_or(ordered[0]);

        Ok(Some(Tab {
            name: saved_tab.name,
            tree: PaneTree::from_parts(live, panes, focused),
        }))
    }

    fn status_text(&self, ws: &Workspace, tab: &Tab) -> String {
        format!(
            " {} · {} · tab {} · {} panes · {} comandos · {} sale ",
            compact_path(&self.start_cwd),
            workspace_label(ws).trim(),
            ws.active,
            tab.tree.pane_count(),
            self.config.prefix.label(),
            self.config.quit.label(),
        )
    }

    fn content_area(&self) -> Rect {
        body_area(term_rect(self.term_size))
    }

    fn active_root(&self) -> PathBuf {
        self.workspaces
            .get(self.active_ws)
            .map(|ws| ws.root.clone())
            .unwrap_or_else(|| {
                workspace_root_from(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            })
    }

    fn active_tab(&self) -> Option<&Tab> {
        let ws = self.workspaces.get(self.active_ws)?;
        ws.tabs.get(ws.active)
    }

    fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        let ws = self.workspaces.get_mut(self.active_ws)?;
        ws.tabs.get_mut(ws.active)
    }
}

fn workspace_root_from(dir: &Path) -> PathBuf {
    let dir = canonicalize_dir(dir);
    crate::git::toplevel(&dir)
        .map(|root| canonicalize_dir(&root))
        .filter(|root| root.is_dir())
        .unwrap_or(dir)
}

fn canonicalize_dir(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn workspace_label(ws: &Workspace) -> String {
    match &ws.branch {
        Some(branch) => format!(" {}:{} ", ws.name, branch),
        None => format!(" {} ", ws.name),
    }
}

fn tab_label(tab: &Tab, index: usize) -> String {
    if let Some(name) = &tab.name {
        return format!(" {name} ");
    }
    let focused = tab
        .tree
        .panes
        .get(&tab.tree.focused)
        .map(Pane::title)
        .unwrap_or("?");
    let count = tab.tree.pane_count();
    if count > 1 {
        format!(" {index}:{focused}×{count} ")
    } else {
        format!(" {index}:{focused} ")
    }
}

fn compact_path(path: &Path) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        if let Ok(stripped) = path.strip_prefix(&home) {
            if stripped.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

fn chrome(area: Rect) -> [Rect; 4] {
    Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area)
}

fn index_at(x: u16, area: Rect, count: usize) -> Option<usize> {
    if count == 0 || x < area.x || x >= area.x.saturating_add(area.width) {
        return None;
    }
    let rel = u32::from(x.saturating_sub(area.x));
    let width = u32::from(area.width.max(1));
    Some(((rel * count as u32) / width).min(count as u32 - 1) as usize)
}

fn first_ai_cli() -> Option<&'static str> {
    ["codex", "claude", "opencode", "gemini", "cline"]
        .into_iter()
        .find(|name| command_exists(name))
}

fn command_exists(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let candidate = dir.join(name);
            candidate.is_file()
        })
    })
}

fn workspace_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ws")
        .to_string()
}

fn term_rect(size: Size) -> Rect {
    Rect::new(0, 0, size.width, size.height)
}

fn body_area(area: Rect) -> Rect {
    chrome(area)[2]
}

fn half_rect(area: Rect, dir: SplitDir) -> Rect {
    let chunks = match dir {
        SplitDir::Columns => {
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area)
        }
        SplitDir::Rows => {
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area)
        }
    };
    chunks[1]
}
