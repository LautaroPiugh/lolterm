use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub fn ts_ssh_dest(target: &str, default_user: Option<&str>) -> String {
    let host = target.trim().trim_end_matches('.');
    if host.is_empty() {
        return String::new();
    }
    if let Some((user, name)) = host.split_once('@') {
        let name = name.trim_end_matches('.');
        if user.is_empty() {
            return name.to_string();
        }
        return format!("{user}@{name}");
    }
    match default_user.map(str::trim).filter(|user| !user.is_empty()) {
        Some(user) => format!("{user}@{host}"),
        None => host.to_string(),
    }
}

pub fn ssh_args(dest: &str, tmux_session: &str) -> Vec<String> {
    let mut args = vec![
        "-tt".into(),
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        "ServerAliveInterval=30".into(),
        "-o".into(),
        "ServerAliveCountMax=4".into(),
        dest.to_string(),
    ];
    if !tmux_session.trim().is_empty() {
        args.push("sh".into());
        args.push("-lc".into());
        args.push(remote_persist_script(tmux_session));
    }
    args
}

pub fn ts_ssh_args(target: &str, default_user: Option<&str>, tmux_session: &str) -> Vec<String> {
    ssh_args(&ts_ssh_dest(target, default_user), tmux_session)
}

pub fn tmux_session_name(prefix: &str, workspace: &str) -> String {
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return String::new();
    }
    let prefix = sanitize_tmux_session(prefix);
    let slug = crate::workspaces::slug(workspace);
    format!("{prefix}-{slug}")
}

pub fn ssh_dest_from_args(args: &[String]) -> Option<String> {
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "-o" || arg == "-l" || arg == "-p" || arg == "-F" || arg == "-i" {
            skip_next = true;
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        if arg == "sh" {
            break;
        }
        let dest = arg.trim();
        if dest.is_empty() {
            continue;
        }
        return Some(dest.to_string());
    }
    None
}

fn remote_persist_script(session: &str) -> String {
    let session = sanitize_tmux_session(session);
    format!(
        "command -v tmux >/dev/null 2>&1 && exec tmux new-session -A -s {session} || exec \"${{SHELL:-/bin/sh}}\" -l"
    )
}

pub fn ssh_user_ok(user: &str) -> bool {
    let user = user.trim();
    !user.is_empty()
        && user
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
}

fn sanitize_tmux_session(name: &str) -> &str {
    let name = name.trim();
    if !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        name
    } else {
        "lolterm"
    }
}

pub fn parse_ssh_hosts(text: &str) -> Vec<String> {
    parse_ssh_hosts_from(text, &ssh_dir(), 0, &mut HashSet::new())
}

pub fn ssh_config_hosts() -> Vec<String> {
    let path = ssh_dir().join("config");
    std::fs::read_to_string(&path)
        .map(|text| {
            parse_ssh_hosts_from(
                &text,
                path.parent().unwrap_or(Path::new(".")),
                0,
                &mut HashSet::new(),
            )
        })
        .unwrap_or_default()
}

fn ssh_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ssh")
}

fn parse_ssh_hosts_from(
    text: &str,
    base: &Path,
    depth: usize,
    seen: &mut HashSet<PathBuf>,
) -> Vec<String> {
    let mut hosts = Vec::new();
    if depth > 4 {
        return hosts;
    }
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("include ") {
            for path in include_paths(base, trimmed[8..].trim()) {
                let Ok(canon) = path.canonicalize() else {
                    continue;
                };
                if !seen.insert(canon.clone()) {
                    continue;
                }
                if let Ok(nested) = std::fs::read_to_string(&canon) {
                    let nested_base = canon.parent().unwrap_or(base);
                    for host in parse_ssh_hosts_from(&nested, nested_base, depth + 1, seen) {
                        if !hosts.iter().any(|existing| existing == &host) {
                            hosts.push(host);
                        }
                    }
                }
            }
            continue;
        }
        if !lower.starts_with("host ") {
            continue;
        }
        for name in trimmed[5..].split_whitespace() {
            if name.contains('*') || name.contains('?') || name.starts_with('!') {
                continue;
            }
            if !hosts.iter().any(|existing| existing == name) {
                hosts.push(name.to_string());
            }
        }
    }
    hosts
}

fn include_paths(base: &Path, raw: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for token in raw.split_whitespace() {
        let expanded = expand_ssh_path(base, token);
        if let Some(parent) = expanded.parent()
            && let Some(name) = expanded.file_name().and_then(|name| name.to_str())
            && name.contains('*')
        {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        out.push(path);
                    }
                }
            }
            continue;
        }
        out.push(expanded);
    }
    out
}

fn expand_ssh_path(base: &Path, raw: &str) -> PathBuf {
    let raw = raw.trim().trim_matches('"');
    if let Some(rest) = raw.strip_prefix("~/") {
        return ssh_dir().parent().unwrap_or(Path::new(".")).join(rest);
    }
    if raw.starts_with('/') {
        return PathBuf::from(raw);
    }
    base.join(raw)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostItem {
    pub name: String,
    pub target: String,
    pub hint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_config_skips_wildcards() {
        let hosts = parse_ssh_hosts(
            "Host pi casa\n  User me\nHost *\n  StrictHostKeyChecking no\nHost work\n",
        );
        assert_eq!(hosts, vec!["pi", "casa", "work"]);
    }

    #[test]
    fn ts_ssh_dest_does_not_invent_user() {
        assert_eq!(
            ts_ssh_dest("box.tailnet.ts.net", None),
            "box.tailnet.ts.net"
        );
        assert_eq!(ts_ssh_dest("me@box", None), "me@box");
        assert_eq!(ts_ssh_dest("box", Some("me")), "me@box");
    }

    #[test]
    fn empty_user_is_not_ok() {
        assert!(!ssh_user_ok(""));
        assert!(ssh_user_ok("chae"));
    }

    #[test]
    fn ssh_args_attach_tmux_by_default() {
        let args = ssh_args("me@pi", "lolterm");
        assert_eq!(args[0], "-tt");
        assert_eq!(ssh_dest_from_args(&args).as_deref(), Some("me@pi"));
        assert!(args.iter().any(|arg| arg == "ServerAliveInterval=30"));
        assert!(
            args.last()
                .unwrap()
                .contains("tmux new-session -A -s lolterm")
        );
    }

    #[test]
    fn ssh_args_skip_tmux_when_session_empty() {
        let args = ssh_args("pi", "");
        assert_eq!(ssh_dest_from_args(&args).as_deref(), Some("pi"));
        assert!(!args.iter().any(|arg| arg == "sh"));
        assert!(args.iter().any(|arg| arg == "ServerAliveInterval=30"));
    }

    #[test]
    fn ssh_config_follows_include() {
        let dir = std::env::temp_dir().join(format!(
            "lolterm-ssh-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let extra = dir.join("extra");
        let _ = std::fs::create_dir_all(&extra);
        std::fs::write(extra.join("box"), "Host included\n").expect("write");
        let text = "Host root\nInclude extra/*\nHost after\n";
        let hosts = parse_ssh_hosts_from(text, &dir, 0, &mut HashSet::new());
        assert_eq!(hosts, vec!["root", "included", "after"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ssh_dest_from_args_skips_flags() {
        let args = ssh_args("me@chae.tailnet.ts.net", "lolterm");
        assert_eq!(
            ssh_dest_from_args(&args).as_deref(),
            Some("me@chae.tailnet.ts.net")
        );
        assert_eq!(
            ssh_dest_from_args(&["-tt".into(), "pi".into()]).as_deref(),
            Some("pi")
        );
        assert_eq!(ssh_dest_from_args(&["-tt".into(), "sh".into()]), None);
    }

    #[test]
    fn tmux_session_name_joins_prefix_and_workspace() {
        assert_eq!(tmux_session_name("lolterm", "api"), "lolterm-api");
        assert_eq!(tmux_session_name("lolterm", "LoLTerm"), "lolterm-lolterm");
        assert_eq!(tmux_session_name("", "api"), "");
        assert_eq!(tmux_session_name("  ", "api"), "");
    }
}
