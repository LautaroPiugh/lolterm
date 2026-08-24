use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

pub struct BytePty {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    child_exited: bool,
    exit_code: Option<u32>,
}

impl BytePty {
    pub fn spawn(
        id: u64,
        size: PtySize,
        cwd: &Path,
        program: Option<&str>,
        args: &[String],
        env: &[(String, String)],
        tx: Sender<(u64, Vec<u8>)>,
    ) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(size)
            .map_err(|err| eyre!("failed to open pty: {err:#}"))?;

        let mut cmd = match program {
            Some(program) => {
                let mut cmd = CommandBuilder::new(program);
                for arg in args {
                    cmd.arg(arg);
                }
                cmd
            }
            None => CommandBuilder::new_default_prog(),
        };
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.cwd(cwd);

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|err| eyre!("failed to spawn: {err:#}"))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| eyre!("failed to clone pty reader: {err:#}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| eyre!("failed to take pty writer: {err:#}"))?;

        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = tx.send((id, Vec::new()));
                        break;
                    }
                    Ok(n) => {
                        if tx.send((id, buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = tx.send((id, Vec::new()));
                        break;
                    }
                }
            }
        });

        Ok(Self {
            master: pair.master,
            child,
            writer,
            child_exited: false,
            exit_code: None,
        })
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let size = PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        };
        self.master
            .resize(size)
            .map_err(|err| eyre!("failed to resize pty: {err:#}"))
    }

    pub fn write_input(&mut self, bytes: &[u8]) -> Result<()> {
        if self.child_exited {
            return Ok(());
        }
        match self.writer.write_all(bytes) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {
                self.child_exited = true;
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        }
        let _ = self.writer.flush();
        Ok(())
    }

    pub fn child_exited(&self) -> bool {
        self.child_exited
    }

    pub fn exit_code(&self) -> Option<u32> {
        self.exit_code
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
            Ok(Some(status)) => {
                self.exit_code = Some(status.exit_code());
                self.child_exited = true;
            }
            Err(_) => self.child_exited = true,
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

/// portable-pty ya hace `setsid` en Unix. Si hay líder de grupo, hay que
/// avisar a todo el session (nvim, ssh, …), no sólo al pid del hijo.
pub fn should_signal_group(pgid: Option<i32>) -> bool {
    pgid.is_some_and(|pid| pid > 1)
}

impl Drop for BytePty {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if should_signal_group(self.master.process_group_leader())
                && let Some(pgid) = self.master.process_group_leader()
            {
                unsafe {
                    libc::killpg(pgid, libc::SIGHUP);
                }
            }
        }
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_signal_skips_init_and_none() {
        assert!(!should_signal_group(None));
        assert!(!should_signal_group(Some(0)));
        assert!(!should_signal_group(Some(1)));
        assert!(should_signal_group(Some(42)));
    }
}
