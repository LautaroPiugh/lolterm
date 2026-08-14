use std::path::Path;

use color_eyre::Result;
use portable_pty::PtySize;

use crate::terminal::Shell;

pub struct Pane {
    pub id: u64,
    pub(crate) shell: Shell,
}

impl Pane {
    pub fn spawn(id: u64, size: PtySize, cwd: &Path) -> Result<Self> {
        Ok(Self {
            id,
            shell: Shell::spawn(size, cwd)?,
        })
    }
}
