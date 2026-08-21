use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Claro,
    Oscuro,
    Contraste,
    Tide,
    Ember,
}

impl Theme {
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "claro" => Some(Self::Claro),
            "oscuro" => Some(Self::Oscuro),
            "contraste" => Some(Self::Contraste),
            "tide" => Some(Self::Tide),
            "ember" => Some(Self::Ember),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claro => "claro",
            Self::Oscuro => "oscuro",
            Self::Contraste => "contraste",
            Self::Tide => "tide",
            Self::Ember => "ember",
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
    pub theme: String,
    pub remote: RemoteConfig,
    pub machines: Vec<Machine>,
    /// Qué abre `tab.new` / Ctrl-Alt-N: `shell`, `ssh`, `tailscale` o una CLI del catálogo.
    pub new_tab: String,
    /// Si un agente abre en `git worktree` (no pisa el working tree de nvim).
    pub agent_worktrees: bool,
    /// `'autowrite'` de nvim al abrir un archivo (default off, como nvim).
    pub editor_autowrite: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "claro".into(),
            remote: RemoteConfig::default(),
            machines: Vec::new(),
            new_tab: "shell".into(),
            agent_worktrees: true,
            editor_autowrite: false,
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
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("claro")
            .to_ascii_lowercase(),
        remote: RemoteConfig {
            user: parsed.remote.user.filter(|user| !user.is_empty()),
            tmux: match parsed.remote.tmux {
                None => "lolterm".into(),
                Some(name) if name.trim().is_empty() => String::new(),
                Some(name) => name,
            },
        },
        new_tab: parsed.ui.new_tab.unwrap_or_else(|| "shell".into()),
        agent_worktrees: parsed.ai.worktrees.unwrap_or(true),
        editor_autowrite: parsed.editor.autowrite.unwrap_or(false),
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
            theme: Some(config.theme.clone()),
            new_tab: Some(config.new_tab.clone()),
        },
        ai: FileAi {
            worktrees: Some(config.agent_worktrees),
        },
        editor: FileEditor {
            autowrite: Some(config.editor_autowrite),
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

/// Estado efímero de esta máquina (sockets). No va en dotfiles sincronizados.
pub fn runtime_dir() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return std::path::PathBuf::from(dir).join("lolterm");
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    std::env::temp_dir().join(format!("lolterm-{user}"))
}

/// Datos de esta máquina (worktrees, historial). No es config portable.
pub fn data_dir() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return std::path::PathBuf::from(dir).join("lolterm");
    }
    std::env::var_os("HOME")
        .map(|home| std::path::PathBuf::from(home).join(".local/share/lolterm"))
        .unwrap_or_else(|| std::path::PathBuf::from(".").join("lolterm-data"))
}

pub fn config_path() -> std::path::PathBuf {
    config_dir().join("config.toml")
}

/// Pedido de la CLI para que el Desktop haga algo al arrancar o al recibir
/// una segunda instancia. No guarda secretos ni argv arbitrario.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingLaunch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
}

pub fn pending_path() -> std::path::PathBuf {
    config_dir().join("pending.toml")
}

pub fn write_pending(pending: &PendingLaunch) -> Result<(), String> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|err| format!("no pude crear {dir:?}: {err}"))?;
    let text =
        toml::to_string(pending).map_err(|err| format!("no pude serializar pending: {err}"))?;
    std::fs::write(pending_path(), text).map_err(|err| format!("no pude escribir pending: {err}"))
}

pub fn take_pending() -> Option<PendingLaunch> {
    let path = pending_path();
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    toml::from_str(&text).ok()
}

#[derive(Default, Serialize, Deserialize)]
struct FileConfig {
    #[serde(default)]
    ui: FileUi,
    #[serde(default)]
    ai: FileAi,
    #[serde(default)]
    editor: FileEditor,
    #[serde(default)]
    remote: FileRemote,
    #[serde(default)]
    machines: Vec<FileMachine>,
}

#[derive(Default, Serialize, Deserialize)]
struct FileEditor {
    #[serde(default)]
    autowrite: Option<bool>,
}

#[derive(Default, Serialize, Deserialize)]
struct FileUi {
    #[serde(default)]
    theme: Option<String>,
    #[serde(default)]
    new_tab: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct FileAi {
    #[serde(default)]
    worktrees: Option<bool>,
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
        assert_eq!(Theme::parse("Claro"), Some(Theme::Claro));
        assert_eq!(Theme::parse("contraste"), Some(Theme::Contraste));
        assert_eq!(Theme::parse("TIDE"), Some(Theme::Tide));
        assert_eq!(Theme::parse("solarized"), None);
        assert_eq!(Theme::Claro.as_str(), "claro");
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
    fn pending_toml_roundtrip() {
        let text = toml::to_string(&PendingLaunch {
            ssh: Some("chae".into()),
            open: Some("/home/me/dev/lolterm".into()),
            run: Some("nvim".into()),
        })
        .expect("toml");
        let parsed: PendingLaunch = toml::from_str(&text).expect("parse");
        assert_eq!(parsed.ssh.as_deref(), Some("chae"));
        assert_eq!(parsed.open.as_deref(), Some("/home/me/dev/lolterm"));
        assert_eq!(parsed.run.as_deref(), Some("nvim"));
    }

    #[test]
    fn editor_autowrite_defaults_off() {
        let parsed: FileConfig = toml::from_str("").expect("empty");
        assert_eq!(parsed.editor.autowrite, None);
        let parsed: FileConfig = toml::from_str("[editor]\nautowrite = true\n").expect("editor");
        assert_eq!(parsed.editor.autowrite, Some(true));
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
