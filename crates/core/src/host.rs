//! CPU/RAM livianos para la status bar. Lee `/proc` en Linux; si no hay, omite.

use std::fs;

use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
pub struct HostStats {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem: Option<u8>,
}

pub fn stats() -> HostStats {
    HostStats {
        load: loadavg(),
        mem: mem_used_pct(),
    }
}

fn loadavg() -> Option<String> {
    let text = fs::read_to_string("/proc/loadavg").ok()?;
    text.split_whitespace().next().map(str::to_string)
}

fn mem_used_pct() -> Option<u8> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let mut total = None;
    let mut available = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available = parse_kb(rest);
        }
    }
    let total = total?;
    let available = available?;
    if total == 0 {
        return None;
    }
    let used = 100.0 * (total - available) as f64 / total as f64;
    Some(used.round().clamp(0.0, 100.0) as u8)
}

fn parse_kb(rest: &str) -> Option<u64> {
    rest.split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_meminfo_line() {
        assert_eq!(parse_kb("       16384000 kB"), Some(16_384_000));
    }
}
