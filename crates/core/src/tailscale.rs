use std::collections::HashMap;
use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Status {
    Missing,
    Stopped,
    NeedsLogin,
    Running {
        host: String,
        ip: Option<String>,
        online_peers: u32,
    },
}

impl Status {
    pub fn chip(&self) -> Option<(String, bool)> {
        match self {
            Self::Missing => None,
            Self::Stopped => Some((" ts:off ".into(), false)),
            Self::NeedsLogin => Some((" ts:login ".into(), false)),
            Self::Running {
                host, online_peers, ..
            } => {
                let extra = if *online_peers > 0 {
                    format!("·{online_peers}")
                } else {
                    String::new()
                };
                Some((format!(" ts:{host}{extra} "), true))
            }
        }
    }

    pub fn notice(&self) -> String {
        match self {
            Self::Missing => " tailscale no está en PATH ".into(),
            Self::Stopped => " tailscale está detenido · /tailscale up ".into(),
            Self::NeedsLogin => " tailscale necesita login · /tailscale login ".into(),
            Self::Running {
                host,
                ip,
                online_peers,
            } => {
                let addr = ip.clone().unwrap_or_default();
                format!(" tailscale {host} {addr} · {online_peers} online · /ts-ssh host ")
            }
        }
    }
}

#[derive(Deserialize)]
struct StatusJson {
    #[serde(rename = "BackendState")]
    backend_state: String,
    #[serde(rename = "Self")]
    self_node: Option<NodeJson>,
    #[serde(rename = "Peer")]
    peer: Option<HashMap<String, NodeJson>>,
    #[serde(rename = "TailscaleIPs")]
    tailscale_ips: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Peer {
    pub name: String,
    pub target: String,
    pub online: bool,
    pub ip: Option<String>,
}

#[derive(Deserialize)]
struct NodeJson {
    #[serde(rename = "HostName")]
    host_name: Option<String>,
    #[serde(rename = "DNSName")]
    dns_name: Option<String>,
    #[serde(rename = "Online")]
    online: Option<bool>,
    #[serde(rename = "TailscaleIPs")]
    tailscale_ips: Option<Vec<String>>,
}

pub fn probe() -> Status {
    match load_status_json() {
        None => {
            if command_on_path("tailscale") {
                Status::Stopped
            } else {
                Status::Missing
            }
        }
        Some(parsed) => parse_status(parsed),
    }
}

pub fn peers() -> Vec<Peer> {
    let Some(parsed) = load_status_json() else {
        return Vec::new();
    };
    list_peers(parsed)
}

fn load_status_json() -> Option<StatusJson> {
    if !command_on_path("tailscale") {
        return None;
    }
    let output = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn list_peers(parsed: StatusJson) -> Vec<Peer> {
    let mut peers: Vec<Peer> = parsed
        .peer
        .unwrap_or_default()
        .into_values()
        .filter_map(peer_from_node)
        .collect();
    peers.sort_by(|a, b| b.online.cmp(&a.online).then(a.name.cmp(&b.name)));
    peers
}

fn peer_from_node(node: NodeJson) -> Option<Peer> {
    let ip = node.tailscale_ips.and_then(|ips| ips.into_iter().next());
    let dns = node
        .dns_name
        .as_deref()
        .map(clean_dns)
        .filter(|name| !name.is_empty());
    let host = node
        .host_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string);
    let target = dns.or(host.clone()).or(ip.clone())?;
    Some(Peer {
        name: host.unwrap_or_else(|| target.clone()),
        target,
        online: node.online.unwrap_or(false),
        ip,
    })
}

fn clean_dns(name: &str) -> String {
    name.trim().trim_end_matches('.').to_string()
}

fn parse_status(parsed: StatusJson) -> Status {
    match parsed.backend_state.as_str() {
        "Running" => {
            let host = parsed
                .self_node
                .as_ref()
                .and_then(|node| node.host_name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "online".into());
            let ip = parsed
                .tailscale_ips
                .as_ref()
                .and_then(|ips| ips.first().cloned());
            let online_peers = parsed
                .peer
                .unwrap_or_default()
                .values()
                .filter(|node| node.online.unwrap_or(false))
                .count() as u32;
            Status::Running {
                host,
                ip,
                online_peers,
            }
        }
        "NeedsLogin" | "NeedsMachineAuth" => Status::NeedsLogin,
        _ => Status::Stopped,
    }
}

fn command_on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_json_has_host_and_peers() {
        let json = r#"{
            "BackendState": "Running",
            "TailscaleIPs": ["100.1.2.3"],
            "Self": { "HostName": "casa", "Online": true },
            "Peer": {
                "n1": { "HostName": "pi", "Online": true },
                "n2": { "HostName": "off", "Online": false }
            }
        }"#;
        let parsed: StatusJson = serde_json::from_str(json).unwrap();
        assert_eq!(
            parse_status(parsed),
            Status::Running {
                host: "casa".into(),
                ip: Some("100.1.2.3".into()),
                online_peers: 1,
            }
        );
    }

    #[test]
    fn stopped_json() {
        let parsed: StatusJson = serde_json::from_str(r#"{"BackendState":"Stopped"}"#).unwrap();
        assert_eq!(parse_status(parsed), Status::Stopped);
    }

    #[test]
    fn peers_online_first() {
        let parsed: StatusJson = serde_json::from_str(
            r#"{
            "BackendState": "Running",
            "Peer": {
                "n1": { "HostName": "zeta", "Online": false },
                "n2": { "HostName": "alfa", "DNSName": "alfa.tailnet.ts.net.", "Online": true, "TailscaleIPs": ["100.1.1.1"] }
            }
        }"#,
        )
        .unwrap();
        let peers = list_peers(parsed);
        assert_eq!(peers[0].name, "alfa");
        assert_eq!(peers[0].target, "alfa.tailnet.ts.net");
        assert!(peers[0].online);
        assert_eq!(peers[1].name, "zeta");
    }

    #[test]
    fn peer_ssh_target_prefers_magicdns() {
        let node = NodeJson {
            host_name: Some("pi".into()),
            dns_name: Some("pi.tailnet.ts.net.".into()),
            online: Some(true),
            tailscale_ips: Some(vec!["100.1.1.1".into()]),
        };
        let peer = peer_from_node(node).unwrap();
        assert_eq!(peer.name, "pi");
        assert_eq!(peer.target, "pi.tailnet.ts.net");
    }
}
