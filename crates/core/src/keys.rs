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
        bind("ctrl+alt+v", "pane.splitRight"),
        bind("ctrl+alt+s", "pane.splitDown"),
        bind("ctrl+alt+r", "pane.restart"),
        bind("ctrl+alt+e", "ui.tabRename"),
        bind("ctrl+alt+n", "tab.new"),
        bind("ctrl+alt+d", "tab.duplicate"),
        bind("ctrl+tab", "tab.next"),
        bind("ctrl+shift+tab", "tab.prev"),
        bind("ctrl+alt+w", "tab.close"),
        bind("ctrl+alt+x", "pane.close"),
        bind("ctrl+alt+[", "workspace.prev"),
        bind("ctrl+alt+]", "workspace.next"),
        bind("ctrl+alt+,", "ui.commands"),
    ]
}

pub fn load() -> Vec<Binding> {
    load_at(&keybindings_path())
}

pub fn keybindings_path() -> std::path::PathBuf {
    config::config_dir().join("keybindings.toml")
}

pub fn apply(chord: &str, command: &str) -> std::io::Result<Vec<Binding>> {
    apply_at(&keybindings_path(), chord, command)
}

pub fn apply_at(
    path: &std::path::Path,
    chord: &str,
    command: &str,
) -> std::io::Result<Vec<Binding>> {
    let chord = normalize_chord(chord);
    if chord.is_empty() {
        return Ok(load_at(path));
    }
    let mut map: BTreeMap<String, String> = load_at(path)
        .into_iter()
        .map(|item| (item.chord, item.command))
        .collect();
    if command.trim().is_empty() {
        map.remove(&chord);
    } else {
        let command = command.trim().to_string();
        map.retain(|_, bound| bound != &command);
        map.insert(chord, command);
    }
    let bindings: Vec<Binding> = map
        .into_iter()
        .map(|(chord, command)| Binding { chord, command })
        .collect();
    save_at(path, &bindings)?;
    Ok(bindings)
}

pub fn reset() -> std::io::Result<Vec<Binding>> {
    reset_at(&keybindings_path())
}

pub fn reset_at(path: &std::path::Path) -> std::io::Result<Vec<Binding>> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(defaults())
}

pub fn save_at(path: &std::path::Path, bindings: &[Binding]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let default_map: BTreeMap<String, String> = defaults()
        .into_iter()
        .map(|item| (item.chord, item.command))
        .collect();
    let current: BTreeMap<String, String> = bindings
        .iter()
        .map(|item| (item.chord.clone(), item.command.clone()))
        .collect();
    let mut keys = BTreeMap::new();
    for (chord, command) in &current {
        if default_map.get(chord) != Some(command) {
            keys.insert(chord.clone(), command.clone());
        }
    }
    for chord in default_map.keys() {
        if !current.contains_key(chord) {
            keys.insert(chord.clone(), String::new());
        }
    }
    let text = toml::to_string_pretty(&FileKeysOut { keys }).unwrap_or_default();
    std::fs::write(
        path,
        format!("# Atajos. Vacío desactiva el default. También: LoLTerm → /commands\n{text}"),
    )
}

fn load_at(path: &std::path::Path) -> Vec<Binding> {
    let mut map: BTreeMap<String, String> = defaults()
        .into_iter()
        .map(|item| (item.chord, item.command))
        .collect();
    if let Ok(text) = std::fs::read_to_string(path)
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

#[derive(serde::Serialize)]
struct FileKeysOut {
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
        assert_eq!(normalize_chord("ctrl+shift+tab"), "ctrl+shift+tab");
        assert_eq!(normalize_chord("Ctrl+Tab"), "ctrl+tab");
    }

    #[test]
    fn defaults_include_palette_and_zoom() {
        let chords: Vec<_> = defaults().into_iter().map(|b| b.command).collect();
        assert!(chords.contains(&"ui.palette".into()));
        assert!(chords.contains(&"pane.zoom".into()));
        assert!(chords.contains(&"tab.next".into()));
        assert!(chords.contains(&"tab.prev".into()));
        assert!(chords.contains(&"pane.splitRight".into()));
        assert!(chords.contains(&"pane.splitDown".into()));
        assert!(chords.contains(&"pane.restart".into()));
        assert!(chords.contains(&"ui.tabRename".into()));
        assert!(chords.contains(&"workspace.next".into()));
        assert!(chords.contains(&"ui.commands".into()));
    }

    #[test]
    fn apply_overrides_and_reset() {
        let dir = std::env::temp_dir().join(format!(
            "lolterm-keys-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("keybindings.toml");
        let bindings = apply_at(&path, "ctrl+alt+b", "ui.palette").expect("apply");
        assert!(
            bindings
                .iter()
                .any(|item| item.chord == "alt+ctrl+b" && item.command == "ui.palette")
        );
        assert_eq!(
            bindings
                .iter()
                .filter(|item| item.command == "ui.palette")
                .count(),
            1,
            "reasignar deja un solo atajo por comando"
        );
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains("ui.palette"));
        reset_at(&path).expect("reset");
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
