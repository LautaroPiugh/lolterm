use color_eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Alignment;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::terminal::Shell;

pub struct App {
    running: bool,
    shell: Shell,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            running: true,
            shell: Shell::spawn()?,
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
        let block = Block::bordered().title(" LolTerm ");
        let inner = block.inner(frame.area());
        frame.render_widget(block, frame.area());

        let pid = self
            .shell
            .process_id()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let size = self
            .shell
            .size()
            .map(|s| format!("{}x{}", s.cols, s.rows))
            .unwrap_or_else(|_| "unknown".to_string());

        let status = if self.shell.child_exited() {
            "shell exited"
        } else {
            "shell running"
        };

        let text = vec![
            Line::from("PTY + shell (incremento 2)"),
            Line::from(""),
            Line::from(format!("status: {status}")),
            Line::from(format!("pid: {pid}")),
            Line::from(format!("pty size: {size}")),
            Line::from(""),
            Line::from("Aun no hay I/O: las teclas no van al shell."),
            Line::from("Presiona 'q' o Ctrl-C para salir"),
        ];
        let paragraph = Paragraph::new(text).alignment(Alignment::Center);
        frame.render_widget(paragraph, inner);
    }

    fn handle_events(&mut self) -> Result<()> {
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
