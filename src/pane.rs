use std::path::{Path, PathBuf};

use color_eyre::Result;
use portable_pty::PtySize;

use crate::terminal::Shell;

pub struct Pane {
    pub id: u64,
    program: Option<String>,
    label: String,
    pub(crate) shell: Shell,
}

impl Pane {
    pub fn spawn_program(id: u64, size: PtySize, cwd: &Path, program: &str) -> Result<Self> {
        Ok(Self {
            id,
            program: Some(program.to_string()),
            label: program.to_string(),
            shell: Shell::spawn_program(size, cwd, program)?,
        })
    }

    pub fn spawn(id: u64, size: PtySize, cwd: &Path) -> Result<Self> {
        Ok(Self {
            id,
            program: None,
            label: default_shell_label(),
            shell: Shell::spawn(size, cwd)?,
        })
    }

    pub fn is_shell(&self) -> bool {
        self.program.is_none()
    }

    pub fn title(&self) -> &str {
        &self.label
    }

    pub fn cwd(&self) -> Option<PathBuf> {
        self.shell.cwd()
    }
}

fn default_shell_label() -> String {
    std::env::var("SHELL")
        .ok()
        .as_deref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("sh")
        .to_string()
}
