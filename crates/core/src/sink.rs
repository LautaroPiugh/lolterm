//! Volumen del **sink** de la PC (PipeWire / PulseAudio), no el de un player.
//! `wpctl` (WirePlumber) o `pactl`. LoLTerm no implementa un mixer propio.

use std::process::{Command, Stdio};

use crate::files;

pub fn get() -> Option<f64> {
    if let Some(text) = run("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])
        && let Some(value) = parse_wpctl(&text)
    {
        return Some(value);
    }
    if let Some(text) = run("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]) {
        return parse_pactl(&text);
    }
    None
}

pub fn set(volume: f64) -> bool {
    let v = volume.clamp(0.0, 1.0);
    let percent = format!("{:.0}%", v * 100.0);
    let unit = format!("{v:.3}");
    run("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", &unit]).is_some()
        || run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &percent]).is_some()
}

pub fn available() -> bool {
    files::command_on_path("wpctl") || files::command_on_path("pactl")
}

fn run(bin: &str, args: &[&str]) -> Option<String> {
    if !files::command_on_path(bin) {
        return None;
    }
    let mut cmd = Command::new(bin);
    if let Some(path) = files::effective_path() {
        cmd.env("PATH", path);
    }
    let out = cmd
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn parse_wpctl(text: &str) -> Option<f64> {
    // Volume: 0.45    /    Volume: 0.45 [MUTED]
    let line = text.lines().next()?.trim();
    let rest = line.strip_prefix("Volume:")?.trim();
    let token = rest.split_whitespace().next()?;
    let n: f64 = token.parse().ok()?;
    Some(n.clamp(0.0, 1.0))
}

pub fn parse_pactl(text: &str) -> Option<f64> {
    // Volume: front-left: 29491 /  45% / -20.00 dB,   front-right: ...
    let pct = text.split('%').next()?.rsplit_once(' ')?.1.trim();
    let n: f64 = pct.parse().ok()?;
    Some((n / 100.0).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wpctl_volume_line() {
        assert!((parse_wpctl("Volume: 0.45\n").unwrap() - 0.45).abs() < 0.001);
        assert!((parse_wpctl("Volume: 1.00 [MUTED]\n").unwrap() - 1.0).abs() < 0.001);
    }

    #[test]
    fn pactl_percent() {
        let text = "Volume: front-left: 29491 /  45% / -20.00 dB,   front-right: 29491 /  45% / -20.00 dB\n";
        assert!((parse_pactl(text).unwrap() - 0.45).abs() < 0.001);
    }
}
