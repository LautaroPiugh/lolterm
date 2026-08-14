use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

use crate::commands::{CommandId, CommandSpec, filter_commands};
use crate::event::{encode_key, encode_mouse, is_prefix_key, is_quit_key};
use crate::pane::Pane;
use crate::session::{SavedNode, SavedTab, SavedWorkspace, Session};
use crate::tree::{FocusDir, Node, PaneTree, SplitDir, pty_size_from_rect};

const MAX_PANES: usize = 8;
const MAX_TABS: usize = 8;
const MAX_WORKSPACES: usize = 6;

struct Palette {
    query: String,
    selected: usize,
}

struct Tab {
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
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        let _ = crate::session::save(&self.snapshot());
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let [ws_bar, tab_bar, body] = chrome(frame.area());

        let ws_titles: Vec<Line> = self
            .workspaces
            .iter()
            .map(|ws| Line::from(workspace_label(ws)))
            .collect();
        frame.render_widget(
            Tabs::new(ws_titles)
                .select(self.active_ws)
                .highlight_style(Style::default().fg(Color::Yellow))
                .style(Style::default().fg(Color::DarkGray)),
            ws_bar,
        );

        let Some(ws) = self.workspaces.get(self.active_ws) else {
            return;
        };

        let tab_titles: Vec<Line> = ws
            .tabs
            .iter()
            .enumerate()
            .map(|(index, _)| Line::from(format!(" {index} ")))
            .collect();
        frame.render_widget(
            Tabs::new(tab_titles)
                .select(ws.active)
                .highlight_style(Style::default().fg(Color::Cyan))
                .style(Style::default().fg(Color::DarkGray)),
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
            let name = pane.title();
            let title = if focused {
                format!(" {marker} {name} · C-b comandos · C-q sale ")
            } else {
                format!(" {marker} {name} ")
            };
            let border = if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            frame.render_widget(
                PseudoTerminal::new(parser.screen())
                    .block(Block::bordered().border_style(border).title(title)),
                area,
            );
        }

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
        if is_quit_key(key) {
            self.running = false;
            return Ok(());
        }
        if self.palette.is_some() {
            return self.handle_palette_key(key);
        }
        if is_prefix_key(key) {
            self.palette = Some(Palette {
                query: "/".to_string(),
                selected: 0,
            });
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
        if is_prefix_key(key) || key.code == KeyCode::Esc {
            self.palette = None;
            return Ok(());
        }

        match key.code {
            KeyCode::Enter => {
                if let Some(command) = self.selected_command() {
                    let id = command.id;
                    self.palette = None;
                    self.run_command(id)?;
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

    fn run_command(&mut self, id: CommandId) -> Result<()> {
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
            CommandId::NewWorkspace => self.new_workspace()?,
            CommandId::NextWorkspace => self.cycle_workspace(1),
            CommandId::CloseWorkspace => self.close_workspace(),
            CommandId::LaunchAi => self.launch_ai()?,
            CommandId::LaunchCodex => self.launch_named("codex")?,
            CommandId::LaunchClaude => self.launch_named("claude")?,
            CommandId::LaunchOpencode => self.launch_named("opencode")?,
            CommandId::LaunchGemini => self.launch_named("gemini")?,
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
                    Line::from(text).style(Style::default().fg(Color::Black).bg(Color::Cyan))
                } else {
                    Line::from(text)
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), list);
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        let [ws_bar, tab_bar, body] = chrome(term_rect(self.term_size));
        let pos = Position::new(mouse.column, mouse.row);

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
        let root = self.active_root();
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

        let pane = self.spawn_pane(spawn_area, &root)?;
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
        let root = self.active_root();
        let area = self.content_area();
        if self
            .workspaces
            .get(self.active_ws)
            .is_none_or(|ws| ws.tabs.len() >= MAX_TABS)
        {
            return Ok(());
        }
        let pane = self.spawn_pane(area, &root)?;
        if let Some(ws) = self.workspaces.get_mut(self.active_ws) {
            ws.tabs.push(Tab {
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
        let branch = git_branch(&root);
        let pane = self.spawn_pane(self.content_area(), &root)?;
        self.workspaces.push(Workspace {
            name,
            root,
            branch,
            tabs: vec![Tab {
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

    fn spawn_program_pane(&mut self, area: Rect, cwd: &Path, program: &str) -> Result<Pane> {
        let id = self.next_id;
        self.next_id += 1;
        Pane::spawn_program(id, pty_size_from_rect(area), cwd, program)
    }

    fn launch_ai(&mut self) -> Result<()> {
        match first_ai_cli() {
            Some(program) => self.launch_named(program),
            None => Ok(()),
        }
    }

    fn launch_named(&mut self, program: &str) -> Result<()> {
        if !command_exists(program) {
            return Ok(());
        }
        let area = self.content_area();
        let root = self.active_root();
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
        let pane = self.spawn_program_pane(spawn_area, &root, program)?;
        if let Some(tab) = self.active_tab_mut() {
            tab.tree.split_focused(SplitDir::Columns, pane);
        }
        Ok(())
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
            ws.branch = git_branch(&ws.root);
        }
    }

    fn open_default_workspace(&mut self) -> Result<()> {
        let root = canonicalize_dir(&workspace_root_from(
            &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        ));
        let name = workspace_name(&root);
        let branch = git_branch(&root);
        let pane = self.spawn_pane(self.content_area(), &root)?;
        self.workspaces.push(Workspace {
            name,
            root,
            branch,
            tabs: vec![Tab {
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
                            let cwds: HashMap<u64, PathBuf> = tab
                                .tree
                                .panes
                                .iter()
                                .filter(|(_, pane)| pane.is_shell())
                                .map(|(id, pane)| {
                                    (*id, pane.cwd().unwrap_or_else(|| ws.root.clone()))
                                })
                                .collect();
                            let persistable: Vec<u64> = tab
                                .tree
                                .leaf_ids()
                                .into_iter()
                                .filter(|id| cwds.contains_key(id))
                                .collect();
                            let focused = persistable
                                .iter()
                                .position(|id| *id == tab.tree.focused)
                                .unwrap_or(0);
                            SavedTab {
                                focused,
                                tree: SavedNode::from_live(&tab.tree.root, &cwds).unwrap_or(
                                    SavedNode::Leaf {
                                        cwd: Some(ws.root.clone()),
                                    },
                                ),
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
            let branch = git_branch(&root);
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

    fn restore_tab(&mut self, root: &Path, area: Rect, saved_tab: SavedTab) -> Result<Option<Tab>> {
        let cwds = saved_tab.tree.leaf_cwds(root);
        let mut ordered = Vec::with_capacity(cwds.len().max(1));
        let mut panes = HashMap::new();
        for cwd in cwds {
            let pane = self.spawn_pane(area, &cwd)?;
            ordered.push(pane.id);
            panes.insert(pane.id, pane);
        }
        if ordered.is_empty() {
            let pane = self.spawn_pane(area, root)?;
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
            tree: PaneTree::from_parts(live, panes, focused),
        }))
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
    std::process::Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| canonicalize_dir(Path::new(stdout.trim())))
        .filter(|root| root.is_dir())
        .unwrap_or(dir)
}

fn git_branch(root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args([
            "-C",
            &root.to_string_lossy(),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| stdout.trim().to_string())
        .filter(|branch| !branch.is_empty())
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

fn chrome(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
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
    ["codex", "claude", "opencode", "gemini"]
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
