use std::collections::BTreeMap;

use serde::Serialize;

use crate::config;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Binding {
    pub chord: String,
    pub command: String,
}

pub fn defaults() -> Vec<Binding> {
    vec![
        bind("ctrl+b", "ui.palette"),
        bind("ctrl+p", "ui.palette"),
        bind("ctrl+alt+h", "pane.focusLeft"),
        bind("ctrl+alt+l", "pane.focusRight"),
        bind("ctrl+alt+k", "pane.focusUp"),
        bind("ctrl+alt+j", "pane.focusDown"),
        bind("ctrl+alt+shift+h", "pane.swapLeft"),
        bind("ctrl+alt+shift+l", "pane.swapRight"),
        bind("ctrl+alt+shift+k", "pane.swapUp"),
        bind("ctrl+alt+shift+j", "pane.swapDown"),
        bind("ctrl+alt+z", "pane.zoom"),
        bind("ctrl+alt+n", "tab.new"),
        bind("ctrl+alt+d", "tab.duplicate"),
        bind("ctrl+alt+w", "tab.close"),
        bind("ctrl+alt+x", "pane.close"),
    ]
}

pub fn load() -> Vec<Binding> {
    let mut map: BTreeMap<String, String> = defaults()
        .into_iter()
        .map(|item| (item.chord, item.command))
        .collect();
    if let Ok(text) = std::fs::read_to_string(keybindings_path())
        && let Ok(file) = toml::from_str::<FileKeys>(&text)
    {
        for (chord, command) in file.keys {
            let chord = normalize_chord(&chord);
            if command.trim().is_empty() {
                map.remove(&chord);
            } else {
                map.insert(chord, command);
            }
        }
    }
    map.into_iter()
        .map(|(chord, command)| Binding { chord, command })
        .collect()
}

pub fn keybindings_path() -> std::path::PathBuf {
    config::config_dir().join("keybindings.toml")
}

pub fn normalize_chord(raw: &str) -> String {
    let mut mods = Vec::new();
    let mut key = String::new();
    for part in raw.split('+') {
        let part = part.trim().to_ascii_lowercase();
        match part.as_str() {
            "control" | "ctrl" => mods.push("ctrl"),
            "alt" | "option" => mods.push("alt"),
            "meta" | "cmd" | "super" => mods.push("meta"),
            "shift" => mods.push("shift"),
            other => key = other.to_string(),
        }
    }
    mods.sort_unstable();
    mods.dedup();
    if key.is_empty() {
        return mods.join("+");
    }
    if mods.is_empty() {
        key
    } else {
        format!("{}+{key}", mods.join("+"))
    }
}

#[derive(Default, serde::Deserialize)]
struct FileKeys {
    #[serde(default)]
    keys: BTreeMap<String, String>,
}

fn bind(chord: &str, command: &str) -> Binding {
    Binding {
        chord: normalize_chord(chord),
        command: command.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_orders_modifiers() {
        assert_eq!(normalize_chord("Shift+Ctrl+Alt+H"), "alt+ctrl+shift+h");
        assert_eq!(normalize_chord("ctrl+b"), "ctrl+b");
    }

    #[test]
    fn defaults_include_palette_and_zoom() {
        let chords: Vec<_> = defaults().into_iter().map(|b| b.command).collect();
        assert!(chords.contains(&"ui.palette".into()));
        assert!(chords.contains(&"pane.zoom".into()));
    }
}
