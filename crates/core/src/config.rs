use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Sage,
    Dusk,
    Mono,
}

impl Theme {
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "sage" => Some(Self::Sage),
            "dusk" => Some(Self::Dusk),
            "mono" => Some(Self::Mono),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sage => "sage",
            Self::Dusk => "dusk",
            Self::Mono => "mono",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub user: Option<String>,
    pub tmux: String,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            user: None,
            tmux: "lolterm".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub theme: Theme,
    pub remote: RemoteConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Sage,
            remote: RemoteConfig::default(),
        }
    }
}

pub fn load() -> AppConfig {
    let path = config_path();
    let Ok(text) = std::fs::read_to_string(path) else {
        return AppConfig::default();
    };
    let parsed: FileConfig = toml::from_str(&text).unwrap_or_default();
    AppConfig {
        theme: parsed
            .ui
            .theme
            .as_deref()
            .and_then(Theme::parse)
            .unwrap_or_default(),
        remote: RemoteConfig {
            user: parsed.remote.user.filter(|user| !user.is_empty()),
            tmux: parsed
                .remote
                .tmux
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "lolterm".into()),
        },
    }
}

pub fn load_remote() -> RemoteConfig {
    load().remote
}

pub fn save(config: &AppConfig) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = FileConfig {
        ui: FileUi {
            theme: Some(config.theme.as_str().to_string()),
        },
        remote: FileRemote {
            user: config.remote.user.clone(),
            tmux: Some(config.remote.tmux.clone()),
        },
    };
    let text = toml::to_string_pretty(&file).unwrap_or_default();
    std::fs::write(path, text)
}

pub fn config_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("lolterm")
}

pub fn config_path() -> std::path::PathBuf {
    config_dir().join("config.toml")
}

#[derive(Default, Serialize, Deserialize)]
struct FileConfig {
    #[serde(default)]
    ui: FileUi,
    #[serde(default)]
    remote: FileRemote,
}

#[derive(Default, Serialize, Deserialize)]
struct FileUi {
    #[serde(default)]
    theme: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct FileRemote {
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    tmux: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_themes() {
        assert_eq!(Theme::parse("Sage"), Some(Theme::Sage));
        assert_eq!(Theme::parse("dusk"), Some(Theme::Dusk));
        assert_eq!(Theme::parse("MONO"), Some(Theme::Mono));
        assert_eq!(Theme::parse("solarized"), None);
    }
}
