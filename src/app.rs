use std::time::Duration;

use color_eyre::Result;
use portable_pty::PtySize;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
            if self.shell.child_exited() {
                self.running = false;
                break;
            }
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
        let block = Block::bordered().title(format!(" LolTerm · pid {pid} · Ctrl-Q sale "));
        let term = PseudoTerminal::new(parser.screen()).block(block);
        frame.render_widget(term, frame.area());
    }

    fn handle_events(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(16))? {
            return Ok(());
        }

        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if is_quit_key(key) {
                    self.running = false;
                    return Ok(());
                }
                if let Some(bytes) = encode_key(key) {
                    self.shell.write_input(&bytes)?;
                }
            }
            Event::Paste(text) => self.shell.write_input(text.as_bytes())?,
            _ => {}
        }

        Ok(())
    }
}

fn is_quit_key(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn encode_key(key: KeyEvent) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    let mut bytes = match key.code {
        KeyCode::Char(ch) if ctrl => encode_ctrl_char(ch),
        KeyCode::Char(ch) => {
            let mut buf = [0u8; 4];
            ch.encode_utf8(&mut buf).as_bytes().to_vec()
        }
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        _ => return None,
    };

    if alt {
        bytes.insert(0, 0x1b);
    }

    Some(bytes)
}

fn encode_ctrl_char(ch: char) -> Vec<u8> {
    let lower = ch.to_ascii_lowercase();
    if lower.is_ascii_alphabetic() {
        vec![lower as u8 & 0x1f]
    } else if ch == ' ' {
        vec![0]
    } else {
        let mut buf = [0u8; 4];
        ch.encode_utf8(&mut buf).as_bytes().to_vec()
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
