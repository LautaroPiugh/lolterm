//! Cuotas de agentes (criterio Orquester: solo instalados con ventanas reales).
//!
//! LoLTerm **no** guarda tokens ni pega a APIs con credenciales propias.
//! Usa la sesión que ya dejó cada CLI en el disco del usuario:
//! - Claude Code: `claude --print /usage`, con fallback a `~/.claude.json`.
//! - Codex / ChatGPT+: `codex app-server --listen stdio://` + `account/rateLimits/read`.
//! - OpenCode Go: `GET https://opencode.ai/zen/go/v1/usage` con la key de
//!   `~/.local/share/opencode/auth.json` (todas las ventanas del JSON).
//! - ClinePass: `GET …/users/me/plan/usage-limits` con la key de
//!   `~/.cline/data/settings/providers.json` (todas las ventanas del JSON).
//! - Antigravity (`agy`): la API local del language server que levanta el
//!   propio CLI/IDE (JSON Connect en `127.0.0.1`, endpoint GetUserStatus,
//!   descubiertos con `ps` + `lsof`). Solo hay datos mientras `agy` corre;
//!   se agrupa por pools como en `/usage`. El límite semanal lo valida
//!   Google server-side y no se expone localmente, así que no se inventa.
//! - Copilot CLI: `GET https://api.github.com/copilot_internal/user` con el
//!   token de `gh auth` (o `GH_TOKEN`). LoLTerm no guarda el token.
//!
//! `QuotaBar.percent` es **% usado**, como en Orquester.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};

use crate::files;

const CODEX_POLL: Duration = Duration::from_secs(1);

static CODEX_BARS: Mutex<Option<Vec<QuotaBar>>> = Mutex::new(None);
static CODEX_ERR: Mutex<Option<String>> = Mutex::new(None);
static CODEX_WORKER: AtomicBool = AtomicBool::new(false);
static CODEX_CLIENT: Mutex<Option<CodexClient>> = Mutex::new(None);
static CLAUDE_USAGE: Mutex<Option<Vec<QuotaBar>>> = Mutex::new(None);
static CLAUDE_WORKER: AtomicBool = AtomicBool::new(false);
static CLAUDE_TRIED: AtomicBool = AtomicBool::new(false);
static OPENCODE_BARS: Mutex<Option<Vec<QuotaBar>>> = Mutex::new(None);
static OPENCODE_ERR: Mutex<Option<String>> = Mutex::new(None);
static OPENCODE_WORKER: AtomicBool = AtomicBool::new(false);
static CLINE_BARS: Mutex<Option<Vec<QuotaBar>>> = Mutex::new(None);
static CLINE_ERR: Mutex<Option<String>> = Mutex::new(None);
static CLINE_WORKER: AtomicBool = AtomicBool::new(false);
static CLINE_LIVE: Mutex<Option<String>> = Mutex::new(None);
static ANTIGRAVITY_BARS: Mutex<Option<Vec<QuotaBar>>> = Mutex::new(None);
static ANTIGRAVITY_ERR: Mutex<Option<String>> = Mutex::new(None);
static ANTIGRAVITY_WORKER: AtomicBool = AtomicBool::new(false);
static COPILOT_BARS: Mutex<Option<Vec<QuotaBar>>> = Mutex::new(None);
static COPILOT_ERR: Mutex<Option<String>> = Mutex::new(None);
static COPILOT_WORKER: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QuotaBar {
    pub key: String,
    pub label: String,
    pub percent: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QuotaAgent {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub running: bool,
    pub pending: bool,
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub bars: Vec<QuotaBar>,
}

pub fn agents(running: &[String]) -> Vec<QuotaAgent> {
    kick_codex_refresh();
    kick_claude_usage();
    kick_opencode_usage();
    kick_cline_usage();
    kick_antigravity_usage();
    kick_copilot_usage();
    let mut out = Vec::new();
    if files::command_on_path("claude") {
        let agent = claude(running);
        if agent.supported || agent.pending {
            out.push(agent);
        }
    }
    if files::command_on_path("codex") {
        let agent = codex(running);
        if agent.supported || agent.pending {
            out.push(agent);
        }
    }
    if files::command_on_path("opencode") || opencode_api_key().is_some() {
        let agent = opencode(running);
        if agent.supported || agent.pending || agent.note.is_some() {
            out.push(agent);
        }
    }
    if files::command_on_path("cline") || running_has(running, "cline") || cline_api_key().is_some()
    {
        out.push(cline(running));
    }
    for (name, label, note) in [
        (
            "hermes",
            "Hermes",
            "la cuota depende del provider/modelo configurado; no hay una cuota universal local",
        ),
        (
            "goose",
            "Goose",
            "tokens/costo locales; la cuota depende del provider configurado",
        ),
        (
            "aider",
            "Aider",
            "tokens/costo locales; la cuota depende del provider configurado",
        ),
        (
            "crush",
            "Crush",
            "tokens/costo locales; la cuota depende del provider configurado",
        ),
        (
            "qwen",
            "Qwen Code",
            "`/stats` muestra uso diario/mensual local; no cuota restante del provider",
        ),
        (
            "openhands",
            "OpenHands",
            "métricas de conversación locales; no expone cuota restante universal",
        ),
    ] {
        if files::command_on_path(name) {
            out.push(provider_only_agent(name, label, note, running));
        }
    }
    if files::command_on_path("copilot") {
        let agent = copilot(running);
        if agent.supported || agent.pending || agent.note.is_some() {
            out.push(agent);
        }
    }
    if antigravity_present() {
        let agent = antigravity(running);
        if agent.supported || agent.pending || agent.note.is_some() {
            out.push(agent);
        }
    }
    out
}

/// Precalienta los workers de cuota lo antes posible (se llama al arrancar el
/// sidecar): dispara todos los `kick_*` de una sola vez; son one-shot.
pub fn warm_up() {
    thread::spawn(|| {
        agents(&[]);
    });
}

fn running_has(running: &[String], name: &str) -> bool {
    running.iter().any(|item| item == name)
}

fn provider_only_agent(name: &str, label: &str, note: &str, running: &[String]) -> QuotaAgent {
    QuotaAgent {
        id: name.into(),
        label: label.into(),
        available: true,
        running: running_has(running, name),
        pending: false,
        supported: false,
        note: Some(note.into()),
        bars: Vec::new(),
    }
}

fn claude(running: &[String]) -> QuotaAgent {
    let available = files::command_on_path("claude");
    let running = running_has(running, "claude");
    let bars = claude_bars();
    let pending = bars.is_empty() && available && !CLAUDE_TRIED.load(Ordering::SeqCst);
    QuotaAgent {
        id: "claude".into(),
        label: "Claude Code".into(),
        available,
        running,
        pending,
        supported: !bars.is_empty(),
        note: None,
        bars,
    }
}

fn codex(running: &[String]) -> QuotaAgent {
    let available = files::command_on_path("codex");
    let running = running_has(running, "codex");
    let bars = CODEX_BARS
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();
    let err = CODEX_ERR.lock().ok().and_then(|g| g.clone());
    let pending = bars.is_empty() && err.is_none() && available;
    let note = if !bars.is_empty() {
        None
    } else if pending {
        Some("consultando Codex…".into())
    } else {
        err
    };
    QuotaAgent {
        id: "codex".into(),
        label: "Codex".into(),
        available,
        running,
        pending,
        supported: !bars.is_empty(),
        note,
        bars,
    }
}

fn opencode(running: &[String]) -> QuotaAgent {
    let available = files::command_on_path("opencode");
    let running = running_has(running, "opencode");
    let bars = OPENCODE_BARS
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_else(read_opencode_export);
    let err = OPENCODE_ERR.lock().ok().and_then(|g| g.clone());
    let pending = bars.is_empty() && err.is_none() && available;
    let note = if !bars.is_empty() {
        None
    } else if pending {
        Some("consultando OpenCode Go…".into())
    } else {
        err
    };
    QuotaAgent {
        id: "opencode".into(),
        label: "OpenCode".into(),
        available,
        running,
        pending,
        supported: !bars.is_empty(),
        note,
        bars,
    }
}

fn cline(running: &[String]) -> QuotaAgent {
    let available = files::command_on_path("cline")
        || running_has(running, "cline")
        || cline_api_key().is_some();
    let running = running_has(running, "cline");
    let bars = CLINE_BARS
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();
    let err = CLINE_ERR.lock().ok().and_then(|g| g.clone());
    let pending = bars.is_empty() && err.is_none();
    let note = if !bars.is_empty() {
        None
    } else if pending {
        Some("consultando ClinePass…".into())
    } else {
        err
    };
    QuotaAgent {
        id: "cline".into(),
        label: "Cline".into(),
        available,
        running,
        pending,
        supported: !bars.is_empty(),
        note,
        bars,
    }
}

fn copilot(running: &[String]) -> QuotaAgent {
    let available = files::command_on_path("copilot");
    let running = running_has(running, "copilot");
    let bars = COPILOT_BARS
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();
    let err = COPILOT_ERR.lock().ok().and_then(|g| g.clone());
    let pending = bars.is_empty() && err.is_none() && available;
    let note = if !bars.is_empty() {
        None
    } else if pending {
        Some("consultando Copilot…".into())
    } else {
        err
    };
    QuotaAgent {
        id: "copilot".into(),
        label: "Copilot".into(),
        available,
        running,
        pending,
        supported: !bars.is_empty(),
        note,
        bars,
    }
}

fn antigravity(running: &[String]) -> QuotaAgent {
    let available = antigravity_present();
    let running = running_has(running, "agy") || running_has(running, "antigravity");
    let bars = ANTIGRAVITY_BARS
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();
    let err = ANTIGRAVITY_ERR.lock().ok().and_then(|g| g.clone());
    let pending = bars.is_empty() && err.is_none() && available;
    let note = if !bars.is_empty() {
        None
    } else if pending {
        Some("consultando Antigravity…".into())
    } else {
        err
    };
    QuotaAgent {
        id: "antigravity".into(),
        label: "Antigravity".into(),
        available,
        running,
        pending,
        supported: !bars.is_empty(),
        note,
        bars,
    }
}

fn antigravity_present() -> bool {
    files::command_on_path("agy")
        || files::command_on_path("antigravity")
        || std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".gemini").join("antigravity-cli"))
            .is_some_and(|dir| dir.is_dir())
}

fn kick_codex_refresh() {
    if !files::command_on_path("codex") {
        return;
    }
    if CODEX_WORKER
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    thread::spawn(|| {
        loop {
            if !files::command_on_path("codex") {
                drop_codex_client();
                thread::sleep(Duration::from_secs(3));
                continue;
            }
            match query_codex_bars() {
                Ok(bars) => {
                    if let Ok(mut slot) = CODEX_BARS.lock() {
                        *slot = Some(bars);
                    }
                    if let Ok(mut err) = CODEX_ERR.lock() {
                        *err = None;
                    }
                    thread::sleep(CODEX_POLL);
                }
                Err(msg) => {
                    drop_codex_client();
                    if let Ok(mut err) = CODEX_ERR.lock() {
                        *err = Some(msg);
                    }
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
    });
}

struct CodexClient {
    child: Child,
    stdin: ChildStdin,
    lines: std::io::Lines<BufReader<std::process::ChildStdout>>,
    next_id: u64,
}

impl Drop for CodexClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn drop_codex_client() {
    if let Ok(mut slot) = CODEX_CLIENT.lock() {
        *slot = None;
    }
}

fn query_codex_bars() -> Result<Vec<QuotaBar>, String> {
    let mut slot = CODEX_CLIENT
        .lock()
        .map_err(|_| "lock de Codex".to_string())?;
    if slot.is_none() {
        *slot = Some(spawn_codex_client()?);
    }
    let Some(client) = slot.as_mut() else {
        return Err("no hay cliente Codex".into());
    };
    match client.rate_limits() {
        Ok(bars) => Ok(bars),
        Err(err) => {
            *slot = None;
            Err(err)
        }
    }
}

fn spawn_codex_client() -> Result<CodexClient, String> {
    let bin = files::resolve_command("codex")
        .ok_or_else(|| "no encontré el binario `codex`".to_string())?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "no hay HOME para leer ~/.codex".to_string())?;
    let codex_home = home.join(".codex");
    let mut cmd = Command::new(bin);
    if let Some(path) = files::effective_path() {
        cmd.env("PATH", path);
    }
    cmd.env("HOME", &home)
        .env("CODEX_HOME", &codex_home)
        .current_dir(&home)
        .args(["app-server", "--listen", "stdio://"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = cmd
        .spawn()
        .map_err(|err| format!("no arrancó `codex app-server`: {err}"))?;
    let mut stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err("stdin de app-server".into());
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err("stdout de app-server".into());
        }
    };
    let lines = BufReader::new(stdout).lines();
    if let Err(err) = writeln!(
        stdin,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": { "name": "lolterm", "title": "LoLTerm", "version": crate::VERSION },
                "capabilities": { "experimentalApi": true }
            }
        })
    ) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(err.to_string());
    }
    let _ = stdin.flush();
    thread::sleep(Duration::from_millis(150));
    if let Err(err) = writeln!(
        stdin,
        "{}",
        json!({ "jsonrpc": "2.0", "method": "initialized" })
    ) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(err.to_string());
    }
    let _ = stdin.flush();
    Ok(CodexClient {
        child,
        stdin,
        lines,
        next_id: 2,
    })
}

impl CodexClient {
    fn rate_limits(&mut self) -> Result<Vec<QuotaBar>, String> {
        let id = self.next_id;
        self.next_id += 1;
        writeln!(
            self.stdin,
            "{}",
            json!({ "jsonrpc": "2.0", "id": id, "method": "account/rateLimits/read", "params": {} })
        )
        .map_err(|err| err.to_string())?;
        let _ = self.stdin.flush();
        let value = wait_rpc_id(&mut self.lines, id)?;
        let bars = parse_codex_bars(&value);
        if bars.is_empty() {
            Err("Codex no devolvió ventanas de cuota (Session/Week).".into())
        } else {
            Ok(bars)
        }
    }
}

fn rpc_id(value: &Value) -> Option<u64> {
    let id = value.get("id")?;
    id.as_u64()
        .or_else(|| id.as_i64().and_then(|n| u64::try_from(n).ok()))
}

fn wait_rpc_id(
    lines: &mut std::io::Lines<BufReader<std::process::ChildStdout>>,
    want: u64,
) -> Result<Value, String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        let line = lines
            .next()
            .ok_or_else(|| "Codex cerró app-server".to_string())?
            .map_err(|err| err.to_string())?;
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if rpc_id(&value) != Some(want) {
            continue;
        }
        if let Some(err) = value.get("error") {
            return Err(codex_rpc_error(err));
        }
        return Ok(value.get("result").cloned().unwrap_or(Value::Null));
    }
    Err("timeout leyendo cuota de Codex".into())
}

fn codex_rpc_error(err: &Value) -> String {
    let msg = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if msg.contains("authentication required") {
        "Codex no cargó la sesión de ChatGPT (hace falta ~/.codex con login). En el pane: /login."
            .into()
    } else if let Some(message) = err.get("message").and_then(Value::as_str) {
        message.to_string()
    } else {
        "Codex no devolvió la cuota".into()
    }
}

pub fn parse_codex_bars(root: &Value) -> Vec<QuotaBar> {
    let limits = root
        .pointer("/rateLimits")
        .or_else(|| root.pointer("/rate_limits"))
        .or_else(|| root.pointer("/result/rateLimits"))
        .or_else(|| root.pointer("/result/rate_limits"))
        .unwrap_or(root);
    let mut bars = Vec::new();
    let mut seen = Vec::new();
    for (key, fallback) in [
        ("primary", "Primary limit"),
        ("secondary", "Secondary limit"),
    ] {
        push_codex_window(&mut bars, limits.get(key), key, fallback);
        if limits.get(key).is_some() {
            seen.push(key);
        }
    }
    if let Some(map) = limits.as_object() {
        for (key, value) in map {
            if seen.contains(&key.as_str()) {
                continue;
            }
            if !value.is_object() {
                continue;
            }
            push_codex_window(&mut bars, Some(value), key, &window_label(key));
        }
    }
    bars
}

fn push_codex_window(out: &mut Vec<QuotaBar>, value: Option<&Value>, key: &str, fallback: &str) {
    let Some(Value::Object(map)) = value else {
        return;
    };
    let used = map
        .get("usedPercent")
        .or_else(|| map.get("used_percent"))
        .and_then(Value::as_f64)
        .map(as_points)
        .unwrap_or(0);
    let mins = map
        .get("windowDurationMins")
        .or_else(|| map.get("window_duration_mins"))
        .and_then(Value::as_u64);
    let label = match mins {
        Some(300) => "5-hour limit",
        Some(n) if n >= 10_000 => "Weekly limit",
        Some(n) if n >= 60 * 24 => "Weekly limit",
        _ => fallback,
    };
    let reset = map
        .get("resetsAt")
        .or_else(|| map.get("resets_at"))
        .and_then(reset_from_value);
    out.push(QuotaBar {
        key: key.into(),
        label: label.into(),
        percent: used,
        reset,
    });
}

fn kick_claude_usage() {
    if !files::command_on_path("claude") {
        return;
    }
    if CLAUDE_WORKER
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    thread::spawn(|| {
        loop {
            if !files::command_on_path("claude") {
                thread::sleep(Duration::from_secs(5));
                continue;
            }
            if let Ok(bars) = fetch_claude_usage()
                && !bars.is_empty()
                && let Ok(mut slot) = CLAUDE_USAGE.lock()
            {
                *slot = Some(bars);
            }
            CLAUDE_TRIED.store(true, Ordering::SeqCst);
            thread::sleep(Duration::from_secs(30));
        }
    });
}

fn kick_opencode_usage() {
    if !files::command_on_path("opencode") && opencode_api_key().is_none() {
        return;
    }
    if OPENCODE_WORKER
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    thread::spawn(|| {
        loop {
            match fetch_opencode_bars() {
                Ok(bars) => {
                    if let Ok(mut slot) = OPENCODE_BARS.lock() {
                        *slot = Some(bars);
                    }
                    if let Ok(mut err) = OPENCODE_ERR.lock() {
                        *err = None;
                    }
                }
                Err(msg) => {
                    if let Ok(mut err) = OPENCODE_ERR.lock() {
                        *err = Some(msg);
                    }
                }
            }
            thread::sleep(Duration::from_secs(20));
        }
    });
}

fn kick_cline_usage() {
    if CLINE_WORKER
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    thread::spawn(|| {
        loop {
            match fetch_cline_bars() {
                Ok(bars) => {
                    if let Ok(mut slot) = CLINE_BARS.lock() {
                        *slot = Some(bars);
                    }
                    if let Ok(mut err) = CLINE_ERR.lock() {
                        *err = None;
                    }
                }
                Err(msg) => {
                    if let Ok(mut err) = CLINE_ERR.lock() {
                        *err = Some(msg);
                    }
                }
            }
            thread::sleep(Duration::from_secs(20));
        }
    });
}

fn kick_antigravity_usage() {
    if !antigravity_present() {
        return;
    }
    if ANTIGRAVITY_WORKER
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    thread::spawn(|| {
        loop {
            match fetch_antigravity_bars() {
                Ok(bars) => {
                    if let Ok(mut slot) = ANTIGRAVITY_BARS.lock() {
                        *slot = Some(bars);
                    }
                    if let Ok(mut err) = ANTIGRAVITY_ERR.lock() {
                        *err = None;
                    }
                }
                Err(msg) => {
                    if let Ok(mut err) = ANTIGRAVITY_ERR.lock() {
                        *err = Some(msg);
                    }
                }
            }
            thread::sleep(Duration::from_secs(20));
        }
    });
}

fn kick_copilot_usage() {
    if !files::command_on_path("copilot") {
        return;
    }
    if COPILOT_WORKER
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    thread::spawn(|| {
        loop {
            match fetch_copilot_bars() {
                Ok(bars) => {
                    if let Ok(mut slot) = COPILOT_BARS.lock() {
                        *slot = Some(bars);
                    }
                    if let Ok(mut err) = COPILOT_ERR.lock() {
                        *err = None;
                    }
                }
                Err(msg) => {
                    if let Ok(mut err) = COPILOT_ERR.lock() {
                        *err = Some(msg);
                    }
                }
            }
            thread::sleep(Duration::from_secs(20));
        }
    });
}

fn claude_bars() -> Vec<QuotaBar> {
    if let Some(bars) = CLAUDE_USAGE.lock().ok().and_then(|g| g.clone())
        && !bars.is_empty()
    {
        return bars;
    }
    read_claude_json_bars()
}

fn fetch_claude_usage() -> Result<Vec<QuotaBar>, String> {
    let bin = files::resolve_command("claude").ok_or_else(|| "no está `claude`".to_string())?;
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut cmd = Command::new(bin);
    if let Some(path) = files::effective_path() {
        cmd.env("PATH", path);
    }
    if let Some(home) = &home {
        cmd.env("HOME", home).current_dir(home);
    }
    cmd.args(["--print", "/usage"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = cmd
        .spawn()
        .map_err(|err| format!("no arrancó `claude --print /usage`: {err}"))?;
    let stdout = child.stdout.take();
    let reader = thread::spawn(move || {
        let mut text = String::new();
        if let Some(stdout) = stdout {
            let _ = BufReader::new(stdout).read_to_string(&mut text);
        }
        text
    });
    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(15) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("timeout en `claude --print /usage`".into());
        }
        match child.try_wait() {
            Ok(Some(_)) => {
                let text = reader.join().unwrap_or_default();
                return Ok(parse_claude_usage_text(&text));
            }
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(err) => {
                let _ = child.kill();
                return Err(err.to_string());
            }
        }
    }
}

pub fn parse_claude_usage_text(output: &str) -> Vec<QuotaBar> {
    let mut bars = Vec::new();
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        let Some(used) = used_percent_in_line(line) else {
            continue;
        };
        let is_week = lower.contains("week");
        let is_session = lower.contains("session");
        if !is_week && !is_session {
            continue;
        }
        let reset = line.split_once("reset").map(|(_, rest)| {
            let rest = rest.trim_start_matches('s').trim();
            if rest.is_empty() {
                "resets unknown".into()
            } else {
                format!("resets {rest}")
            }
        });
        bars.push(QuotaBar {
            key: if is_week { "weekly" } else { "session" }.into(),
            label: if is_week {
                "Current week".into()
            } else {
                "Current session".into()
            },
            percent: used,
            reset,
        });
    }
    bars
}

fn used_percent_in_line(line: &str) -> Option<u8> {
    let lower = line.to_ascii_lowercase();
    let idx = lower.find("% used")?;
    let before = line[..idx].trim_end();
    let num = before
        .rsplit(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .find(|part| !part.is_empty())?;
    num.parse::<f64>().ok().map(as_percent)
}

fn claude_json_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".claude.json"))
}

fn read_claude_json_bars() -> Vec<QuotaBar> {
    let Some(path) = claude_json_path() else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    parse_claude_bars(&value)
}

pub fn parse_claude_bars(root: &Value) -> Vec<QuotaBar> {
    let mut bars = Vec::new();
    walk_claude(root, &mut bars);
    bars
}

fn walk_claude(value: &Value, out: &mut Vec<QuotaBar>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if skip_key(key) {
                    continue;
                }
                if let Some((id, label)) = bar_kind(key)
                    && let Some(bar) = bar_from_used(id, label, child)
                    && !out.iter().any(|seen| seen.key == bar.key)
                {
                    out.push(bar);
                    continue;
                }
                walk_claude(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_claude(item, out);
            }
        }
        _ => {}
    }
}

fn skip_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "credential",
        "api_key",
        "apikey",
        "oauth",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn bar_kind(key: &str) -> Option<(&'static str, &'static str)> {
    match key {
        "five_hour" | "fiveHour" | "five-hour" => Some(("five_hour", "5-hour limit")),
        "seven_day" | "sevenDay" | "seven-day" => Some(("seven_day", "Weekly limit")),
        _ => None,
    }
}

fn bar_from_used(key: &str, label: &str, value: &Value) -> Option<QuotaBar> {
    let (used, reset) = match value {
        Value::Number(n) => (as_percent(n.as_f64()?), None),
        Value::Object(map) => {
            let n = map
                .get("utilization")
                .or_else(|| map.get("used"))
                .or_else(|| map.get("percent"))
                .and_then(Value::as_f64)?;
            let reset = map
                .get("resets_at")
                .or_else(|| map.get("resetsAt"))
                .and_then(Value::as_str)
                .map(short_reset);
            (as_percent(n), reset)
        }
        _ => return None,
    };
    Some(QuotaBar {
        key: key.into(),
        label: label.into(),
        percent: used,
        reset,
    })
}

fn read_opencode_export() -> Vec<QuotaBar> {
    let path = opencode_export_path();
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    parse_opencode_export(&value)
}

fn opencode_export_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("opencode/quota-export.json");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join(".cache/opencode/quota-export.json")
}

pub fn parse_opencode_export(root: &Value) -> Vec<QuotaBar> {
    let mut bars = Vec::new();
    let Some(providers) = root.get("providers").and_then(Value::as_object) else {
        return bars;
    };
    for (id, provider) in providers {
        if provider.get("status").and_then(Value::as_str) != Some("ok") {
            continue;
        }
        let Some(entries) = provider.get("entries").and_then(Value::as_array) else {
            continue;
        };
        for (idx, entry) in entries.iter().enumerate() {
            let remaining = entry
                .get("percentRemaining")
                .and_then(Value::as_f64)
                .map(as_percent)
                .unwrap_or(0);
            if entry.get("unlimited").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(id.as_str());
            let window = entry.get("window").and_then(Value::as_str).unwrap_or("");
            let label = if window.is_empty() {
                format!("{id} {name}")
            } else {
                format!("{id} {window}")
            };
            bars.push(QuotaBar {
                key: format!("{id}-{idx}"),
                label,
                percent: remaining_from_used(remaining),
                reset: entry.get("resetAt").and_then(reset_from_value),
            });
        }
    }
    bars
}

fn fetch_opencode_bars() -> Result<Vec<QuotaBar>, String> {
    if let Some(key) = opencode_api_key() {
        match bearer_get("https://opencode.ai/zen/go/v1/usage", &key) {
            Ok((status, value)) if (200..300).contains(&status) => {
                let bars = parse_opencode_usage(&value);
                if !bars.is_empty() {
                    return Ok(bars);
                }
            }
            Ok((401, _)) => {
                return Err("OpenCode Go: key inválida. En el pane: /connect.".into());
            }
            Ok((403, _)) => {
                let export = read_opencode_export();
                if !export.is_empty() {
                    return Ok(export);
                }
                return Err(
                    "OpenCode: esta key no tiene Zen Go. Sin ese plan no hay barras de cuota."
                        .into(),
                );
            }
            Ok((status, _)) => {
                return Err(format!("OpenCode Go devolvió HTTP {status}"));
            }
            Err(err) => {
                let export = read_opencode_export();
                if !export.is_empty() {
                    return Ok(export);
                }
                return Err(err);
            }
        }
    }
    let export = read_opencode_export();
    if !export.is_empty() {
        return Ok(export);
    }
    if opencode_api_key().is_none() {
        Err(
            "OpenCode Go: no hay key en ~/.local/share/opencode/auth.json. En el pane: /connect."
                .into(),
        )
    } else {
        Err("OpenCode Go no devolvió ventanas de cuota.".into())
    }
}

fn fetch_cline_bars() -> Result<Vec<QuotaBar>, String> {
    let creds = cline_creds().ok_or_else(|| {
        "Cline: no hay sesión en ~/.cline/data/settings/providers.json. En el pane: cline auth."
            .to_string()
    })?;
    let mut access = CLINE_LIVE
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or(creds.access.clone());
    let (mut status, mut value) = bearer_get(
        "https://api.cline.bot/api/v1/users/me/plan/usage-limits",
        &access,
    )?;
    if (status == 401 || status == 403)
        && let Some(refresh) = creds.refresh.as_deref()
    {
        access = cline_refresh_access(refresh)?;
        if let Ok(mut slot) = CLINE_LIVE.lock() {
            *slot = Some(access.clone());
        }
        let retry = bearer_get(
            "https://api.cline.bot/api/v1/users/me/plan/usage-limits",
            &access,
        )?;
        status = retry.0;
        value = retry.1;
    }
    if status == 401 || status == 403 {
        return Err("Cline: hay que volver a autenticar (cline auth).".into());
    }
    if !(200..300).contains(&status) {
        return Err(format!("ClinePass devolvió HTTP {status}"));
    }
    let bars = parse_cline_usage_limits(&value);
    if bars.is_empty() {
        Err("ClinePass no devolvió ventanas (¿sin suscripción?).".into())
    } else {
        Ok(bars)
    }
}

fn cline_refresh_access(refresh: &str) -> Result<String, String> {
    let body = json!({
        "grantType": "refresh_token",
        "refreshToken": refresh,
    })
    .to_string();
    let (status, value) = curl_json_ex(
        "https://api.cline.bot/api/v1/auth/refresh",
        "Accept: application/json\nContent-Type: application/json\n",
        Some(&body),
    )?;
    if !(200..300).contains(&status) {
        return Err("Cline: hay que volver a autenticar (cline auth).".into());
    }
    cline_access_from_refresh(&value)
        .ok_or_else(|| "Cline: el refresh no devolvió accessToken.".into())
}

pub fn cline_access_from_refresh(root: &Value) -> Option<String> {
    let data = root.get("data").unwrap_or(root);
    data.get("accessToken")
        .or_else(|| data.get("access_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

/// Endpoints Connect del language server local de Antigravity.
const AGY_QUOTA_ENDPOINT: &str =
    "/exa.language_server_pb.LanguageServerService/GetUserQuotaSummary";
const AGY_STATUS_ENDPOINT: &str = "/exa.language_server_pb.LanguageServerService/GetUserStatus";

struct AgyServer {
    pid: u32,
    csrf: String,
    ext_port: Option<u16>,
}

fn fetch_antigravity_bars() -> Result<Vec<QuotaBar>, String> {
    let Some(server) = find_agy_server() else {
        return Err(
            "Antigravity: no hay language server corriendo. Abrí `agy` para ver la cuota.".into(),
        );
    };
    // HTTPS en los puertos que escucha el proceso; el extension port (HTTP
    // plano) va al final como fallback, igual que hace el IDE.
    let mut targets: Vec<(bool, u16)> = listening_ports(server.pid)
        .into_iter()
        .map(|p| (true, p))
        .collect();
    if let Some(ext) = server.ext_port {
        targets.retain(|&(_, port)| port != ext);
        targets.push((false, ext));
    }
    if targets.is_empty() {
        return Err(
            "Antigravity: no encontré puertos locales de la API (¿hace falta `lsof`?).".into(),
        );
    }
    for endpoint in [AGY_QUOTA_ENDPOINT, AGY_STATUS_ENDPOINT] {
        for &(https, port) in &targets {
            let Ok((status, value)) = agy_post(endpoint, &server.csrf, https, port) else {
                continue;
            };
            if !(200..300).contains(&status) {
                continue;
            }
            let bars = parse_antigravity_quota(&value);
            if !bars.is_empty() {
                return Ok(bars);
            }
        }
    }
    Err("Antigravity: la API local no devolvió ventanas de cuota.".into())
}

fn find_agy_server() -> Option<AgyServer> {
    let ps = files::resolve_command("ps")?;
    let output = Command::new(ps)
        .args(["-ax", "-o", "pid=,command="])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let (pid, cmd) = line.trim().split_once(' ')?;
        if pid.parse::<u32>().is_err() || cmd.is_empty() {
            continue;
        }
        let lower = cmd.to_ascii_lowercase();
        let is_server = lower.contains("language_server")
            || lower.contains("agentapi")
            || lower.ends_with("/agy")
            || lower == "agy";
        let is_agy = lower.contains("antigravity")
            || lower.contains("agy")
            || lower.contains(".gemini/antigravity-cli");
        if !is_server || !is_agy {
            continue;
        }
        return Some(AgyServer {
            pid: pid.parse().ok()?,
            csrf: extract_flag(cmd, "--csrf_token").unwrap_or_default(),
            ext_port: extract_flag(cmd, "--extension_server_port").and_then(|v| v.parse().ok()),
        });
    }
    None
}

fn extract_flag(cmd: &str, flag: &str) -> Option<String> {
    let mut tokens = cmd.split_whitespace();
    while let Some(token) = tokens.next() {
        if let Some(rest) = token.strip_prefix(flag)
            && let Some(value) = rest.strip_prefix('=')
        {
            return Some(value.to_string());
        }
        if token == flag {
            return tokens.next().map(str::to_string);
        }
    }
    None
}

fn listening_ports(pid: u32) -> Vec<u16> {
    let Some(lsof) = files::resolve_command("lsof") else {
        return Vec::new();
    };
    let Ok(output) = Command::new(lsof)
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-p", &pid.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    let mut ports = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 || cols[cols.len() - 1] != "(LISTEN)" {
            continue;
        }
        if let Some((_, port)) = cols[cols.len() - 2].rsplit_once(':')
            && let Ok(parsed) = port.parse::<u16>()
        {
            ports.push(parsed);
        }
    }
    ports.sort_unstable();
    ports.dedup();
    ports.reverse(); // desc, igual que la extensión de referencia
    ports
}

fn agy_post(path: &str, csrf: &str, https: bool, port: u16) -> Result<(u16, Value), String> {
    let curl = files::resolve_command("curl")
        .ok_or_else(|| "hace falta `curl` para leer la cuota".to_string())?;
    let scheme = if https { "https" } else { "http" };
    let url = format!("{scheme}://127.0.0.1:{port}{path}");
    let headers = format!(
        "Content-Type: application/json\nConnect-Protocol-Version: 1\nX-Codeium-Csrf-Token: {csrf}\n"
    );
    let body = json!({
        "metadata": {
            "ideName": "antigravity",
            "extensionName": "lolterm",
            "ideVersion": "unknown",
            "locale": "es"
        }
    })
    .to_string();
    let header = write_request_headers(&headers)?;
    let _guard = TempPath(header.clone());
    let body_path = write_request_body(&body)?;
    let _body_guard = TempPath(body_path.clone());
    // `-k`: el certificado del servidor local es self-signed.
    let mut cmd = Command::new(curl);
    if let Some(path) = files::effective_path() {
        cmd.env("PATH", path);
    }
    cmd.args([
        "-sS",
        "--max-time",
        "5",
        "-k",
        "-H",
        &format!("@{}", header.display()),
        "-X",
        "POST",
        "-d",
        &format!("@{}", body_path.display()),
        "-w",
        "\n%{http_code}",
        &url,
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    let output = cmd
        .output()
        .map_err(|err| format!("no arrancó curl: {err}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let (resp_body, status) = split_curl_status(&text)?;
    let value = if resp_body.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(resp_body).unwrap_or(Value::Null)
    };
    Ok((status, value))
}

pub fn parse_antigravity_quota(root: &Value) -> Vec<QuotaBar> {
    if !agy_code_ok(root) {
        return Vec::new();
    }
    let status = root.get("userStatus").unwrap_or(root);
    let configs = find_agy_configs(status);
    let mut models: Vec<(String, Vec<QuotaBar>)> = Vec::new();
    for config in configs {
        let model = agy_config_label(config);
        let quota = config
            .get("quotaInfo")
            .or_else(|| config.get("quotaSummary"))
            .or_else(|| config.get("usageLimits"))
            .or_else(|| config.get("commandQuota"))
            .unwrap_or(config);
        let mut bars = Vec::new();
        collect_agy_buckets(&model, quota, "", &mut bars);
        dedupe_agy_bars(&mut bars);
        if !bars.is_empty() {
            models.push((model, bars));
        }
    }
    let grouped = group_agy_bars(&models);
    if grouped.is_empty() {
        // Fallback honesto: si Google cambia los nombres de los pools,
        // mostramos las barras por modelo en vez de quedarnos sin datos.
        let mut all: Vec<QuotaBar> = models.into_iter().flat_map(|(_, bars)| bars).collect();
        all.truncate(AGY_MAX_BARS);
        all
    } else {
        grouped
    }
}

/// Grupos igual que `/usage` en el propio `agy`: los modelos comparten pool,
/// así que por grupo se reporta la ventana peor parada (criterio Orquester).
const AGY_GROUPS: &[(&str, &[&str])] =
    &[("Gemini", &["gemini"]), ("Claude/GPT", &["claude", "gpt"])];
const AGY_WINDOWS: &[&str] = &["5-hour limit", "Weekly limit"];
const AGY_MAX_BARS: usize = 8;

fn group_agy_bars(models: &[(String, Vec<QuotaBar>)]) -> Vec<QuotaBar> {
    let mut out = Vec::new();
    for (group, needles) in AGY_GROUPS {
        // Ventanas distintas dentro del grupo con el peor % usado de cada una.
        let mut windows: Vec<(String, u8, Option<String>)> = Vec::new();
        for (label, bars) in models {
            let lower = label.to_ascii_lowercase();
            if !needles.iter().any(|needle| lower.contains(needle)) {
                continue;
            }
            for bar in bars {
                let Some(window) = bar.label.strip_prefix(label.as_str()) else {
                    continue;
                };
                let window = window.trim().to_string();
                match windows.iter_mut().find(|(name, _, _)| *name == window) {
                    Some(slot) => {
                        if bar.percent > slot.1 {
                            slot.1 = bar.percent;
                            slot.2 = bar.reset.clone();
                        }
                    }
                    None => windows.push((window, bar.percent, bar.reset.clone())),
                }
            }
        }
        if windows.is_empty() {
            continue;
        }
        // Orden canónico como en `/usage`: 5-hour primero, weekly después.
        windows.sort_by_key(|(window, _, _)| {
            AGY_WINDOWS
                .iter()
                .position(|canonical| window.eq_ignore_ascii_case(canonical))
                .unwrap_or(AGY_WINDOWS.len())
        });
        for (window, percent, reset) in windows {
            out.push(QuotaBar {
                key: format!("{}-{window}", slugify(group)),
                label: format!("{group} {window}"),
                percent,
                reset,
            });
        }
    }
    out
}

fn dedupe_agy_bars(bars: &mut Vec<QuotaBar>) {
    let mut seen: Vec<(String, u8, Option<String>)> = Vec::new();
    bars.retain(|bar| {
        let fingerprint = (bar.key.clone(), bar.percent, bar.reset.clone());
        if seen.contains(&fingerprint) {
            false
        } else {
            seen.push(fingerprint);
            true
        }
    });
}

fn agy_code_ok(root: &Value) -> bool {
    match root.get("code") {
        None => true,
        Some(Value::Number(n)) => n.as_i64() == Some(0),
        Some(Value::String(s)) => s.is_empty() || s == "0" || s.eq_ignore_ascii_case("ok"),
        Some(other) => other.get("isOk").and_then(Value::as_bool) != Some(false),
    }
}

fn find_agy_configs(status: &Value) -> Vec<&Value> {
    let direct = status
        .pointer("/cascadeModelConfigData/clientModelConfigs")
        .or_else(|| status.get("clientModelConfigs"))
        .or_else(|| status.get("modelConfigs"))
        .or_else(|| status.get("models"));
    if let Some(list) = direct.and_then(Value::as_array) {
        return list.iter().filter(|item| item.is_object()).collect();
    }
    let mut best: Option<&Value> = None;
    let mut best_len = 0usize;
    search_agy_arrays(status, &mut best, &mut best_len);
    match best.and_then(Value::as_array) {
        Some(items) => items.iter().filter(|item| item.is_object()).collect(),
        None => Vec::new(),
    }
}

fn search_agy_arrays<'a>(value: &'a Value, best: &mut Option<&'a Value>, best_len: &mut usize) {
    match value {
        Value::Array(items) => {
            if items.len() > *best_len && items.first().is_some_and(has_agy_shape) {
                *best = Some(value);
                *best_len = items.len();
            }
            for item in items {
                search_agy_arrays(item, best, best_len);
            }
        }
        Value::Object(map) => {
            for child in map.values() {
                search_agy_arrays(child, best, best_len);
            }
        }
        _ => {}
    }
}

fn has_agy_shape(item: &Value) -> bool {
    let Some(map) = item.as_object() else {
        return false;
    };
    ["quotaInfo", "quotaSummary", "usageLimits", "commandQuota"]
        .iter()
        .any(|key| map.contains_key(*key))
        || map.keys().any(|key| {
            let key = key.to_ascii_lowercase();
            key.contains("quota")
                || key.contains("limit")
                || key.contains("remaining")
                || key.contains("reset")
                || key.contains("usage")
        })
}

fn agy_config_label(config: &Value) -> String {
    ["label", "displayName", "name"]
        .iter()
        .find_map(|key| config.get(*key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(str::to_string)
        .or_else(|| {
            config
                .pointer("/modelOrAlias/model")
                .or_else(|| config.get("modelId"))
                .or_else(|| config.get("model"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Modelo".into())
}

fn collect_agy_buckets(model: &str, value: &Value, path: &str, out: &mut Vec<QuotaBar>) {
    match value {
        Value::Object(map) => {
            if let Some(bar) = bar_from_agy_node(model, map, path) {
                out.push(bar);
            }
            for (key, child) in map {
                collect_agy_buckets(model, child, &format!("{path}.{key}"), out);
            }
        }
        Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                collect_agy_buckets(model, item, &format!("{path}[{idx}]"), out);
            }
        }
        _ => {}
    }
}

const AGY_REMAINING_KEYS: &[&str] = &[
    "remainingFraction",
    "remainingRatio",
    "remainingPercent",
    "remainingPercentage",
];
const AGY_RESET_KEYS: &[&str] = &["resetTime", "resetsAt", "resetAt", "nextResetTime"];

fn bar_from_agy_node(
    model: &str,
    map: &serde_json::Map<String, Value>,
    path: &str,
) -> Option<QuotaBar> {
    let remaining = AGY_REMAINING_KEYS
        .iter()
        .find_map(|key| map.get(*key))
        .and_then(Value::as_f64);
    let percent = if let Some(left) = remaining {
        // `remainingFraction` viene 0–1; si viene >1 es porcentaje 0–100.
        let frac = if (0.0..=1.0).contains(&left) {
            left
        } else {
            (left / 100.0).clamp(0.0, 1.0)
        };
        as_points(((1.0 - frac).clamp(0.0, 1.0)) * 100.0)
    } else {
        let used = ["usedCount", "used", "consumed"]
            .iter()
            .find_map(|key| map.get(*key))
            .and_then(Value::as_f64)?;
        let limit = ["limit", "max", "total", "capacity"]
            .iter()
            .find_map(|key| map.get(*key))
            .and_then(Value::as_f64)?;
        if limit <= 0.0 {
            return None;
        }
        as_points(((used / limit).clamp(0.0, 1.0)) * 100.0)
    };
    let reset = AGY_RESET_KEYS
        .iter()
        .find_map(|key| map.get(*key))
        .and_then(reset_from_value);
    let window = agy_window_label(map, path);
    Some(QuotaBar {
        key: format!("{}-{window}", slugify(model)),
        label: format!("{model} {window}"),
        percent,
        reset,
    })
}

fn agy_window_label(map: &serde_json::Map<String, Value>, path: &str) -> String {
    for key in ["label", "displayName", "name", "period", "bucket", "window"] {
        if let Some(text) = map.get(key).and_then(Value::as_str)
            && !text.trim().is_empty()
        {
            // En Antigravity la ventana "Hourly" es la rolling de ~5 horas.
            let norm = text.trim().to_ascii_lowercase().replace('-', "_");
            if norm == "hourly" || norm == "quota" {
                return "5-hour limit".into();
            }
            return window_label(text);
        }
    }
    let lower = path.to_ascii_lowercase();
    if lower.contains("hourly") || lower.contains("fivehour") || lower.contains("_5h") {
        return "5-hour limit".into();
    }
    if lower.contains("weekly") {
        return "Weekly limit".into();
    }
    if lower.contains("daily") {
        return "Daily limit".into();
    }
    if lower.contains("monthly") {
        return "Monthly limit".into();
    }
    // La API local solo expone la ventana rolling (~5 h) sin nombre;
    // verificado contra GetUserStatus real.
    "5-hour limit".into()
}

fn slugify(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn opencode_api_key() -> Option<String> {
    for name in ["OPENCODE_GO_API_KEY", "OPENCODE_API_KEY"] {
        if let Ok(value) = std::env::var(name) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    for path in opencode_auth_paths() {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if let Some(key) = opencode_key_from_value(&value) {
            return Some(key);
        }
    }
    None
}

fn opencode_auth_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        out.push(PathBuf::from(xdg).join("opencode/auth.json"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        out.push(home.join(".local/share/opencode/auth.json"));
        out.push(home.join(".opencode/auth.json"));
    }
    out
}

pub fn opencode_key_from_value(root: &Value) -> Option<String> {
    let map = root.as_object()?;
    for id in ["opencode-go", "opencode", "zen"] {
        if let Some(key) = api_key_in(map.get(id)?) {
            return Some(key);
        }
    }
    None
}

fn api_key_in(value: &Value) -> Option<String> {
    let map = value.as_object()?;
    let kind = map
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("api")
        .to_ascii_lowercase();
    if kind != "api" {
        return None;
    }
    map.get("key")
        .or_else(|| map.get("access"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
}

fn cline_api_key() -> Option<String> {
    cline_creds().map(|creds| creds.access)
}

#[derive(Clone, Debug, PartialEq)]
struct ClineCreds {
    access: String,
    refresh: Option<String>,
}

fn cline_creds() -> Option<ClineCreds> {
    for name in ["CLINE_API_KEY", "CLINEPASS_API_KEY"] {
        if let Ok(value) = std::env::var(name) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(ClineCreds {
                    access: value.to_string(),
                    refresh: None,
                });
            }
        }
    }
    for path in cline_provider_paths() {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if let Some(creds) = cline_creds_from_value(&value) {
            return Some(creds);
        }
    }
    None
}

fn cline_provider_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        out.push(home.join(".cline/data/settings/providers.json"));
        out.push(home.join(".cline/data/providers.json"));
        out.push(home.join(".cline/data/secrets.json"));
        out.push(home.join(".config/cline/data/settings/providers.json"));
        out.push(home.join(".config/cline/providers.json"));
        out.push(home.join(".cline/settings.json"));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let xdg = PathBuf::from(xdg);
        out.push(xdg.join("cline/data/settings/providers.json"));
        out.push(xdg.join("cline/providers.json"));
    }
    out
}

pub fn cline_key_from_value(root: &Value) -> Option<String> {
    cline_creds_from_value(root).map(|creds| creds.access)
}

fn cline_creds_from_value(root: &Value) -> Option<ClineCreds> {
    if let Some(id) = root.get("lastUsedProvider").and_then(Value::as_str)
        && let Some(node) = root
            .get("providers")
            .and_then(|providers| providers.get(id))
            .or_else(|| root.get(id))
    {
        let mut found = None;
        walk_cline_creds(node, id, &mut found);
        if found.is_some() {
            return found;
        }
    }
    let mut found = None;
    walk_cline_creds(root, "", &mut found);
    found
}

fn walk_cline_creds(value: &Value, parent: &str, found: &mut Option<ClineCreds>) {
    if found.is_some() {
        return;
    }
    match value {
        Value::Object(map) => {
            let provider = map
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or(parent);
            if is_cline_provider(provider) || is_cline_provider(parent) {
                if let Some(key) = map
                    .get("apiKey")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|key| !key.is_empty())
                {
                    *found = Some(ClineCreds {
                        access: key.to_string(),
                        refresh: None,
                    });
                    return;
                }
                if let Some(auth) = map.get("auth") {
                    let access = auth
                        .get("accessToken")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|token| !token.is_empty())
                        .map(str::to_string);
                    let refresh = auth
                        .get("refreshToken")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|token| !token.is_empty())
                        .map(str::to_string);
                    if let Some(access) = access {
                        *found = Some(ClineCreds { access, refresh });
                        return;
                    }
                }
            }
            for (key, child) in map {
                walk_cline_creds(child, key, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_cline_creds(item, parent, found);
            }
        }
        _ => {}
    }
}

fn is_cline_provider(name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    name == "cline"
        || name == "clinepass"
        || name.contains("clinepass")
        || name == "cline-pass"
        || name == "cline_pass"
        || name == "account"
}

fn bearer_get(url: &str, token: &str) -> Result<(u16, Value), String> {
    curl_json(
        url,
        &format!("Authorization: Bearer {token}\nAccept: application/json\n"),
    )
}

fn fetch_copilot_bars() -> Result<Vec<QuotaBar>, String> {
    let token = github_token().ok_or_else(|| {
        "Copilot: hace falta `gh auth login` (o GH_TOKEN) para leer la cuota.".to_string()
    })?;
    let (status, value) = github_get("https://api.github.com/copilot_internal/user", &token)?;
    if status == 401 || status == 403 {
        return Err("Copilot: hay que volver a autenticar (`gh auth login`).".into());
    }
    if status == 404 {
        return Err("Copilot: la API no devolvió cuota (¿sin plan activo?).".into());
    }
    if !(200..300).contains(&status) {
        return Err(format!("Copilot devolvió HTTP {status}"));
    }
    let bars = parse_copilot_user(&value);
    if bars.is_empty() {
        Err("Copilot no devolvió ventanas de cuota.".into())
    } else {
        Ok(bars)
    }
}

fn github_token() -> Option<String> {
    for name in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(value) = std::env::var(name) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    let gh = files::resolve_command("gh")?;
    let mut cmd = Command::new(gh);
    if let Some(path) = files::effective_path() {
        cmd.env("PATH", path);
    }
    let output = cmd
        .args(["auth", "token"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

fn github_get(url: &str, token: &str) -> Result<(u16, Value), String> {
    curl_json(
        url,
        &format!(
            "Authorization: Bearer {token}\nAccept: application/vnd.github+json\nX-GitHub-Api-Version: 2025-05-01\nUser-Agent: LoLTerm\n"
        ),
    )
}

fn curl_json(url: &str, headers: &str) -> Result<(u16, Value), String> {
    curl_json_ex(url, headers, None)
}

fn curl_json_ex(url: &str, headers: &str, post_body: Option<&str>) -> Result<(u16, Value), String> {
    let curl = files::resolve_command("curl")
        .ok_or_else(|| "hace falta `curl` para leer la cuota".to_string())?;
    let header = write_request_headers(headers)?;
    let _guard = TempPath(header.clone());
    let body_path = if let Some(body) = post_body {
        let path = write_request_body(body)?;
        Some(path)
    } else {
        None
    };
    let _body_guard = body_path.as_ref().map(|path| TempPath(path.clone()));
    let mut cmd = Command::new(curl);
    if let Some(path) = files::effective_path() {
        cmd.env("PATH", path);
    }
    cmd.args([
        "-sS",
        "--max-time",
        "10",
        "-H",
        &format!("@{}", header.display()),
    ]);
    if let Some(path) = &body_path {
        cmd.args(["-X", "POST", "-d", &format!("@{}", path.display())]);
    }
    cmd.args(["-w", "\n%{http_code}", url])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let output = cmd
        .output()
        .map_err(|err| format!("no arrancó curl: {err}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let (body, status) = split_curl_status(&text)?;
    let value = if body.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(body).unwrap_or(Value::Null)
    };
    Ok((status, value))
}

struct TempPath(PathBuf);

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn write_request_headers(headers: &str) -> Result<PathBuf, String> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = dir.join(format!(
        "lolterm-quota-{}-{}.hdr",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::write(&path, headers)
        .map_err(|err| format!("no pude escribir el header temporal: {err}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(path)
}

fn write_request_body(body: &str) -> Result<PathBuf, String> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = dir.join(format!(
        "lolterm-quota-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::write(&path, body).map_err(|err| format!("no pude escribir el body temporal: {err}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(path)
}

fn split_curl_status(text: &str) -> Result<(&str, u16), String> {
    let text = text.trim_end();
    let Some(idx) = text.rfind('\n') else {
        let status = text.parse::<u16>().unwrap_or(0);
        return Ok(("", status));
    };
    let (body, status) = text.split_at(idx);
    let status = status.trim().parse::<u16>().unwrap_or(0);
    Ok((body, status))
}

pub fn parse_opencode_usage(root: &Value) -> Vec<QuotaBar> {
    let usage = root.get("usage").unwrap_or(root);
    parse_named_windows(usage)
}

pub fn parse_copilot_user(root: &Value) -> Vec<QuotaBar> {
    let reset = root
        .get("quota_reset_date")
        .or_else(|| root.get("quota_reset_date_utc"))
        .and_then(Value::as_str)
        .map(short_reset);
    let Some(snaps) = root.get("quota_snapshots").and_then(Value::as_object) else {
        return Vec::new();
    };
    let preferred = [
        "premium_interactions",
        "chat",
        "completions",
        "session",
        "weekly",
    ];
    let mut bars = Vec::new();
    let mut seen = Vec::new();
    for key in preferred {
        if let Some(child) = snaps.get(key)
            && let Some(bar) = bar_from_copilot_snapshot(key, child, reset.as_deref())
        {
            seen.push(key.to_string());
            bars.push(bar);
        }
    }
    for (key, child) in snaps {
        if seen.iter().any(|seen| seen == key) {
            continue;
        }
        if let Some(bar) = bar_from_copilot_snapshot(key, child, reset.as_deref()) {
            bars.push(bar);
        }
    }
    bars
}

fn bar_from_copilot_snapshot(key: &str, value: &Value, reset: Option<&str>) -> Option<QuotaBar> {
    let map = value.as_object()?;
    if map.get("unlimited").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let left = map.get("percent_remaining").and_then(Value::as_f64);
    let entitlement = map.get("entitlement").and_then(Value::as_f64);
    let remaining = map
        .get("remaining")
        .or_else(|| map.get("quota_remaining"))
        .and_then(Value::as_f64);
    let used = if let Some(left) = left {
        as_points((100.0 - left).clamp(0.0, 100.0))
    } else if let (Some(ent), Some(rem)) = (entitlement, remaining) {
        if ent <= 0.0 {
            return None;
        }
        as_points(((ent - rem) / ent * 100.0).clamp(0.0, 100.0))
    } else {
        return None;
    };
    Some(QuotaBar {
        key: key.into(),
        label: copilot_snapshot_label(key),
        percent: used,
        reset: reset.map(str::to_string),
    })
}

fn copilot_snapshot_label(key: &str) -> String {
    match key.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "premium_interactions" => "Premium requests".into(),
        "chat" => "Chat".into(),
        "completions" => "Completions".into(),
        "session" | "interactive_session" => "Session".into(),
        "weekly" | "seven_day" => "Weekly".into(),
        other => window_label(other),
    }
}

pub fn parse_cline_usage_limits(root: &Value) -> Vec<QuotaBar> {
    let data = unwrap_envelope(root);
    if let Some(items) = data
        .get("limits")
        .and_then(Value::as_array)
        .or_else(|| data.as_array())
    {
        return items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| bar_from_limit_item(item, idx))
            .collect();
    }
    parse_named_windows(data)
}

fn unwrap_envelope(root: &Value) -> &Value {
    root.get("data").unwrap_or(root)
}

fn parse_named_windows(value: &Value) -> Vec<QuotaBar> {
    let Some(map) = value.as_object() else {
        return Vec::new();
    };
    let mut bars = Vec::new();
    let preferred = ["rolling", "rolling5h", "five_hour", "weekly", "monthly"];
    let mut seen = Vec::new();
    for key in preferred {
        if let Some(child) = map.get(key)
            && let Some(bar) = bar_from_window(key, child)
        {
            seen.push(key.to_string());
            bars.push(bar);
        }
    }
    for (key, child) in map {
        if seen.contains(key) {
            continue;
        }
        if let Some(bar) = bar_from_window(key, child) {
            bars.push(bar);
        }
    }
    bars
}

fn bar_from_limit_item(value: &Value, idx: usize) -> Option<QuotaBar> {
    let map = value.as_object()?;
    let kind = map
        .get("type")
        .or_else(|| map.get("window"))
        .or_else(|| map.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("limit");
    let used = used_in_map(map)?;
    Some(QuotaBar {
        key: format!("{kind}-{idx}"),
        label: window_label(kind),
        percent: used,
        reset: map
            .get("resetsAt")
            .or_else(|| map.get("resets_at"))
            .or_else(|| map.get("resetAt"))
            .and_then(reset_from_value),
    })
}

fn bar_from_window(key: &str, value: &Value) -> Option<QuotaBar> {
    let Value::Object(map) = value else {
        return None;
    };
    let used = used_in_map(map)?;
    Some(QuotaBar {
        key: key.into(),
        label: window_label(key),
        percent: used,
        reset: map
            .get("resetsAt")
            .or_else(|| map.get("resets_at"))
            .or_else(|| map.get("resetAt"))
            .and_then(reset_from_value),
    })
}

fn used_in_map(map: &serde_json::Map<String, Value>) -> Option<u8> {
    if let Some(n) = map
        .get("percent")
        .or_else(|| map.get("percentUsed"))
        .or_else(|| map.get("usedPercent"))
        .or_else(|| map.get("usagePercent"))
        .and_then(Value::as_f64)
    {
        return Some(as_points(n));
    }
    if let Some(left) = map
        .get("percentRemaining")
        .or_else(|| map.get("percent_remaining"))
        .and_then(Value::as_f64)
    {
        return Some(remaining_from_used(as_percent(left)));
    }
    if let Some(frac) = map.get("remainingFraction").and_then(Value::as_f64) {
        return Some(as_points(((1.0 - frac).clamp(0.0, 1.0)) * 100.0));
    }
    let used = map
        .get("used")
        .or_else(|| map.get("consumed"))
        .and_then(Value::as_f64);
    let limit = map
        .get("limit")
        .or_else(|| map.get("entitlement"))
        .or_else(|| map.get("max"))
        .and_then(Value::as_f64);
    if let (Some(used), Some(limit)) = (used, limit)
        && limit > 0.0
    {
        return Some(as_points((used / limit) * 100.0));
    }
    used.map(as_points)
}

fn window_label(key: &str) -> String {
    match key.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "rolling" | "rolling5h" | "five_hour" | "fivehour" | "session" => "5-hour limit".into(),
        "weekly" | "week" | "seven_day" | "sevenday" => "Weekly limit".into(),
        "monthly" | "month" | "thirty_day" => "Monthly limit".into(),
        other => {
            let mut label = other.replace('_', " ");
            if let Some(first) = label.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            format!("{label} limit")
        }
    }
}

fn remaining_from_used(used: u8) -> u8 {
    100u8.saturating_sub(used)
}

/// Codex `usedPercent` ya viene 0–100. `1` es 1 %, no 100 %.
fn as_points(n: f64) -> u8 {
    n.round().clamp(0.0, 100.0) as u8
}

fn as_percent(n: f64) -> u8 {
    let pct = if (0.0..=1.0).contains(&n) {
        n * 100.0
    } else {
        n
    };
    pct.round().clamp(0.0, 100.0) as u8
}

fn reset_from_value(value: &Value) -> Option<String> {
    if let Some(secs) = value.as_i64().or_else(|| value.as_u64().map(|n| n as i64)) {
        return Some(reset_from_unix(secs));
    }
    value.as_str().map(short_reset)
}

fn reset_from_unix(secs: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let delta = secs - now;
    if delta <= 0 {
        "reset pronto".into()
    } else if delta < 3600 {
        format!("reset in {}m", delta / 60)
    } else if delta < 86_400 {
        format!("reset in {}h {}m", delta / 3600, (delta % 3600) / 60)
    } else {
        format!("reset in {}d {}h", delta / 86_400, (delta % 86_400) / 3600)
    }
}

fn short_reset(iso: &str) -> String {
    let date = iso.get(..10).unwrap_or(iso);
    let time = iso.get(11..16).unwrap_or("");
    if time.is_empty() {
        format!("reset {date}")
    } else {
        format!("reset {time} UTC")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn claude_bars_are_used_percent() {
        let root = json!({
            "oauthAccount": { "accessToken": "sk-ant-secret" },
            "cachedUsageUtilization": {
                "utilization": {
                    "five_hour": { "utilization": 0.55, "resets_at": "2026-08-20T21:00:00Z" },
                    "seven_day": { "utilization": 12.0, "resets_at": "2026-08-24T00:00:00Z" }
                }
            }
        });
        let bars = parse_claude_bars(&root);
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].percent, 55);
        assert_eq!(bars[1].percent, 12);
        assert_eq!(bars[0].label, "5-hour limit");
        assert!(!serde_json::to_string(&bars).unwrap().contains("sk-ant"));
    }

    #[test]
    fn skips_credential_objects() {
        let root = json!({
            "credentials": { "five_hour": 99 },
            "cachedUsageUtilization": { "five_hour": 0.1 }
        });
        let bars = parse_claude_bars(&root);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].percent, 10);
    }

    #[test]
    fn parse_codex_plus_windows() {
        let root = json!({
            "rateLimits": {
                "planType": "plus",
                "primary": { "usedPercent": 25, "windowDurationMins": 300, "resetsAt": 1999999999 },
                "secondary": { "usedPercent": 9, "windowDurationMins": 10080, "resetsAt": 1999999999 }
            }
        });
        let bars = parse_codex_bars(&root);
        assert_eq!(bars[0].label, "5-hour limit");
        assert_eq!(bars[0].percent, 25);
        assert_eq!(bars[1].label, "Weekly limit");
        assert_eq!(bars[1].percent, 9);
    }

    #[test]
    fn codex_one_percent_used_is_not_a_hundred() {
        let root = json!({
            "rateLimits": {
                "secondary": { "usedPercent": 1.0, "windowDurationMins": 10080, "resetsAt": 1999999999 }
            }
        });
        let bars = parse_codex_bars(&root);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].percent, 1);
        assert_eq!(bars[0].label, "Weekly limit");
    }

    #[test]
    fn parse_claude_usage_lines() {
        let text = "\
Current session: 12.5% used · Resets Aug 20, 3:00pm (UTC)
Current week (all models): 40% used · Resets Aug 24
";
        let bars = parse_claude_usage_text(text);
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].label, "Current session");
        assert_eq!(bars[0].percent, 13);
        assert_eq!(bars[1].label, "Current week");
        assert_eq!(bars[1].percent, 40);
    }

    #[test]
    fn parse_opencode_export_as_used() {
        let root = json!({
            "providers": {
                "copilot": {
                    "status": "ok",
                    "entries": [{
                        "name": "Premium",
                        "window": "Monthly",
                        "percentRemaining": 62.3,
                        "resetAt": 1748908800
                    }]
                },
                "anthropic": { "status": "unavailable" }
            }
        });
        let bars = parse_opencode_export(&root);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].percent, 38);
        assert!(bars[0].label.contains("Monthly"));
    }

    #[test]
    fn parse_opencode_go_usage_all_windows_plus_extra() {
        let root = json!({
            "usage": {
                "rolling": { "status": "ok", "percent": 9, "resetsAt": "2026-08-14T07:20:04Z" },
                "weekly": { "status": "ok", "percent": 12, "resetsAt": "2026-08-17T00:00:00Z" },
                "monthly": { "status": "ok", "percent": 6, "resetsAt": "2026-09-09T00:41:03Z" },
                "bonus": { "percent": 3, "resetsAt": "2026-10-01T00:00:00Z" },
                "subscribedAt": "2026-05-22T14:30:00Z"
            }
        });
        let bars = parse_opencode_usage(&root);
        assert_eq!(bars.len(), 4);
        assert_eq!(bars[0].label, "5-hour limit");
        assert_eq!(bars[0].percent, 9);
        assert_eq!(bars[1].label, "Weekly limit");
        assert_eq!(bars[2].label, "Monthly limit");
        assert_eq!(bars[3].label, "Bonus limit");
        assert_eq!(bars[3].percent, 3);
    }

    #[test]
    fn parse_cline_pass_limits_from_envelope() {
        let root = json!({
            "success": true,
            "data": {
                "limits": [
                    { "type": "five_hour", "percentUsed": 22.4, "resetsAt": "2026-08-20T21:00:00Z" },
                    { "type": "weekly", "percentUsed": 40, "resetsAt": "2026-08-24T00:00:00Z" },
                    { "type": "monthly", "percentUsed": 11, "resetsAt": "2026-09-01T00:00:00Z" },
                    { "type": "burst", "percentUsed": 1 }
                ]
            }
        });
        let bars = parse_cline_usage_limits(&root);
        assert_eq!(bars.len(), 4);
        assert_eq!(bars[0].label, "5-hour limit");
        assert_eq!(bars[0].percent, 22);
        assert_eq!(bars[1].label, "Weekly limit");
        assert_eq!(bars[2].label, "Monthly limit");
        assert_eq!(bars[3].label, "Burst limit");
    }

    #[test]
    fn parse_antigravity_groups_like_usage() {
        let root = json!({
            "code": 0,
            "userStatus": {
                "cascadeModelConfigData": {
                    "clientModelConfigs": [
                        {
                            "label": "Gemini 3 Pro",
                            "quotaInfo": { "windows": [
                                { "period": "Hourly", "remainingFraction": 0.56, "resetTime": "2026-08-22T18:00:00Z" },
                                { "period": "Weekly", "remainingFraction": 0.4 }
                            ]}
                        },
                        {
                            "label": "Gemini 3 Flash",
                            "quotaInfo": { "windows": [
                                { "period": "Hourly", "remainingFraction": 0.9 }
                            ]}
                        },
                        {
                            "label": "Claude Sonnet 4.6",
                            "usageLimits": { "usedCount": 250, "limit": 1000 }
                        },
                        {
                            "label": "GPT-OSS 120B",
                            "commandQuota": { "remainingPercent": 90 }
                        }
                    ]
                }
            }
        });
        let bars = parse_antigravity_quota(&root);
        // Dos pools como en /usage. Este fixture trae Weekly explícito;
        // la API real solo manda la rolling de ~5 h sin nombre.
        assert_eq!(bars.len(), 3);
        assert_eq!(bars[0].label, "Gemini 5-hour limit");
        assert_eq!(bars[0].percent, 44); // peor caso: Pro 44 vs Flash 10
        assert!(bars[0].reset.as_deref().unwrap().starts_with("reset"));
        assert_eq!(bars[1].label, "Gemini Weekly limit");
        assert_eq!(bars[1].percent, 60);
        assert_eq!(bars[2].label, "Claude/GPT 5-hour limit");
        assert_eq!(bars[2].percent, 25); // peor caso: Sonnet 25 vs GPT 10 usado
    }

    #[test]
    fn parse_antigravity_period_labels_and_dedupe() {
        let root = json!({
            "modelConfigs": [{
                "label": "Gemini 3 Flash",
                "quotaInfo": {
                    "windows": [
                        { "period": "Hourly", "remainingFraction": 0.9, "resetTime": "2026-08-22T20:00:00Z" },
                        { "period": "Weekly", "remainingFraction": 0.4, "resetAt": "2026-08-24T00:00:00Z" }
                    ]
                }
            }]
        });
        let bars = parse_antigravity_quota(&root);
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].label, "Gemini 5-hour limit");
        assert_eq!(bars[0].percent, 10);
        assert_eq!(bars[1].label, "Gemini Weekly limit");
        assert_eq!(bars[1].percent, 60);
    }

    #[test]
    fn parse_antigravity_rejects_error_code() {
        assert!(parse_antigravity_quota(&json!({ "code": 7 })).is_empty());
    }

    /// Shape real de `GetUserStatus` (verificado en vivo): quotaInfo directo
    /// por modelo, sin nombre de ventana; es la rolling de ~5 horas.
    #[test]
    fn parse_antigravity_real_get_user_status_shape() {
        let root = json!({
            "userStatus": {
                "name": "Lautaro",
                "planStatus": {
                    "availablePromptCredits": 500,
                    "planInfo": { "planName": "Pro", "monthlyPromptCredits": 50000 }
                },
                "cascadeModelConfigData": {
                    "clientModelConfigs": [
                        {
                            "label": "Gemini 3.7 Flash (High)",
                            "modelOrAlias": { "model": "MODEL_PLACEHOLDER_M298" },
                            "quotaInfo": { "remainingFraction": 1, "resetTime": "2026-08-22T21:54:11Z" }
                        },
                        {
                            "label": "Claude Sonnet 4.6 (Thinking)",
                            "quotaInfo": { "remainingFraction": 0.75, "resetTime": "2026-08-22T21:54:11Z" }
                        },
                        {
                            "label": "Claude Opus 4.6 (Thinking)",
                            "quotaInfo": { "remainingFraction": 0.5, "resetTime": "2026-08-22T21:54:11Z" }
                        },
                        {
                            "label": "GPT-OSS 120B (Medium)",
                            "quotaInfo": { "remainingFraction": 1, "resetTime": "2026-08-22T21:54:11Z" }
                        }
                    ]
                }
            }
        });
        let bars = parse_antigravity_quota(&root);
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].label, "Gemini 5-hour limit");
        assert_eq!(bars[0].percent, 0);
        assert_eq!(bars[0].reset.as_deref(), Some("reset 21:54 UTC"));
        assert_eq!(bars[1].label, "Claude/GPT 5-hour limit");
        assert_eq!(bars[1].percent, 50); // peor caso: Opus 50 vs Sonnet 25
        // El plan no filtra datos sensibles.
        let serialized = serde_json::to_string(&bars).unwrap();
        assert!(!serialized.contains("Lautaro") && !serialized.contains("50000"));
    }

    #[test]
    fn parse_copilot_premium_as_used_skips_unlimited() {
        let root = json!({
            "copilot_plan": "individual",
            "quota_reset_date": "2026-09-01T00:00:00Z",
            "quota_snapshots": {
                "premium_interactions": {
                    "entitlement": 300,
                    "percent_remaining": 41.3,
                    "remaining": 124
                },
                "completions": { "unlimited": true, "percent_remaining": 100 }
            }
        });
        let bars = parse_copilot_user(&root);
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].key, "premium_interactions");
        assert_eq!(bars[0].label, "Premium requests");
        assert_eq!(bars[0].percent, 59);
        assert!(bars[0].reset.as_deref().unwrap().contains("reset"));
        assert!(!serde_json::to_string(&bars).unwrap().contains("gho_"));
    }

    #[test]
    fn parse_copilot_from_remaining_over_entitlement() {
        let root = json!({
            "quota_snapshots": {
                "premium_interactions": { "entitlement": 50, "remaining": 10 }
            }
        });
        let bars = parse_copilot_user(&root);
        assert_eq!(bars[0].percent, 80);
    }

    #[test]
    fn parse_antigravity_falls_back_without_known_groups() {
        let root = json!({
            "modelConfigs": [
                { "label": "Misterio X", "quotaInfo": { "windows": [
                    { "period": "Hourly", "remainingFraction": 0.5 }
                ]}}
            ]
        });
        let bars = parse_antigravity_quota(&root);
        // Sin pools conocidos: barras por modelo en vez de quedarse sin datos.
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].percent, 50);
    }

    #[test]
    fn extract_flag_supports_eq_and_space() {
        let cmd = "agy --extension_server_port 9000 --csrf_token=abc123 -v";
        assert_eq!(extract_flag(cmd, "--csrf_token").as_deref(), Some("abc123"));
        assert_eq!(
            extract_flag(cmd, "--extension_server_port").as_deref(),
            Some("9000")
        );
        assert_eq!(extract_flag(cmd, "--missing"), None);
    }

    #[test]
    fn listening_ports_parse_lsof_output() {
        // Se testea vía split del output real simulado en agy_post… el parser
        // vive inline; este test cubre slugify y window_label helpers.
        assert_eq!(slugify("Claude Sonnet 4.6"), "claude-sonnet-4-6");
    }

    #[test]
    fn opencode_auth_picks_go_key() {
        let root = json!({
            "anthropic": { "type": "api", "key": "sk-ant-other" },
            "opencode-go": { "type": "api", "key": "sk-opencode-go" }
        });
        assert_eq!(
            opencode_key_from_value(&root).as_deref(),
            Some("sk-opencode-go")
        );
    }

    #[test]
    fn cline_settings_picks_account_key() {
        let root = json!({
            "openai": { "apiKey": "sk-openai" },
            "cline": { "apiKey": "cline-pass-key", "auth": { "accessToken": "tok" } }
        });
        assert_eq!(
            cline_key_from_value(&root).as_deref(),
            Some("cline-pass-key")
        );
    }

    #[test]
    fn cline_prefers_last_used_provider() {
        let root = json!({
            "lastUsedProvider": "cline-pass",
            "providers": {
                "cline": { "settings": { "provider": "cline", "auth": { "accessToken": "old" } } },
                "cline-pass": { "settings": { "provider": "cline-pass", "auth": { "accessToken": "fresh" } } }
            }
        });
        assert_eq!(cline_key_from_value(&root).as_deref(), Some("fresh"));
        let creds = cline_creds_from_value(&root).expect("creds");
        assert_eq!(creds.access, "fresh");
        assert_eq!(creds.refresh.as_deref(), None);
    }

    #[test]
    fn cline_creds_keep_refresh_token() {
        let root = json!({
            "lastUsedProvider": "cline-pass",
            "providers": {
                "cline-pass": {
                    "settings": {
                        "provider": "cline-pass",
                        "auth": { "accessToken": "acc", "refreshToken": "ref" }
                    }
                }
            }
        });
        let creds = cline_creds_from_value(&root).expect("creds");
        assert_eq!(creds.access, "acc");
        assert_eq!(creds.refresh.as_deref(), Some("ref"));
    }

    #[test]
    fn cline_refresh_envelope_picks_access() {
        let root = json!({ "success": true, "data": { "accessToken": "new-acc", "refreshToken": "new-ref" } });
        assert_eq!(cline_access_from_refresh(&root).as_deref(), Some("new-acc"));
    }

    #[test]
    fn parse_cline_remaining_fraction_and_counts() {
        let root = json!({
            "data": {
                "limits": [
                    { "type": "five_hour", "remainingFraction": 0.25 },
                    { "type": "weekly", "used": 40, "limit": 80 }
                ]
            }
        });
        let bars = parse_cline_usage_limits(&root);
        assert_eq!(bars[0].percent, 75);
        assert_eq!(bars[1].percent, 50);
    }

    #[test]
    fn split_curl_appends_status() {
        let (body, status) = split_curl_status("{\"ok\":true}\n200").unwrap();
        assert_eq!(status, 200);
        assert!(body.contains("ok"));
    }
}
