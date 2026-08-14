use color_eyre::Result;
use color_eyre::eyre::eyre;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

pub struct Shell {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    child_exited: bool,
}

impl Shell {
    pub fn spawn() -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize::default())
            .map_err(|err| eyre!("failed to open pty: {err:#}"))?;

        let mut cmd = CommandBuilder::new_default_prog();
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|err| eyre!("failed to spawn user shell: {err:#}"))?;

        Ok(Self {
            master: pair.master,
            child,
            child_exited: false,
        })
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    pub fn size(&self) -> Result<PtySize> {
        self.master
            .get_size()
            .map_err(|err| eyre!("failed to read pty size: {err:#}"))
    }

    pub fn child_exited(&self) -> bool {
        self.child_exited
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
