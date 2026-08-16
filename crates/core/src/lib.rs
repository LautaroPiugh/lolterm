/// Versión global de LoLTerm. Sale de `workspace.package.version` al compilar
/// el crate; Release Please la sube junto con `apps/desktop/package.json`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod commands;
pub mod config;
pub mod files;
pub mod git;
pub mod keys;
pub mod layout;
pub mod mux;
pub mod presets;
pub mod pty;
pub mod session;
pub mod ssh;
pub mod tailscale;

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
