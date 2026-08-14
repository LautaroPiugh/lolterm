use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use color_eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent};
use ratatui::layout::{Constraint, Layout, Position, Rect, Size};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Tabs};
use ratatui::{DefaultTerminal, Frame};
use tui_term::widget::PseudoTerminal;

use crate::event::{encode_key, encode_mouse, is_prefix_key, is_quit_key};
use crate::pane::Pane;
use crate::tree::{FocusDir, PaneTree, SplitDir, pty_size_from_rect};

const MAX_PANES: usize = 8;
const MAX_TABS: usize = 8;
const MAX_WORKSPACES: usize = 6;
const PREFIX_TIMEOUT: Duration = Duration::from_millis(1000);

struct Tab {
    tree: PaneTree,
}

struct Workspace {
    name: String,
    root: PathBuf,
    tabs: Vec<Tab>,
    active: usize,
}

pub struct App {
    running: bool,
    workspaces: Vec<Workspace>,
    active_ws: usize,
    next_id: u64,
    prefix: bool,
    prefix_at: Option<Instant>,
    term_size: Size,
}

impl App {
    pub fn new(term_size: Size) -> Result<Self> {
        let root = workspace_root();
        let name = workspace_name(&root);
        let mut app = Self {
            running: true,
            workspaces: Vec::new(),
            active_ws: 0,
            next_id: 1,
            prefix: false,
            prefix_at: None,
            term_size,
        };
        let pane = app.spawn_pane(body_area(term_rect(term_size)), &root)?;
        app.workspaces.push(Workspace {
            name,
            root,
            tabs: vec![Tab {
                tree: PaneTree::new(pane),
            }],
            active: 0,
        });
        Ok(app)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while self.running {
            self.expire_prefix();
            self.reap()?;
            if !self.running {
                break;
            }
            self.term_size = terminal.size()?;
            self.sync_sizes()?;
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let [ws_bar, tab_bar, body] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(frame.area());

        let ws_titles: Vec<Line> = self
            .workspaces
            .iter()
            .map(|ws| Line::from(format!(" {} ", ws.name)))
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
            let pid = pane
                .shell
                .process_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "?".to_string());
            let marker = if focused && self.prefix {
                "PREFIX"
            } else if focused {
                "*"
            } else {
                " "
            };
            let title = if focused {
                format!(" {marker} {pid} · C-b %/\" +/− x o ←↑↓→ · c n p · w W · C-q ")
            } else {
                format!(" {marker} {pid} ")
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
    }

    fn handle_events(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(16))? {
            return Ok(());
        }

        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.handle_key(key)?,
            Event::Mouse(mouse) => self.handle_mouse(mouse)?,
            Event::Paste(text) => {
                if let Some(shell) = self.focused_shell() {
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
        if self.prefix {
            self.handle_prefix(key)?;
            return Ok(());
        }
        if is_prefix_key(key) {
            self.prefix = true;
            self.prefix_at = Some(Instant::now());
            return Ok(());
        }
        if let Some(bytes) = encode_key(key)
            && let Some(shell) = self.focused_shell()
        {
            shell.write_input(&bytes)?;
        }
        Ok(())
    }

    fn handle_prefix(&mut self, key: KeyEvent) -> Result<()> {
        self.clear_prefix();

        if is_prefix_key(key) {
            if let Some(shell) = self.focused_shell() {
                shell.write_input(&[0x02])?;
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Char('%') => self.split(SplitDir::Columns)?,
            KeyCode::Char('"') => self.split(SplitDir::Rows)?,
            KeyCode::Char('+') | KeyCode::Char('=') => self.grow_focused(5),
            KeyCode::Char('-') => self.grow_focused(-5),
            KeyCode::Char('o') => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.tree.focus_next();
                }
            }
            KeyCode::Char('x') => self.close_pane(),
            KeyCode::Char('c') => self.new_tab()?,
            KeyCode::Char('n') => self.cycle_tab(1),
            KeyCode::Char('p') => self.cycle_tab(-1),
            KeyCode::Char('&') => self.close_tab(),
            KeyCode::Char('w') => self.cycle_workspace(1),
            KeyCode::Char('W') => self.new_workspace()?,
            KeyCode::PageUp => self.scroll_focused(5),
            KeyCode::PageDown => self.scroll_focused(-5),
            KeyCode::Left => self.focus_dir(FocusDir::Left),
            KeyCode::Right => self.focus_dir(FocusDir::Right),
            KeyCode::Up => self.focus_dir(FocusDir::Up),
            KeyCode::Down => self.focus_dir(FocusDir::Down),
            _ => {}
        }

        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        let body = self.content_area();
        let Some(tab) = self.active_tab_mut() else {
            return Ok(());
        };

        let Some((id, area)) = tab
            .tree
            .areas(body)
            .into_iter()
            .find(|(_, rect)| rect.contains(Position::new(mouse.column, mouse.row)))
        else {
            return Ok(());
        };

        tab.tree.focused = id;
        let inner = Rect {
            x: area.x.saturating_add(1),
            y: area.y.saturating_add(1),
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        match mouse.kind {
            event::MouseEventKind::ScrollUp => {
                if let Some(pane) = tab.tree.panes.get(&id) {
                    pane.shell.scroll_by(3);
                }
            }
            event::MouseEventKind::ScrollDown => {
                if let Some(pane) = tab.tree.panes.get(&id) {
                    pane.shell.scroll_by(-3);
                }
            }
            _ if inner.contains(Position::new(mouse.column, mouse.row)) => {
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
        let root = workspace_root();
        if let Some(index) = self.workspaces.iter().position(|ws| ws.root == root) {
            self.active_ws = index;
            return Ok(());
        }
        let name = workspace_name(&root);
        let pane = self.spawn_pane(self.content_area(), &root)?;
        self.workspaces.push(Workspace {
            name,
            root,
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

    fn expire_prefix(&mut self) {
        if self.prefix
            && self
                .prefix_at
                .is_some_and(|at| at.elapsed() >= PREFIX_TIMEOUT)
        {
            self.clear_prefix();
        }
    }

    fn clear_prefix(&mut self) {
        self.prefix = false;
        self.prefix_at = None;
    }

    fn content_area(&self) -> Rect {
        body_area(term_rect(self.term_size))
    }

    fn active_root(&self) -> PathBuf {
        self.workspaces
            .get(self.active_ws)
            .map(|ws| ws.root.clone())
            .unwrap_or_else(workspace_root)
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

fn workspace_root() -> PathBuf {
    std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| PathBuf::from(stdout.trim()))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
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
    Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas::<3>(area)[2]
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
