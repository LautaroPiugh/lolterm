//! API keys de agentes, machine-local. No van a config sincronizable ni al
//! contexto; solo se inyectan como env en panes de agente.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::session;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Secrets {
    #[serde(default)]
    pub keys: BTreeMap<String, String>,
}

pub fn path() -> PathBuf {
    crate::config::data_dir().join("secrets.json")
}

pub fn load() -> Secrets {
    let Ok(text) = std::fs::read_to_string(path()) else {
        return Secrets::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn persist(secrets: &Secrets) -> Result<(), String> {
    let target = path();
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("no pude crear dir: {err}"))?;
    }
    let text = serde_json::to_string_pretty(secrets).unwrap_or_else(|_| "{}".into());
    let tmp = target.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|err| format!("no pude escribir secrets.json: {err}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))
            .map_err(|err| format!("no pude ajustar permisos: {err}"))?;
    }
    std::fs::rename(&tmp, target).map_err(|err| format!("no pude publicar secrets.json: {err}"))
}

pub fn key_ok(key: &str) -> bool {
    session::env_key_ok(key.trim())
}

pub fn set(key: &str, value: &str) -> Result<(), String> {
    let key = key.trim().to_string();
    if !key_ok(&key) {
        return Err("nombre de variable inválido (letras, números y _)".into());
    }
    let mut secrets = load();
    if value.is_empty() {
        secrets.keys.remove(&key);
    } else {
        secrets.keys.insert(key, value.to_string());
    }
    persist(&secrets)
}

pub fn remove(key: &str) -> Result<(), String> {
    let mut secrets = load();
    secrets.keys.remove(key.trim());
    persist(&secrets)
}

/// Solo nombres, para el snapshot. Nunca valores.
pub fn names() -> Vec<String> {
    load().keys.keys().cloned().collect()
}

/// Pares completos, para inyectar en el env de un pane de agente.
pub fn all() -> Vec<(String, String)> {
    load().keys.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_is_outside_portable_config() {
        let target = path();
        assert!(
            !target.starts_with(crate::config::config_dir()),
            "secrets.json no debe ir en config sincronizable: {}",
            target.display()
        );
        assert!(target.ends_with("secrets.json"));
    }

    #[test]
    fn validates_key_names() {
        assert!(key_ok("OPENCODE_API_KEY"));
        assert!(key_ok(" _with_underscore "));
        assert!(!key_ok("1BAD"));
        assert!(!key_ok("HAS SPACE"));
        assert!(!key_ok(""));
        assert!(!key_ok("FOO-BAR"));
    }
}
