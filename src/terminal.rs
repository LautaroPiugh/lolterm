use std::io::Read;
use std::sync::{Arc, PoisonError, RwLock};

use color_eyre::Result;
use color_eyre::eyre::eyre;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tui_term::vt100;

pub struct Shell {
    _master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    parser: Arc<RwLock<vt100::Parser>>,
    child_exited: bool,
}

impl Shell {
    pub fn spawn(size: PtySize) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(size)
            .map_err(|err| eyre!("failed to open pty: {err:#}"))?;

        let mut cmd = CommandBuilder::new_default_prog();
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|err| eyre!("failed to spawn user shell: {err:#}"))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| eyre!("failed to clone pty reader: {err:#}"))?;

        let parser = Arc::new(RwLock::new(vt100::Parser::new(size.rows, size.cols, 0)));
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
            _master: pair.master,
            child,
            parser,
            child_exited: false,
        })
    }

    pub fn parser(&self) -> std::sync::RwLockReadGuard<'_, vt100::Parser> {
        self.parser.read().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    pub fn poll_exit(&mut self) -> Result<()> {
        if self.child_exited {
            return Ok(());
        }

        if self.child.try_wait()?.is_some() {
            self.child_exited = true;
        }

        Ok(())
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
