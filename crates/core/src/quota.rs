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
    if files::command_on_path("opencode") {
        let agent = opencode(running);
        if agent.supported || agent.pending || agent.note.is_some() {
            out.push(agent);
        }
    }
    if files::command_on_path("cline") {
        let agent = cline(running);
        if agent.supported || agent.pending || agent.note.is_some() {
            out.push(agent);
        }
    }
    if files::command_on_path("copilot") {
        let agent = copilot(running);
        if agent.supported || agent.pending || agent.note.is_some() {
            out.push(agent);
        }
    }
    out
}

fn running_has(running: &[String], name: &str) -> bool {
    running.iter().any(|item| item == name)
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
    let available = files::command_on_path("cline");
    let running = running_has(running, "cline");
    let bars = CLINE_BARS
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();
    let err = CLINE_ERR.lock().ok().and_then(|g| g.clone());
    let pending = bars.is_empty() && err.is_none() && available;
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
    if !files::command_on_path("opencode") {
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
    if !files::command_on_path("cline") {
        return;
    }
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
                return Err("OpenCode: la key no tiene suscripción Go.".into());
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
    let key = cline_api_key().ok_or_else(|| {
        "Cline: no hay key en ~/.cline/data/settings/providers.json. En el pane: cline auth."
            .to_string()
    })?;
    let (status, value) = bearer_get(
        "https://api.cline.bot/api/v1/users/me/plan/usage-limits",
        &key,
    )?;
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
    for name in ["CLINE_API_KEY", "CLINEPASS_API_KEY"] {
        if let Ok(value) = std::env::var(name) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
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
        if let Some(key) = cline_key_from_value(&value) {
            return Some(key);
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
    }
    out
}

pub fn cline_key_from_value(root: &Value) -> Option<String> {
    let mut found = None;
    walk_cline_key(root, "", &mut found);
    found
}

fn walk_cline_key(value: &Value, parent: &str, found: &mut Option<String>) {
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
                    *found = Some(key.to_string());
                    return;
                }
                if let Some(key) = map
                    .get("auth")
                    .and_then(|auth| auth.get("accessToken"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|key| !key.is_empty())
                {
                    *found = Some(key.to_string());
                    return;
                }
            }
            for (key, child) in map {
                walk_cline_key(child, key, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_cline_key(item, parent, found);
            }
        }
        _ => {}
    }
}

fn is_cline_provider(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "cline" | "clinepass" | "cline-pass" | "cline_pass"
    )
}

fn bearer_get(url: &str, token: &str) -> Result<(u16, Value), String> {
    curl_json(
        url,
        &format!("Authorization: Bearer {token}\nAccept: application/json\n"),
    )
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
    let curl = files::resolve_command("curl")
        .ok_or_else(|| "hace falta `curl` para leer la cuota".to_string())?;
    let header = write_request_headers(headers)?;
    let _guard = TempPath(header.clone());
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
        "-w",
        "\n%{http_code}",
        url,
    ])
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
    map.get("percent")
        .or_else(|| map.get("percentUsed"))
        .or_else(|| map.get("usedPercent"))
        .or_else(|| map.get("usagePercent"))
        .or_else(|| map.get("used"))
        .and_then(Value::as_f64)
        .map(as_points)
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
    fn split_curl_appends_status() {
        let (body, status) = split_curl_status("{\"ok\":true}\n200").unwrap();
        assert_eq!(status, 200);
        assert!(body.contains("ok"));
    }
}
