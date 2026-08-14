use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use color_eyre::Result;
use color_eyre::eyre::eyre;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tui_term::vt100;

const SCROLLBACK: usize = 2000;

pub struct Shell {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    parser: Arc<RwLock<vt100::Parser>>,
    child_exited: bool,
}

impl Shell {
    pub fn spawn(size: PtySize, cwd: &Path) -> Result<Self> {
        Self::spawn_cmd(size, cwd, CommandBuilder::new_default_prog())
    }

    pub fn spawn_program(size: PtySize, cwd: &Path, program: &str) -> Result<Self> {
        Self::spawn_cmd(size, cwd, CommandBuilder::new(program))
    }

    fn spawn_cmd(size: PtySize, cwd: &Path, mut cmd: CommandBuilder) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(size)
            .map_err(|err| eyre!("failed to open pty: {err:#}"))?;

        cmd.env("TERM", "xterm-256color");
        cmd.cwd(cwd);

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|err| eyre!("failed to spawn user shell: {err:#}"))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| eyre!("failed to clone pty reader: {err:#}"))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|err| eyre!("failed to take pty writer: {err:#}"))?;

        let parser = Arc::new(RwLock::new(vt100::Parser::new(
            size.rows, size.cols, SCROLLBACK,
        )));
        {
            let parser = Arc::clone(&parser);
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Ok(mut parser) = parser.write() {
                                parser.process(&buf[..n]);
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        Ok(Self {
            master: pair.master,
            child,
            writer,
            parser,
            child_exited: false,
        })
    }

    pub fn resize(&self, size: PtySize) -> Result<()> {
        let current = self
            .master
            .get_size()
            .map_err(|err| eyre!("failed to read pty size: {err:#}"))?;
        if current.rows == size.rows && current.cols == size.cols {
            return Ok(());
        }

        self.master
            .resize(size)
            .map_err(|err| eyre!("failed to resize pty: {err:#}"))?;

        let mut parser = self.parser.write().unwrap_or_else(PoisonError::into_inner);
        parser.screen_mut().set_size(size.rows, size.cols);

        Ok(())
    }

    pub fn write_input(&mut self, bytes: &[u8]) -> Result<()> {
        self.set_scrollback(0);
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn scroll_by(&self, delta: i32) {
        let mut parser = self.parser.write().unwrap_or_else(PoisonError::into_inner);
        let current = parser.screen().scrollback() as i32;
        parser
            .screen_mut()
            .set_scrollback(current.saturating_add(delta).max(0) as usize);
    }

    fn set_scrollback(&self, rows: usize) {
        let mut parser = self.parser.write().unwrap_or_else(PoisonError::into_inner);
        parser.screen_mut().set_scrollback(rows);
    }

    pub fn parser(&self) -> std::sync::RwLockReadGuard<'_, vt100::Parser> {
        self.parser.read().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn wants_mouse(&self) -> bool {
        self.parser().screen().mouse_protocol_mode() != vt100::MouseProtocolMode::None
    }

    pub fn child_exited(&self) -> bool {
        self.child_exited
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    pub fn cwd(&self) -> Option<PathBuf> {
        self.process_id().and_then(process_cwd)
    }

    pub fn poll_exit(&mut self) -> Result<()> {
        if self.child_exited {
            return Ok(());
        }

        match self.child.try_wait() {
            Ok(Some(_)) | Err(_) => self.child_exited = true,
            Ok(None) => {}
        }

        Ok(())
    }
}

fn process_cwd(pid: u32) -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let output = std::process::Command::new("lsof")
            .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout)
            .ok()?
            .lines()
            .find_map(|line| line.strip_prefix('n').map(PathBuf::from))
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
