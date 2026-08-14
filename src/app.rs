use std::time::Duration;

use color_eyre::Result;
use portable_pty::PtySize;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Size;
use ratatui::widgets::Block;
use ratatui::{DefaultTerminal, Frame};
use tui_term::widget::PseudoTerminal;

use crate::terminal::Shell;

pub struct App {
    running: bool,
    shell: Shell,
}

impl App {
    pub fn new(term_size: Size) -> Result<Self> {
        Ok(Self {
            running: true,
            shell: Shell::spawn(pty_size_from_term(term_size))?,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while self.running {
            self.shell.poll_exit()?;
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let parser = self.shell.parser();
        let pid = self
            .shell
            .process_id()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "?".to_string());
        let block = Block::bordered().title(format!(" LolTerm · pid {pid} · q / Ctrl-C sale "));
        let term = PseudoTerminal::new(parser.screen()).block(block);
        frame.render_widget(term, frame.area());
    }

    fn handle_events(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(16))? {
            return Ok(());
        }

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') => self.running = false,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.running = false;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn pty_size_from_term(size: Size) -> PtySize {
    PtySize {
        rows: size.height.saturating_sub(2).max(1),
        cols: size.width.saturating_sub(2).max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}
