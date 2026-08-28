//! HTTP opt-in: LAN/VPN, password en data_dir (no portable), sin TLS.
//! Vista web del workspace (git/archivos/REST/prompt). Los PTYs vivos siguen
//! en Desktop; la web puede mandar bytes al pane enfocado.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::config;
use crate::mux::Mux;

const PAGE: &str = r#"<!DOCTYPE html>
<html lang="es"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>LoLTerm</title>
<style>
body{font:14px/1.4 system-ui;margin:0;background:#111;color:#ddd}
header{padding:12px 16px;border-bottom:1px solid #333;display:flex;gap:8px;align-items:center}
main{padding:16px;display:grid;gap:12px}
pre,textarea{width:100%;background:#1a1a1a;color:#ddd;border:1px solid #333;padding:8px}
button{background:#2a4;color:#111;border:0;padding:6px 10px}
.muted{color:#888;font-size:12px}
</style>
<header><strong>LoLTerm</strong> <span class="muted" id="meta"></span></header>
<main>
<p class="muted">LAN opt-in. Password local. Sin TLS: usá VPN o SSH tunnel.</p>
<label>token <input id="tok" type="password"></label>
<button id="load">cargar</button>
<pre id="snap"></pre>
<textarea id="prompt" rows="3" placeholder="escribir al pane enfocado"></textarea>
<button id="send">enviar</button>
</main>
<script>
const tok=()=>document.getElementById('tok').value;
async function api(method,params){
  const r=await fetch('/api',{method:'POST',headers:{'Content-Type':'application/json','Authorization':'Bearer '+tok()},body:JSON.stringify({method,params:params||{}})});
  if(!r.ok) throw new Error(await r.text());
  return r.json();
}
document.getElementById('load').onclick=async()=>{
  const s=await api('snapshot');
  document.getElementById('meta').textContent=(s.name||'')+' · '+(s.branch||'');
  document.getElementById('snap').textContent=JSON.stringify({root:s.root,git:s.git,agents:s.agents,tabs:s.tabs?.map(t=>t.name)},null,2);
};
document.getElementById('send').onclick=async()=>{
  const s=await api('snapshot');
  const pane=s.tabs?.[s.active_tab]?.focused;
  const text=document.getElementById('prompt').value;
  await api('write',{pane,b64:btoa(text)});
};
</script></html>"#;

pub fn serve(mux: Arc<Mutex<Mux>>) {
    let cfg = load_config();
    if !cfg.enabled {
        return;
    }
    let bind = cfg.bind();
    let Ok(listener) = TcpListener::bind(&bind) else {
        return;
    };
    let _ = listener.set_nonblocking(false);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mux = Arc::clone(&mux);
            thread::spawn(move || {
                let _ = handle(stream, &mux);
            });
        }
    });
}

#[derive(Clone)]
pub struct HttpCfg {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

impl HttpCfg {
    pub fn bind(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub fn load_config() -> HttpCfg {
    let path = config::config_dir().join("config.toml");
    let text = std::fs::read_to_string(path).unwrap_or_default();
    parse_config(&text)
}

fn parse_config(text: &str) -> HttpCfg {
    let enabled = extract(text, "enabled")
        .map(|value| value == "true")
        .unwrap_or(false);
    let host = extract(text, "host").unwrap_or_else(|| "127.0.0.1".into());
    let port = extract(text, "port")
        .and_then(|value| value.parse().ok())
        .unwrap_or(47832);
    HttpCfg {
        enabled,
        host,
        port,
    }
}

fn extract(text: &str, key: &str) -> Option<String> {
    let mut in_http = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_http = line == "[http]";
            continue;
        }
        if in_http && let Some(rest) = line.strip_prefix(&format!("{key} =")) {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

pub fn password_path() -> std::path::PathBuf {
    config::data_dir().join("http.password")
}

pub fn set_password(raw: &str) -> Result<(), String> {
    let raw = raw.trim();
    if raw.len() < 8 {
        return Err("password HTTP: mínimo 8 caracteres".into());
    }
    let path = password_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    std::fs::write(&path, raw).map_err(|err| err.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn password_ok(got: &str) -> bool {
    let Ok(want) = std::fs::read_to_string(password_path()) else {
        return false;
    };
    let want = want.trim();
    !want.is_empty() && want == got.trim()
}

fn handle(mut stream: TcpStream, mux: &Arc<Mutex<Mux>>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(8)))?;
    let mut buf = [0u8; 65536];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let mut lines = req.split("\r\n");
    let start = lines.next().unwrap_or("");
    let mut auth = String::new();
    let mut content_len = 0usize;
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix("Authorization: Bearer ") {
            auth = rest.trim().to_string();
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_len = rest.trim().parse().unwrap_or(0);
        }
    }
    let body_start = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(req.len());
    let body = req.get(body_start..).unwrap_or("").as_bytes();
    let body = if content_len > 0 {
        String::from_utf8_lossy(&body[..content_len.min(body.len())]).into_owned()
    } else {
        String::new()
    };

    if start.starts_with("GET / ") || start.starts_with("GET /index") {
        return write_http(
            &mut stream,
            200,
            "text/html; charset=utf-8",
            PAGE.as_bytes(),
        );
    }
    if !start.starts_with("POST /api") {
        return write_http(&mut stream, 404, "text/plain", b"not found");
    }
    if !password_ok(&auth) {
        return write_http(&mut stream, 401, "text/plain", b"unauthorized");
    }
    let parsed: Value = serde_json::from_str(&body).unwrap_or(json!({}));
    let method = parsed["method"].as_str().unwrap_or("");
    let params = parsed.get("params").cloned().unwrap_or(json!({}));
    let result = dispatch(mux, method, &params);
    let payload = match result {
        Ok(value) => value.to_string(),
        Err(err) => json!({"error": err}).to_string(),
    };
    write_http(&mut stream, 200, "application/json", payload.as_bytes())
}

fn dispatch(mux: &Arc<Mutex<Mux>>, method: &str, params: &Value) -> Result<Value, String> {
    let mut mux = mux.lock().map_err(|err| err.to_string())?;
    match method {
        "snapshot" => serde_json::to_value(mux.snapshot()).map_err(|err| err.to_string()),
        "hud" => serde_json::to_value(mux.hud()).map_err(|err| err.to_string()),
        "write" => {
            let pane = params["pane"].as_u64().unwrap_or(0);
            let bytes = decode_b64(params["b64"].as_str().unwrap_or(""));
            mux.write(pane, &bytes).map_err(|err| err.to_string())?;
            Ok(json!({"ok": true}))
        }
        "readFile" => mux
            .read_file(params["rel"].as_str().unwrap_or(""))
            .map(|text| json!({"text": text})),
        "gitOp" => {
            mux.git_op(
                params["op"].as_str().unwrap_or(""),
                params["path"].as_str(),
                params["message"].as_str(),
            )?;
            serde_json::to_value(mux.snapshot()).map_err(|err| err.to_string())
        }
        other => Err(format!("método HTTP no permitido: {other}")),
    }
}

fn write_http(
    stream: &mut TcpStream,
    status: u16,
    ctype: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn decode_b64(text: &str) -> Vec<u8> {
    let mut vals = Vec::new();
    for ch in text.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => continue,
            _ => continue,
        };
        vals.push(v);
    }
    let mut out = Vec::new();
    for chunk in vals.chunks(4) {
        let a = u32::from(*chunk.first().unwrap_or(&0));
        let b = u32::from(*chunk.get(1).unwrap_or(&0));
        let c = u32::from(*chunk.get(2).unwrap_or(&0));
        let d = u32::from(*chunk.get(3).unwrap_or(&0));
        let n = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::parse_config;

    #[test]
    fn http_enabled_is_scoped_to_http_section() {
        let cfg = parse_config(
            r#"
[http]
enabled = false
host = "0.0.0.0"
port = 48123

[other]
enabled = true
"#,
        );

        assert!(!cfg.enabled);
        assert_eq!(cfg.bind(), "0.0.0.0:48123");
    }

    #[test]
    fn http_defaults_when_section_is_missing() {
        let cfg = parse_config("[other]\nenabled = true\n");

        assert!(!cfg.enabled);
        assert_eq!(cfg.bind(), "127.0.0.1:47832");
    }
}
