//! Canal local de solo lectura entre `lolterm` y el mux en vivo.
//!
//! Un Unix socket en el runtime dir (no en config sincronizable). El core
//! escucha; la CLI pregunta `context` / `panes` / `processes`. Sin writes.

use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use crate::config;
use crate::mux::Mux;

const ALLOWED: &[&str] = &["context", "panes", "processes"];

pub fn mux_socket_path() -> std::path::PathBuf {
    config::runtime_dir().join("mux.sock")
}

pub fn query(method: &str) -> Option<Value> {
    if !ALLOWED.contains(&method) {
        return None;
    }
    #[cfg(unix)]
    {
        unix::query(method)
    }
    #[cfg(not(unix))]
    {
        let _ = method;
        None
    }
}

pub fn serve(mux: Arc<Mutex<Mux>>) {
    #[cfg(unix)]
    unix::serve(mux);
    #[cfg(not(unix))]
    {
        let _ = mux;
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};

    pub fn query(method: &str) -> Option<Value> {
        let path = super::mux_socket_path();
        let mut stream = UnixStream::connect(&path).ok()?;
        let timeout = Duration::from_millis(400);
        stream.set_read_timeout(Some(timeout)).ok()?;
        stream.set_write_timeout(Some(timeout)).ok()?;
        let req = json!({ "id": 1, "method": method, "params": {} });
        writeln!(stream, "{req}").ok()?;
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).ok()?;
        let value: Value = serde_json::from_str(line.trim()).ok()?;
        if value.get("error").is_some() {
            return None;
        }
        value.get("result").cloned()
    }

    pub fn serve(mux: Arc<Mutex<Mux>>) {
        let path = super::mux_socket_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if path.exists() {
            if UnixStream::connect(&path).is_ok() {
                return;
            }
            let _ = std::fs::remove_file(&path);
        }
        let Ok(listener) = UnixListener::bind(&path) else {
            return;
        };
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        std::thread::spawn(move || {
            for incoming in listener.incoming() {
                let Ok(stream) = incoming else {
                    continue;
                };
                handle_client(&mux, stream);
            }
            let _ = std::fs::remove_file(&path);
        });
    }

    fn handle_client(mux: &Arc<Mutex<Mux>>, stream: UnixStream) {
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        if stream.read_line(&mut line).is_err() || line.trim().is_empty() {
            return;
        }
        let Ok(req) = serde_json::from_str::<Value>(line.trim()) else {
            let _ = writeln!(stream.get_mut(), "{}", json!({ "id": 0, "error": "json" }));
            return;
        };
        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
        let reply = match method {
            "context" => mux
                .lock()
                .ok()
                .and_then(|mux| serde_json::to_value(mux.context()).ok()),
            "panes" => mux
                .lock()
                .ok()
                .and_then(|mux| serde_json::to_value(mux.pane_rows()).ok()),
            "processes" => mux
                .lock()
                .ok()
                .and_then(|mux| serde_json::to_value(mux.process_names()).ok()),
            _ => None,
        };
        let body = match reply {
            Some(result) => json!({ "id": id, "result": result }),
            None => json!({ "id": id, "error": "método no permitido" }),
        };
        let _ = writeln!(stream.get_mut(), "{body}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_lives_outside_portable_config() {
        let path = mux_socket_path();
        let config = crate::config::config_dir();
        assert!(
            !path.starts_with(&config),
            "el socket no debe ir en config sincronizable: {}",
            path.display()
        );
        assert!(path.ends_with("mux.sock"));
    }

    #[test]
    fn query_rejects_unknown_methods() {
        assert!(query("write").is_none());
        assert!(query("run").is_none());
    }
}
