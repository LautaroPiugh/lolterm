use std::fs;
use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;
use serde::Deserialize;

#[derive(Clone, Copy)]
pub struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub ch: char,
}

impl Chord {
    pub fn label(self) -> String {
        let mut text = String::new();
        if self.ctrl {
            text.push_str("C-");
        }
        if self.alt {
            text.push_str("A-");
        }
        text.push(self.ch);
        text
    }

    pub fn matches(self, key: KeyEvent) -> bool {
        let KeyCode::Char(ch) = key.code else {
            return false;
        };
        ch.eq_ignore_ascii_case(&self.ch)
            && key.modifiers.contains(KeyModifiers::CONTROL) == self.ctrl
            && key.modifiers.contains(KeyModifiers::ALT) == self.alt
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub focus: Color,
    pub inactive: Color,
    pub workspace: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            focus: Color::Cyan,
            inactive: Color::DarkGray,
            workspace: Color::Yellow,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Config {
    pub prefix: Chord,
    pub quit: Chord,
    pub new_tab: Option<Chord>,
    pub split_right: Option<Chord>,
    pub split_down: Option<Chord>,
    pub theme: Theme,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: Chord {
                ctrl: true,
                alt: false,
                ch: 'b',
            },
            quit: Chord {
                ctrl: true,
                alt: false,
                ch: 'q',
            },
            new_tab: None,
            split_right: None,
            split_down: None,
            theme: Theme::default(),
        }
    }
}

#[derive(Default, Deserialize)]
struct FileConfig {
    #[serde(default)]
    keys: FileKeys,
    #[serde(default)]
    theme: FileTheme,
}

#[derive(Deserialize)]
struct FileKeys {
    #[serde(default = "default_prefix")]
    prefix: String,
    #[serde(default = "default_quit")]
    quit: String,
    #[serde(default)]
    new_tab: Option<String>,
    #[serde(default)]
    split_right: Option<String>,
    #[serde(default)]
    split_down: Option<String>,
}

impl Default for FileKeys {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
            quit: default_quit(),
            new_tab: None,
            split_right: None,
            split_down: None,
        }
    }
}

#[derive(Default, Deserialize)]
struct FileTheme {
    #[serde(default)]
    focus: Option<String>,
    #[serde(default)]
    inactive: Option<String>,
    #[serde(default)]
    workspace: Option<String>,
}

fn default_prefix() -> String {
    "ctrl+b".to_string()
}

fn default_quit() -> String {
    "ctrl+q".to_string()
}

fn path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("lolterm").join("config.toml")
}

pub fn load() -> Config {
    let Ok(text) = fs::read_to_string(path()) else {
        return Config::default();
    };
    let parsed: FileConfig = toml::from_str(&text).unwrap_or_default();
    let defaults = Config::default();
    Config {
        prefix: parse_chord(&parsed.keys.prefix).unwrap_or(defaults.prefix),
        quit: parse_chord(&parsed.keys.quit).unwrap_or(defaults.quit),
        new_tab: parsed.keys.new_tab.as_deref().and_then(parse_chord),
        split_right: parsed.keys.split_right.as_deref().and_then(parse_chord),
        split_down: parsed.keys.split_down.as_deref().and_then(parse_chord),
        theme: Theme {
            focus: parsed
                .theme
                .focus
                .as_deref()
                .and_then(parse_color)
                .unwrap_or(defaults.theme.focus),
            inactive: parsed
                .theme
                .inactive
                .as_deref()
                .and_then(parse_color)
                .unwrap_or(defaults.theme.inactive),
            workspace: parsed
                .theme
                .workspace
                .as_deref()
                .and_then(parse_color)
                .unwrap_or(defaults.theme.workspace),
        },
    }
}

fn parse_chord(raw: &str) -> Option<Chord> {
    let mut ctrl = false;
    let mut alt = false;
    let mut ch = None;
    for part in raw
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" => alt = true,
            "shift" => {}
            other if other.chars().count() == 1 => ch = other.chars().next(),
            _ => return None,
        }
    }
    Some(Chord { ctrl, alt, ch: ch? })
}

fn parse_color(raw: &str) -> Option<Color> {
    Some(match raw.trim().to_ascii_lowercase().as_str() {
        "cyan" => Color::Cyan,
        "yellow" => Color::Yellow,
        "darkgray" | "darkgrey" | "gray" | "grey" => Color::DarkGray,
        "white" => Color::White,
        "black" => Color::Black,
        "blue" => Color::Blue,
        "green" => Color::Green,
        "magenta" => Color::Magenta,
        "red" => Color::Red,
        "lightcyan" => Color::LightCyan,
        "lightyellow" => Color::LightYellow,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ctrl_b() {
        let chord = parse_chord("ctrl+b").unwrap();
        assert!(chord.ctrl);
        assert_eq!(chord.ch, 'b');
    }

    #[test]
    fn parses_theme_color() {
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("nope"), None);
    }
}
