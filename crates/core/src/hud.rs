//! Foto liviana para el chrome (cuota + now-playing). No es el snapshot del mux.

use serde::Serialize;

use crate::host::{self, HostStats};
use crate::music::{self, NowPlaying};
use crate::quota::{self, QuotaAgent};
use crate::sink;

#[derive(Clone, Debug, Serialize)]
pub struct Hud {
    pub playerctl: bool,
    pub sink: bool,
    pub volume: f64,
    pub music: Option<NowPlaying>,
    pub quota: Vec<QuotaAgent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<HostStats>,
    #[serde(default)]
    pub extra: crate::inspect::HostExtra,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
}

pub fn snapshot(running: &[String]) -> Hud {
    let host = host::stats();
    Hud {
        playerctl: music::available(),
        sink: sink::available(),
        volume: sink::get().unwrap_or(0.0),
        music: music::now(),
        quota: quota::agents(running),
        host: (host.load.is_some() || host.mem.is_some()).then_some(host),
        extra: crate::inspect::HostExtra::default(),
        notice: None,
    }
}

pub fn after_music(running: &[String], action: &str, volume: Option<f64>) -> Hud {
    let before = music::now();
    let notice = music::action(action, volume).err();
    let mut hud = snapshot(running);
    hud.notice = notice;
    if let Some(level) = volume
        && matches!(action, "volume" | "music.volume")
    {
        hud.volume = level.clamp(0.0, 1.0);
    }
    if matches!(action, "playPause" | "play-pause" | "music.playPause")
        && let (Some(old), Some(now)) = (before.as_ref(), hud.music.as_mut())
        && now.playing == old.playing
    {
        now.playing = !old.playing;
    }
    hud
}
