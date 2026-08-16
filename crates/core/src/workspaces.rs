use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config;
use crate::session::{self, Session, StartupCmd};

/// Definición portable de un workspace: identidad, no el layout vivo.
///
/// No incluye `env`: esos valores pueden ser secretos y quedan en `session.toml`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceDef {
    pub name: String,
    pub root: String,
    #[serde(default)]
    pub startup: Vec<StartupCmd>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Catalog {
    #[serde(default)]
    pub workspaces: Vec<WorkspaceDef>,
}

pub fn path() -> PathBuf {
    config::config_dir().join("workspaces.toml")
}

pub fn load() -> Catalog {
    let Ok(text) = std::fs::read_to_string(path()) else {
        return Catalog::default();
    };
    toml::from_str(&text).unwrap_or_default()
}

pub fn save(catalog: &Catalog) -> std::io::Result<()> {
    let path = path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(catalog).unwrap_or_default();
    std::fs::write(path, text)
}

pub fn expand_root(raw: &str) -> PathBuf {
    expand_root_with(raw, home_dir().as_deref())
}

pub fn compact_root(path: &Path) -> String {
    compact_root_with(path, home_dir().as_deref())
}

pub fn expand_root_with(raw: &str, home: Option<&Path>) -> PathBuf {
    let raw = raw.trim();
    if let Some(home) = home {
        if raw == "~" {
            return home.to_path_buf();
        }
        if let Some(rest) = raw.strip_prefix("~/") {
            return home.join(rest);
        }
    }
    PathBuf::from(raw)
}

pub fn compact_root_with(path: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home
        && let Ok(rel) = path.strip_prefix(home)
    {
        if rel.as_os_str().is_empty() {
            return "~".into();
        }
        return format!("~/{}", rel.display());
    }
    path.display().to_string()
}

pub fn canonical_root(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// El catálogo manda en nombre y startup. El session conserva tabs y env.
pub fn merge_into_session(session: &mut Session, catalog: &Catalog) {
    merge_into_session_with(session, catalog, home_dir().as_deref());
}

pub fn merge_into_session_with(session: &mut Session, catalog: &Catalog, home: Option<&Path>) {
    for def in &catalog.workspaces {
        let name = def.name.trim();
        if name.is_empty() || def.root.trim().is_empty() {
            continue;
        }
        let root = canonical_root(&expand_root_with(&def.root, home));
        if let Some(existing) = session.workspaces.iter_mut().find(|ws| ws.root == root) {
            existing.name = name.to_string();
            existing.startup = def.startup.clone();
        } else {
            session.workspaces.push(session::SavedWorkspace {
                name: name.to_string(),
                root,
                active_tab: 0,
                tabs: vec![],
                startup: def.startup.clone(),
                env: vec![],
            });
        }
    }
}

pub fn upsert_def(list: &mut Vec<WorkspaceDef>, def: WorkspaceDef) {
    let key = canonical_root(&expand_root(&def.root));
    if let Some(existing) = list
        .iter_mut()
        .find(|item| canonical_root(&expand_root(&item.root)) == key)
    {
        *existing = def;
    } else {
        list.push(def);
    }
}

pub fn upsert_identity(list: &mut Vec<WorkspaceDef>, def: WorkspaceDef) {
    let key = canonical_root(&expand_root(&def.root));
    if let Some(existing) = list
        .iter_mut()
        .find(|item| canonical_root(&expand_root(&item.root)) == key)
    {
        existing.name = def.name;
        existing.root = def.root;
        existing.startup = def.startup;
    } else {
        list.push(def);
    }
}

pub fn remove_root(list: &mut Vec<WorkspaceDef>, root: &Path) {
    let key = canonical_root(root);
    list.retain(|item| canonical_root(&expand_root(&item.root)) != key);
}

pub fn ensure_in_catalog(list: &mut Vec<WorkspaceDef>, name: &str, root: &Path) {
    let key = canonical_root(root);
    let compact = compact_root(root);
    if let Some(existing) = list
        .iter_mut()
        .find(|item| canonical_root(&expand_root(&item.root)) == key)
    {
        if existing.name.trim().is_empty() {
            existing.name = name.to_string();
        }
        return;
    }
    list.push(WorkspaceDef {
        name: name.to_string(),
        root: compact,
        startup: Vec::new(),
        notes: String::new(),
    });
}

pub fn notes_for(root: &Path) -> String {
    let key = canonical_root(root);
    load()
        .workspaces
        .into_iter()
        .find(|item| canonical_root(&expand_root(&item.root)) == key)
        .map(|item| item.notes)
        .unwrap_or_default()
}

pub fn name_from_root(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("workspace")
        .to_string()
}

pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "ws".into() } else { out }
}

pub fn catalog_from_saved(workspaces: &[session::SavedWorkspace]) -> Catalog {
    let mut catalog = load();
    for ws in workspaces {
        upsert_identity(
            &mut catalog.workspaces,
            WorkspaceDef {
                name: ws.name.clone(),
                root: compact_root(&ws.root),
                startup: ws.startup.clone(),
                notes: String::new(),
            },
        );
    }
    catalog
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{EnvVar, SavedWorkspace, StartupCmd};

    #[test]
    fn tilde_expands_and_compacts_against_home() {
        let home = Path::new("/home/lauta");
        assert_eq!(
            expand_root_with("~/dev/api", Some(home)),
            PathBuf::from("/home/lauta/dev/api")
        );
        assert_eq!(
            compact_root_with(Path::new("/home/lauta/dev/api"), Some(home)),
            "~/dev/api"
        );
        assert_eq!(
            compact_root_with(Path::new("/opt/bin"), Some(home)),
            "/opt/bin"
        );
        assert_eq!(expand_root_with("~", Some(home)), home.to_path_buf());
        assert_eq!(
            expand_root_with("~other/x", Some(home)),
            PathBuf::from("~other/x")
        );
    }

    #[test]
    fn catalog_overlays_name_and_startup_but_keeps_session_env() {
        let mut session = Session {
            workspaces: vec![SavedWorkspace {
                name: "folder".into(),
                root: PathBuf::from("/home/lauta/dev/api"),
                active_tab: 1,
                tabs: vec![],
                startup: vec![],
                env: vec![EnvVar {
                    key: "TOKEN".into(),
                    value: "secret".into(),
                }],
            }],
            ..Session::default()
        };
        let catalog = Catalog {
            workspaces: vec![WorkspaceDef {
                name: "API".into(),
                root: "~/dev/api".into(),
                startup: vec![StartupCmd {
                    program: "nvim".into(),
                    args: vec![],
                }],
                notes: String::new(),
            }],
        };
        merge_into_session_with(&mut session, &catalog, Some(Path::new("/home/lauta")));
        assert_eq!(session.workspaces.len(), 1);
        assert_eq!(session.workspaces[0].name, "API");
        assert_eq!(session.workspaces[0].startup[0].program, "nvim");
        assert_eq!(session.workspaces[0].env[0].value, "secret");
        assert_eq!(session.workspaces[0].active_tab, 1);
    }

    #[test]
    fn ensure_in_catalog_keeps_notes_and_name() {
        let mut list = vec![WorkspaceDef {
            name: "named".into(),
            root: "/tmp/ws".into(),
            startup: vec![],
            notes: "keep".into(),
        }];
        ensure_in_catalog(&mut list, "other", Path::new("/tmp/ws"));
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "named");
        assert_eq!(list[0].notes, "keep");
        ensure_in_catalog(&mut list, "fresh", Path::new("/tmp/other"));
        assert_eq!(list.len(), 2);
        assert_eq!(list[1].name, "fresh");
    }

    #[test]
    fn upsert_def_updates_in_place() {
        let mut list = vec![WorkspaceDef {
            name: "old".into(),
            root: "/tmp/ws".into(),
            startup: vec![],
            notes: "keep".into(),
        }];
        upsert_identity(
            &mut list,
            WorkspaceDef {
                name: "new".into(),
                root: "/tmp/ws".into(),
                startup: vec![],
                notes: String::new(),
            },
        );
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "new");
        assert_eq!(list[0].notes, "keep");
    }

    #[test]
    fn slug_and_remove_root() {
        assert_eq!(slug("LoLTerm"), "lolterm");
        assert_eq!(slug("API Server"), "api-server");
        assert_eq!(slug("***"), "ws");
        let mut list = vec![WorkspaceDef {
            name: "a".into(),
            root: "/tmp/a".into(),
            startup: vec![],
            notes: String::new(),
        }];
        remove_root(&mut list, Path::new("/tmp/a"));
        assert!(list.is_empty());
    }

    #[test]
    fn catalog_toml_skips_env_field() {
        let catalog = Catalog {
            workspaces: vec![WorkspaceDef {
                name: "lolterm".into(),
                root: "~/Projects/lolterm".into(),
                startup: vec![],
                notes: String::new(),
            }],
        };
        let text = toml::to_string(&catalog).expect("toml");
        assert!(!text.contains("env"));
        assert!(text.contains("lolterm"));
    }
}
