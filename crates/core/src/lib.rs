/// Versión global de LoLTerm. Sale de `workspace.package.version` al compilar
/// el crate; Release Please la sube junto con `apps/desktop/package.json`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod agents;
pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod ctl;
pub mod ext;
pub mod files;
pub mod git;
pub mod host;
pub mod http;
pub mod hud;
pub mod inspect;
pub mod keys;
pub mod layout;
pub mod music;
pub mod mux;
pub mod presets;
pub mod pty;
pub mod quota;
pub mod registry;
pub mod rest;
pub mod session;
pub mod sink;
pub mod ssh;
pub mod tailscale;
pub mod workspaces;

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_semver_triplet() {
        let parts: Vec<_> = crate::VERSION.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "expected MAJOR.MINOR.PATCH, got {}",
            crate::VERSION
        );
        assert!(
            parts
                .iter()
                .all(|part| part.chars().all(|ch| ch.is_ascii_digit()))
        );
    }
}
