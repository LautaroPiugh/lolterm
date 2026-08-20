//! Cuotas de agentes (criterio Orquester: solo instalados con ventanas reales).
//!
//! LoLTerm **no** guarda tokens ni pega a api.openai.com / api.anthropic.com.
//! - Claude Code: `claude --print /usage`, con fallback a `~/.claude.json`.
//! - Codex / ChatGPT+: `codex app-server --listen stdio://` + `account/rateLimits/read`.
//! - OpenCode: sólo si hay export de opencode-quota (OpenCode no tiene cuota Plus).
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
        if agent.supported {
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
    let bars = read_opencode_export();
    QuotaAgent {
        id: "opencode".into(),
        label: "OpenCode".into(),
        available,
        running,
        pending: false,
        supported: !bars.is_empty(),
        note: None,
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
    push_codex_window(&mut bars, limits.get("primary"), "primary", "Primary limit");
    push_codex_window(
        &mut bars,
        limits.get("secondary"),
        "secondary",
        "Secondary limit",
    );
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
}
