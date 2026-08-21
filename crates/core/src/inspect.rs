//! Disco, batería, puertos y árbol de procesos de los PTYs del mux.
//! Solo Linux (`/proc`, `statvfs`). Si falta, el HUD omite el campo.

use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

static EXTRA: Mutex<Option<(Instant, HostExtra)>> = Mutex::new(None);

#[derive(Clone, Debug, Default, Serialize)]
pub struct HostExtra {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<PortRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub processes: Vec<ProcRow>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortRow {
    pub port: u16,
    pub pid: u32,
    pub pane: u64,
    pub program: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProcRow {
    pub pid: u32,
    pub program: String,
    pub pane: u64,
}

pub fn extra(root: &Path, pane_pids: &[(u64, u32)]) -> HostExtra {
    if let Ok(guard) = EXTRA.lock()
        && let Some((at, cached)) = guard.as_ref()
        && at.elapsed() < Duration::from_secs(4)
    {
        return cached.clone();
    }
    let extra = HostExtra {
        disk: disk_used_pct(root),
        battery: battery_pct(),
        ports: listening_ports(pane_pids),
        processes: proc_rows(pane_pids),
    };
    if let Ok(mut slot) = EXTRA.lock() {
        *slot = Some((Instant::now(), extra.clone()));
    }
    extra
}

fn disk_used_pct(root: &Path) -> Option<u8> {
    #[cfg(unix)]
    {
        disk_used_pct_unix(root)
    }
    #[cfg(not(unix))]
    {
        let _ = root;
        None
    }
}

#[cfg(unix)]
fn disk_used_pct_unix(root: &Path) -> Option<u8> {
    let path = std::ffi::CString::new(root.to_str()?).ok()?;
    unsafe {
        let mut vfs: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(path.as_ptr(), &mut vfs) != 0 {
            return None;
        }
        let blocks = vfs.f_blocks as f64;
        if blocks <= 0.0 {
            return None;
        }
        let avail = vfs.f_bavail as f64;
        let used = 100.0 * (blocks - avail) / blocks;
        Some(used.round().clamp(0.0, 100.0) as u8)
    }
}

fn battery_pct() -> Option<u8> {
    for name in ["BAT0", "BAT1"] {
        let path = format!("/sys/class/power_supply/{name}/capacity");
        if let Ok(text) = fs::read_to_string(path)
            && let Ok(pct) = text.trim().parse::<u8>()
        {
            return Some(pct.min(100));
        }
    }
    None
}

fn proc_rows(pane_pids: &[(u64, u32)]) -> Vec<ProcRow> {
    let mut out = Vec::new();
    for (pane, pid) in pane_pids {
        out.push(ProcRow {
            pid: *pid,
            program: comm(*pid).unwrap_or_else(|| "?".into()),
            pane: *pane,
        });
        for child in descendants(*pid, 3) {
            if out.len() >= 40 {
                break;
            }
            if out.iter().any(|row| row.pid == child) {
                continue;
            }
            out.push(ProcRow {
                pid: child,
                program: comm(child).unwrap_or_else(|| "?".into()),
                pane: *pane,
            });
        }
    }
    out
}

fn comm(pid: u32) -> Option<String> {
    let text = fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let name = text.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn children(pid: u32) -> Vec<u32> {
    fs::read_to_string(format!("/proc/{pid}/task/{pid}/children"))
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|item| item.parse().ok())
        .collect()
}

fn descendants(pid: u32, depth: u8) -> Vec<u32> {
    let mut out = Vec::new();
    walk_descendants(pid, depth, &mut out);
    out
}

fn walk_descendants(pid: u32, depth: u8, out: &mut Vec<u32>) {
    if depth == 0 || out.len() >= 48 {
        return;
    }
    for child in children(pid) {
        if out.contains(&child) {
            continue;
        }
        out.push(child);
        walk_descendants(child, depth - 1, out);
    }
}

fn listening_ports(pane_pids: &[(u64, u32)]) -> Vec<PortRow> {
    let mut owned: Vec<(u32, u64)> = Vec::new();
    for (pane, pid) in pane_pids {
        owned.push((*pid, *pane));
        for child in descendants(*pid, 3) {
            owned.push((child, *pane));
        }
    }
    if owned.is_empty() {
        return Vec::new();
    }
    let mut inodes = listen_inodes("/proc/net/tcp");
    inodes.extend(listen_inodes("/proc/net/tcp6"));
    let mut out = Vec::new();
    for (pid, pane) in &owned {
        for (port, inode) in &inodes {
            if fd_has_inode(*pid, *inode) {
                out.push(PortRow {
                    port: *port,
                    pid: *pid,
                    pane: *pane,
                    program: comm(*pid).unwrap_or_else(|| "?".into()),
                });
            }
        }
        if out.len() >= 24 {
            break;
        }
    }
    out.sort_by_key(|row| row.port);
    out.dedup_by_key(|row| row.port);
    out
}

fn listen_inodes(path: &str) -> Vec<(u16, u64)> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 10 {
            continue;
        }
        // st 0A = LISTEN
        if cols[3] != "0A" {
            continue;
        }
        let Some((_, port_hex)) = cols[1].split_once(':') else {
            continue;
        };
        let Ok(port) = u16::from_str_radix(port_hex, 16) else {
            continue;
        };
        let Ok(inode) = cols[9].parse::<u64>() else {
            continue;
        };
        out.push((port, inode));
    }
    out
}

fn fd_has_inode(pid: u32, inode: u64) -> bool {
    let dir = format!("/proc/{pid}/fd");
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    let needle = format!("socket:[{inode}]");
    for entry in entries.flatten() {
        if let Ok(link) = fs::read_link(entry.path())
            && link.to_string_lossy() == needle
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_of_tmp_is_a_percent() {
        let pct = disk_used_pct(Path::new("/tmp"));
        if let Some(pct) = pct {
            assert!(pct <= 100);
        }
    }
}
