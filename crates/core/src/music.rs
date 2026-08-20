//! Now-playing vía **playerctl** (MPRIS). Incluye pestañas web (YouTube,
//! Spotify Web) cuando el navegador publica un player. LoLTerm no habla con
//! las APIs de YouTube/Spotify.

use std::fs;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;

use crate::files;

static SELECTED: Mutex<Option<String>> = Mutex::new(None);
static ART: Mutex<Option<(String, Option<String>)>> = Mutex::new(None);

const META_FMT: &str = "{{playerName}}\x1f{{status}}\x1f{{artist}}\x1f{{title}}\x1f{{volume}}\x1f{{xesam:url}}\x1f{{mpris:artUrl}}";

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NowPlaying {
    pub playing: bool,
    pub artist: String,
    pub title: String,
    /// 0.0–1.0 según playerctl.
    pub volume: f64,
    pub player: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art: Option<String>,
}

#[derive(Clone, Debug)]
struct Row {
    player: String,
    status: String,
    artist: String,
    title: String,
    volume: f64,
    url: String,
    art_url: String,
}

pub fn available() -> bool {
    files::command_on_path("playerctl")
}

pub fn now() -> Option<NowPlaying> {
    let rows = list_rows()?;
    let row = pick_row(&rows)?;
    remember(&row.player);
    Some(to_now(row))
}

pub fn play_pause() -> Result<(), String> {
    target_run(&["play-pause"])
}

pub fn next() -> Result<(), String> {
    target_run(&["next"])
}

pub fn previous() -> Result<(), String> {
    target_run(&["previous"])
}

pub fn action(name: &str, volume: Option<f64>) -> Result<(), String> {
    match name {
        "playPause" | "play-pause" | "music.playPause" => play_pause(),
        "next" | "music.next" => next(),
        "prev" | "previous" | "music.prev" => previous(),
        "volume" | "music.volume" => {
            if crate::sink::set(volume.unwrap_or(0.5)) {
                Ok(())
            } else {
                Err("no pude cambiar el volumen del sistema (wpctl/pactl)".into())
            }
        }
        other => Err(format!("acción de media desconocida: {other}")),
    }
}

fn missing() -> String {
    if available() {
        "ningún reproductor MPRIS (YouTube/Spotify Web en el navegador, o un player nativo)".into()
    } else {
        "instalá playerctl para controlar media".into()
    }
}

fn remember(player: &str) {
    if let Ok(mut slot) = SELECTED.lock() {
        *slot = Some(player.to_string());
    }
}

fn selected_player() -> Option<String> {
    SELECTED
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .or_else(|| player_names().into_iter().next())
}

fn target_run(args: &[&str]) -> Result<(), String> {
    if let Some(player) = selected_player()
        && player_ok(Some(&player), args)
    {
        return Ok(());
    }
    if player_ok(None, args) {
        return Ok(());
    }
    Err(missing())
}

fn player_names() -> Vec<String> {
    player_text(None, &["-l"])
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn list_rows() -> Option<Vec<Row>> {
    let mut rows = Vec::new();
    let names = player_names();
    if names.is_empty() {
        if let Some(text) = player_text(None, &["-a", "metadata", "--format", META_FMT]) {
            rows.extend(text.lines().filter_map(parse_row));
        }
    } else {
        for name in names {
            if let Some(text) = player_text(Some(&name), &["metadata", "--format", META_FMT]) {
                rows.extend(text.lines().filter_map(parse_row));
            }
        }
    }
    if rows.is_empty() { None } else { Some(rows) }
}

fn pick_row(rows: &[Row]) -> Option<&Row> {
    let playing: Vec<&Row> = rows
        .iter()
        .filter(|row| row.status.eq_ignore_ascii_case("Playing"))
        .collect();
    playing
        .iter()
        .copied()
        .find(|row| is_streaming_site(row))
        .or_else(|| playing.iter().copied().find(|row| is_browser(row)))
        .or_else(|| playing.first().copied())
        .or_else(|| rows.iter().find(|row| is_streaming_site(row)))
        .or_else(|| rows.iter().find(|row| is_browser(row)))
        .or_else(|| rows.first())
}

fn is_streaming_site(row: &Row) -> bool {
    let url = row.url.to_ascii_lowercase();
    let player = row.player.to_ascii_lowercase();
    url.contains("youtube.com")
        || url.contains("youtu.be")
        || url.contains("music.youtube.com")
        || url.contains("open.spotify.com")
        || url.contains("spotify.com")
        || player.contains("spotify")
}

fn is_browser(row: &Row) -> bool {
    let player = row.player.to_ascii_lowercase();
    player.contains("chromium")
        || player.contains("chrome")
        || player.contains("firefox")
        || player.contains("brave")
        || player.contains("vivaldi")
        || player.contains("edge")
}

fn source_label(row: &Row) -> String {
    let url = row.url.to_ascii_lowercase();
    let player = row.player.to_ascii_lowercase();
    if url.contains("youtube.com") || url.contains("youtu.be") || url.contains("music.youtube.com")
    {
        "YouTube".into()
    } else if url.contains("spotify") || player.contains("spotify") {
        "Spotify".into()
    } else if player.contains("firefox") {
        "Firefox".into()
    } else if player.contains("brave") {
        "Brave".into()
    } else if player.contains("chromium") || player.contains("chrome") {
        "Chrome".into()
    } else {
        short_player(&row.player)
    }
}

fn short_player(name: &str) -> String {
    name.split('.').next().unwrap_or(name).to_string()
}

fn to_now(row: &Row) -> NowPlaying {
    NowPlaying {
        playing: row.status.eq_ignore_ascii_case("Playing"),
        artist: row.artist.clone(),
        title: if row.title.is_empty() {
            "Sin título".into()
        } else {
            row.title.clone()
        },
        volume: row.volume,
        player: row.player.clone(),
        source: source_label(row),
        art: resolve_art(&row.art_url, &row.url),
    }
}

fn parse_row(line: &str) -> Option<Row> {
    let mut parts = line.split('\x1f');
    let player = parts.next()?.trim().to_string();
    if player.is_empty() {
        return None;
    }
    let status = parts.next().unwrap_or("").trim().to_string();
    let artist = parts.next().unwrap_or("").trim().to_string();
    let title = parts.next().unwrap_or("").trim().to_string();
    let volume = parse_volume(parts.next().unwrap_or(""));
    let url = parts.next().unwrap_or("").trim().to_string();
    let art_url = parts.next().unwrap_or("").trim().to_string();
    if title.is_empty() && artist.is_empty() && status.is_empty() {
        return None;
    }
    Some(Row {
        player,
        status,
        artist,
        title,
        volume,
        url,
        art_url,
    })
}

fn parse_volume(raw: &str) -> f64 {
    let t = raw.trim().trim_end_matches('%');
    let n: f64 = t.parse().unwrap_or(0.0);
    if n > 1.0 {
        (n / 100.0).clamp(0.0, 1.0)
    } else {
        n.clamp(0.0, 1.0)
    }
}

fn resolve_art(art_url: &str, page_url: &str) -> Option<String> {
    let key = if art_url.is_empty() {
        page_url
    } else {
        art_url
    };
    if key.is_empty() {
        return youtube_thumb(page_url);
    }
    if let Ok(cache) = ART.lock()
        && let Some((cached_key, value)) = cache.as_ref()
        && cached_key == key
    {
        return value.clone().or_else(|| youtube_thumb(page_url));
    }
    let value = art_from_url(art_url).or_else(|| youtube_thumb(page_url));
    if let Ok(mut cache) = ART.lock() {
        *cache = Some((key.to_string(), value.clone()));
    }
    value
}

fn art_from_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    if url.starts_with("https://") || url.starts_with("http://") {
        return Some(url.to_string());
    }
    let path = url.strip_prefix("file://")?;
    let path = percent_decode(path);
    let bytes = fs::read(&path).ok()?;
    if bytes.is_empty() || bytes.len() > 400_000 {
        return None;
    }
    let mime = if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".webp") {
        "image/webp"
    } else if path.ends_with(".gif") {
        "image/gif"
    } else {
        "image/jpeg"
    };
    Some(format!("data:{mime};base64,{}", b64(&bytes)))
}

fn youtube_thumb(page: &str) -> Option<String> {
    let id = youtube_id(page)?;
    Some(format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"))
}

fn youtube_id(page: &str) -> Option<String> {
    let page = page.trim();
    if let Some(rest) = page.split("v=").nth(1) {
        let id = rest.split('&').next()?.split('#').next()?.trim();
        if id.len() >= 8 {
            return Some(id.to_string());
        }
    }
    if let Some(rest) = page.split("youtu.be/").nth(1) {
        let id = rest.split(['?', '&', '/']).next()?.trim();
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    None
}

fn percent_decode(input: &str) -> String {
    let mut out = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2]))
        {
            out.push((hi << 4) | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn player_ok(player: Option<&str>, args: &[&str]) -> bool {
    player_output(player, args, false).is_some()
}

fn player_text(player: Option<&str>, args: &[&str]) -> Option<String> {
    player_output(player, args, true)
}

fn player_output(player: Option<&str>, args: &[&str], need_stdout: bool) -> Option<String> {
    let mut cmd = Command::new("playerctl");
    if let Some(path) = files::effective_path() {
        cmd.env("PATH", path);
    }
    if let Some(name) = player {
        cmd.args(["-p", name]);
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
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if need_stdout && text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_playerctl_format() {
        let now = to_now(
            &parse_row("chromium.instance123\x1fPlaying\x1fRadiohead\x1fWeird Fishes\x1f0.42\x1fhttps://www.youtube.com/watch?v=abcdefghijk\x1f").unwrap(),
        );
        assert!(now.playing);
        assert_eq!(now.artist, "Radiohead");
        assert_eq!(now.title, "Weird Fishes");
        assert_eq!(now.source, "YouTube");
        assert_eq!(
            now.art.as_deref(),
            Some("https://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg")
        );
        assert!((now.volume - 0.42).abs() < 0.001);
    }

    #[test]
    fn volume_percent_becomes_unit() {
        assert!((parse_volume("80%") - 0.8).abs() < 0.001);
        assert!((parse_volume("0.25") - 0.25).abs() < 0.001);
    }

    #[test]
    fn prefers_playing_youtube_over_paused_native() {
        let rows = vec![
            parse_row("vlc\x1fPaused\x1f\x1fLocal file\x1f0.5\x1ffile:///tmp/a.mp3\x1f").unwrap(),
            parse_row("chromium.instance1\x1fPlaying\x1f\x1fLo-fi mix\x1f1\x1fhttps://www.youtube.com/watch?v=1\x1f").unwrap(),
        ];
        let picked = pick_row(&rows).unwrap();
        assert_eq!(picked.player, "chromium.instance1");
        assert_eq!(source_label(picked), "YouTube");
    }

    #[test]
    fn youtube_id_from_watch_url() {
        assert_eq!(
            youtube_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=3"),
            Some("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn prefers_playing_spotify_web() {
        let rows = vec![
            parse_row("firefox.instance_2\x1fPlaying\x1fArtist\x1fTrack\x1f0.4\x1fhttps://open.spotify.com/track/1\x1fhttps://i.scdn.co/image/ab").unwrap(),
        ];
        assert_eq!(source_label(pick_row(&rows).unwrap()), "Spotify");
    }
}
