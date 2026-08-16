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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MachineKind {
    #[default]
    Ssh,
    Tailscale,
}

impl MachineKind {
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "ssh" => Some(Self::Ssh),
            "ts" | "tailscale" => Some(Self::Tailscale),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::Tailscale => "tailscale",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Machine {
    pub name: String,
    pub target: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub kind: MachineKind,
}

pub const MACHINE_CAP: usize = 12;

impl Machine {
    pub fn dest(&self, fallback_user: Option<&str>) -> String {
        let user = self
            .user
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .or(fallback_user);
        match self.kind {
            MachineKind::Tailscale => crate::ssh::ts_ssh_dest(&self.target, user),
            MachineKind::Ssh => {
                if self.target.contains('@') {
                    self.target.clone()
                } else {
                    match user {
                        Some(user) => format!("{user}@{}", self.target),
                        None => self.target.clone(),
                    }
                }
            }
        }
    }
}

pub fn host_label(target: &str) -> String {
    let host = target
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(target)
        .trim()
        .trim_end_matches('.');
    host.split('.')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(host)
        .to_string()
}

pub fn upsert_machine(list: &mut Vec<Machine>, machine: Machine) {
    list.retain(|item| item.target != machine.target);
    list.insert(0, machine);
    list.truncate(MACHINE_CAP);
}

pub fn machine_target_ok(target: &str) -> bool {
    let target = target.trim();
    !target.is_empty()
        && target
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '@'))
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub theme: Theme,
    pub remote: RemoteConfig,
    pub machines: Vec<Machine>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Sage,
            remote: RemoteConfig::default(),
            machines: Vec::new(),
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
            tmux: match parsed.remote.tmux {
                None => "lolterm".into(),
                Some(name) if name.trim().is_empty() => String::new(),
                Some(name) => name,
            },
        },
        machines: parsed
            .machines
            .into_iter()
            .filter_map(|item| {
                let name = item.name.trim().to_string();
                let target = item.target.trim().to_string();
                if name.is_empty() || !machine_target_ok(&target) {
                    return None;
                }
                let user = item
                    .user
                    .map(|value| value.trim().to_string())
                    .filter(|value| crate::ssh::ssh_user_ok(value));
                Some(Machine {
                    name,
                    target,
                    user,
                    kind: MachineKind::parse(item.kind.as_deref().unwrap_or("ssh"))
                        .unwrap_or_default(),
                })
            })
            .collect(),
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
        machines: config
            .machines
            .iter()
            .map(|item| FileMachine {
                name: item.name.clone(),
                target: item.target.clone(),
                user: item.user.clone(),
                kind: Some(item.kind.as_str().to_string()),
            })
            .collect(),
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
    #[serde(default)]
    machines: Vec<FileMachine>,
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

#[derive(Default, Serialize, Deserialize)]
struct FileMachine {
    #[serde(default)]
    name: String,
    #[serde(default)]
    target: String,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    kind: Option<String>,
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

    #[test]
    fn machine_target_rejects_spaces_and_urls() {
        assert!(machine_target_ok("chae.tailnet.ts.net"));
        assert!(machine_target_ok("me@pi"));
        assert!(!machine_target_ok(""));
        assert!(!machine_target_ok("host name"));
        assert!(!machine_target_ok("https://evil"));
    }

    #[test]
    fn machine_dest_uses_kind() {
        let ts = Machine {
            name: "box".into(),
            target: "box.tailnet.ts.net".into(),
            user: None,
            kind: MachineKind::Tailscale,
        };
        assert_eq!(ts.dest(Some("me")), "me@box.tailnet.ts.net");
        let ssh = Machine {
            name: "pi".into(),
            target: "pi".into(),
            user: Some("root".into()),
            kind: MachineKind::Ssh,
        };
        assert_eq!(ssh.dest(None), "root@pi");
    }

    #[test]
    fn machines_toml_roundtrip_skips_passwords() {
        let parsed: FileConfig = toml::from_str(
            "[[machines]]\nname = \"chae\"\ntarget = \"chae.ts.net\"\nuser = \"lauta\"\nkind = \"tailscale\"\n",
        )
        .expect("toml");
        assert_eq!(parsed.machines[0].name, "chae");
        assert_eq!(parsed.machines[0].kind.as_deref(), Some("tailscale"));
    }

    #[test]
    fn host_label_uses_first_dns_label() {
        assert_eq!(host_label("chae.tailnet.ts.net"), "chae");
        assert_eq!(host_label("lauta@pi"), "pi");
        assert_eq!(host_label("pi"), "pi");
    }

    #[test]
    fn upsert_machine_moves_existing_to_front() {
        let mut list = vec![Machine {
            name: "old".into(),
            target: "a".into(),
            user: None,
            kind: MachineKind::Ssh,
        }];
        upsert_machine(
            &mut list,
            Machine {
                name: "new".into(),
                target: "a".into(),
                user: Some("me".into()),
                kind: MachineKind::Ssh,
            },
        );
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "new");
        assert_eq!(list[0].user.as_deref(), Some("me"));
    }
}
