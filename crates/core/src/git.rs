use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub branch: String,
    pub detached: bool,
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    pub ahead: u32,
    pub behind: u32,
}

impl Status {
    pub fn dirty(&self) -> bool {
        self.staged > 0 || self.unstaged > 0 || self.untracked > 0
    }

    pub fn badge(&self) -> String {
        if self.dirty() {
            format!("{}*", self.branch)
        } else {
            self.branch.clone()
        }
    }

    pub fn chips(&self) -> Vec<(String, Tone)> {
        let mut chips = vec![(format!(" git:{}", self.branch), Tone::Branch)];
        if self.staged > 0 {
            chips.push((format!(" +{}", self.staged), Tone::Plus));
        }
        if self.unstaged > 0 {
            chips.push((format!(" -{}", self.unstaged), Tone::Minus));
        }
        if self.untracked > 0 {
            chips.push((format!(" ?{}", self.untracked), Tone::Minus));
        }
        if self.ahead > 0 {
            chips.push((format!(" ↑{}", self.ahead), Tone::Plus));
        }
        if self.behind > 0 {
            chips.push((format!(" ↓{}", self.behind), Tone::Minus));
        }
        if !self.dirty() && self.ahead == 0 && self.behind == 0 {
            chips.push((" ✓".to_string(), Tone::Plus));
        }
        chips
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tone {
    Branch,
    Plus,
    Minus,
}

pub fn toplevel(dir: &Path) -> Option<PathBuf> {
    let stdout = git_output(dir, &["rev-parse", "--show-toplevel"])?;
    let root = PathBuf::from(stdout);
    root.is_dir().then_some(root)
}

pub fn status(dir: &Path) -> Option<Status> {
    if !dir.exists() {
        return None;
    }
    let abbrev = git_output(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let detached = abbrev == "HEAD";
    let branch = if detached {
        git_output(dir, &["rev-parse", "--short", "HEAD"])
            .map(|sha| format!("detached@{sha}"))
            .unwrap_or(abbrev)
    } else {
        abbrev
    };
    let porcelain = git_output(dir, &["status", "--porcelain"]).unwrap_or_default();
    let (staged, unstaged, untracked) = count_porcelain(&porcelain);
    let (ahead, behind) = ahead_behind(dir);
    Some(Status {
        branch,
        detached,
        staged,
        unstaged,
        untracked,
        ahead,
        behind,
    })
}

pub fn branch_label(dir: &Path) -> Option<String> {
    status(dir).map(|status| status.badge())
}

pub fn oneline(dir: &Path, limit: usize) -> Vec<String> {
    let stdout = git_output(dir, &["log", "--oneline", &format!("-{limit}")]).unwrap_or_default();
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn path_marks(dir: &Path) -> HashMap<String, char> {
    let porcelain = git_output(dir, &["status", "--porcelain"]).unwrap_or_default();
    marks_from_porcelain(&porcelain)
}

pub fn marks_from_porcelain(text: &str) -> HashMap<String, char> {
    let mut marks = HashMap::new();
    for line in text.lines() {
        let bytes = line.as_bytes();
        if bytes.len() < 4 {
            continue;
        }
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        let path = line[3..].trim();
        let path = path
            .rsplit_once(" -> ")
            .map(|(_, next)| next)
            .unwrap_or(path)
            .trim_matches('"');
        if path.is_empty() {
            continue;
        }
        let mark = if x == '?' && y == '?' {
            '?'
        } else if x == 'A' || y == 'A' {
            'A'
        } else if x == 'D' || y == 'D' {
            'D'
        } else {
            'M'
        };
        marks.insert(path.replace('\\', "/"), mark);
    }
    marks
}

pub fn count_porcelain(text: &str) -> (u32, u32, u32) {
    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked = 0;
    for line in text.lines() {
        let bytes = line.as_bytes();
        if bytes.len() < 2 {
            continue;
        }
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        if x == '?' && y == '?' {
            untracked += 1;
            continue;
        }
        if x == '!' {
            continue;
        }
        if x != ' ' {
            staged += 1;
        }
        if y != ' ' {
            unstaged += 1;
        }
    }
    (staged, unstaged, untracked)
}

fn ahead_behind(dir: &Path) -> (u32, u32) {
    let output = git_output(
        dir,
        &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
    );
    let Some(output) = output else {
        return (0, 0);
    };
    let mut parts = output.split_whitespace();
    let ahead = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
    let behind = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
    (ahead, behind)
}

fn git_output(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", &dir.to_string_lossy()])
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() && args.first().is_some_and(|arg| *arg != "status") {
        return None;
    }
    Some(trimmed.to_string())
}

/// Nuevo worktree en `path` con rama `branch` desde HEAD. No borra worktrees.
pub fn worktree_add(repo: &Path, path: &Path, branch: &str) -> Result<(), String> {
    let branch = branch.trim();
    if branch.is_empty() || path.as_os_str().is_empty() {
        return Err("worktree inválido".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let output = Command::new("git")
        .args(["-C", &repo.to_string_lossy()])
        .args([
            "worktree",
            "add",
            "-b",
            branch,
            &path.to_string_lossy(),
            "HEAD",
        ])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&output.stderr);
    Err(err.trim().to_string())
}

#[derive(Clone, Debug, Serialize)]
pub struct WorkingFile {
    pub path: String,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub mark: String,
}

pub fn working_files(dir: &Path) -> Vec<WorkingFile> {
    let porcelain = git_output(dir, &["status", "--porcelain"]).unwrap_or_default();
    let mut out = Vec::new();
    for line in porcelain.lines() {
        let bytes = line.as_bytes();
        if bytes.len() < 4 {
            continue;
        }
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        let path = line[3..].trim();
        let path = path
            .rsplit_once(" -> ")
            .map(|(_, next)| next)
            .unwrap_or(path)
            .trim_matches('"')
            .replace('\\', "/");
        if path.is_empty() {
            continue;
        }
        let untracked = x == '?' && y == '?';
        out.push(WorkingFile {
            path,
            staged: !untracked && x != ' ' && x != '!',
            unstaged: !untracked && y != ' ' && y != '!',
            untracked,
            mark: format!("{x}{y}"),
        });
    }
    out
}

pub fn branches(dir: &Path) -> Vec<String> {
    let stdout = git_output(dir, &["branch", "--format=%(refname:short)"]).unwrap_or_default();
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Operaciones de trabajo. Nunca `--force` ni push con lease saltado.
pub fn run_op(
    dir: &Path,
    op: &str,
    path: Option<&str>,
    message: Option<&str>,
) -> Result<(), String> {
    let op = op.trim();
    match op {
        "init" => git_ok(dir, &["init"])?,
        "stage" => {
            if optional_path(path)?.is_none() {
                git_ok(dir, &["add", "-A"])?;
            } else {
                let path = path_arg(path)?;
                git_ok(dir, &["add", "--", path])?;
            }
        }
        "unstage" => {
            if optional_path(path)?.is_none() {
                git_ok(dir, &["restore", "--staged", "."])?;
            } else {
                let path = path_arg(path)?;
                git_ok(dir, &["restore", "--staged", "--", path])?;
            }
        }
        "discard" => {
            let path = path_arg(path)?;
            git_ok(dir, &["checkout", "--", path])?;
        }
        "commit" => {
            let message = message
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .ok_or("hace falta un mensaje")?;
            git_ok(dir, &["commit", "-m", message])?;
        }
        "fetch" => git_ok(dir, &["fetch", "--all", "--prune"])?,
        "pull" => git_ok(dir, &["pull", "--ff-only"])?,
        "checkout" => {
            let branch = path_arg(path)?;
            if !branch
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '/'))
            {
                return Err("rama inválida".into());
            }
            git_ok(dir, &["checkout", branch])?;
        }
        other => return Err(format!("git op desconocida: {other}")),
    }
    Ok(())
}

fn optional_path(path: Option<&str>) -> Result<Option<&str>, String> {
    match path.map(str::trim).filter(|p| !p.is_empty()) {
        None => Ok(None),
        Some(path) => path_arg(Some(path)).map(Some),
    }
}

fn path_arg(path: Option<&str>) -> Result<&str, String> {
    let path = path
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or("hace falta un path")?;
    if path.starts_with('-') || path.contains('\0') || path.split('/').any(|p| p == "..") {
        return Err("path git inválido".into());
    }
    Ok(path)
}

fn git_ok(dir: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .args(["-C", &dir.to_string_lossy()])
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&output.stderr);
    Err(err.trim().to_string())
}

pub fn working_files_cached(dir: &Path) -> Vec<WorkingFile> {
    git_side_cache(dir).files
}

pub fn branches_cached(dir: &Path) -> Vec<String> {
    git_side_cache(dir).branches
}

pub fn oneline_cached(dir: &Path, limit: usize) -> Vec<String> {
    let _ = limit;
    git_side_cache(dir).log
}

#[derive(Clone)]
struct Side {
    status: Option<Status>,
    files: Vec<WorkingFile>,
    branches: Vec<String>,
    log: Vec<String>,
}

pub fn status_cached(dir: &Path) -> Option<Status> {
    git_side_cache(dir).status
}

pub fn invalidate_cache() {
    if let Ok(mut slot) = GIT_SIDE.lock() {
        *slot = None;
    }
}

static GIT_SIDE: Mutex<Option<(PathBuf, Instant, Side)>> = Mutex::new(None);

fn git_side_cache(dir: &Path) -> Side {
    if let Ok(guard) = GIT_SIDE.lock()
        && let Some((root, at, side)) = guard.as_ref()
        && root == dir
        && at.elapsed() < Duration::from_millis(800)
    {
        return side.clone();
    }
    let side = Side {
        status: status(dir),
        files: working_files(dir),
        branches: branches(dir),
        log: oneline(dir, 8),
    };
    if let Ok(mut slot) = GIT_SIDE.lock() {
        *slot = Some((dir.to_path_buf(), Instant::now(), side.clone()));
    }
    side
}

pub fn origin_label(dir: &Path) -> Option<String> {
    let url = git_output(dir, &["remote", "get-url", "origin"])?;
    let label = sanitize_remote(&url);
    (!label.is_empty()).then_some(label)
}

pub fn sanitize_remote(raw: &str) -> String {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix("git@") {
        return rest.trim_end_matches(".git").replace(':', "/");
    }
    let mut rest = raw.to_string();
    for prefix in ["https://", "http://", "ssh://", "git://"] {
        if let Some(stripped) = rest.strip_prefix(prefix) {
            rest = stripped.to_string();
            break;
        }
    }
    if let Some(at) = rest.rfind('@') {
        rest = rest[at + 1..].to_string();
    }
    rest.trim_end_matches(".git")
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_dir_has_no_status() {
        assert!(status(Path::new("/no/such/lolterm-git-root")).is_none());
    }

    #[test]
    fn this_repo_has_a_branch_label() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let git = status(root).expect("lolterm should be a git checkout");
        assert!(!git.branch.is_empty());
        assert!(!git.branch.contains('\n'));
    }

    #[test]
    fn optional_path_empty_means_all_files() {
        assert_eq!(optional_path(None).unwrap(), None);
        assert_eq!(optional_path(Some("")).unwrap(), None);
        assert_eq!(optional_path(Some("  ")).unwrap(), None);
        assert_eq!(
            optional_path(Some("src/git.rs")).unwrap(),
            Some("src/git.rs")
        );
        assert!(optional_path(Some("../secret")).is_err());
    }

    #[test]
    fn porcelain_counts_staged_unstaged_untracked() {
        let sample = "M  staged.txt\n M unstaged.txt\nMM both.txt\n?? new.txt\n";
        assert_eq!(count_porcelain(sample), (2, 2, 1));
        let marks = marks_from_porcelain(sample);
        assert_eq!(marks.get("staged.txt"), Some(&'M'));
        assert_eq!(marks.get("new.txt"), Some(&'?'));
    }

    #[test]
    fn clean_chips_use_plus_tone() {
        let git = Status {
            branch: "master".into(),
            detached: false,
            staged: 0,
            unstaged: 0,
            untracked: 0,
            ahead: 0,
            behind: 0,
        };
        assert_eq!(
            git.chips(),
            vec![
                (" git:master".into(), Tone::Branch),
                (" ✓".into(), Tone::Plus),
            ]
        );
        assert_eq!(git.badge(), "master");
    }

    #[test]
    fn dirty_chips_use_plus_and_minus() {
        let git = Status {
            branch: "dev".into(),
            detached: false,
            staged: 1,
            unstaged: 2,
            untracked: 3,
            ahead: 4,
            behind: 1,
        };
        assert_eq!(
            git.chips(),
            vec![
                (" git:dev".into(), Tone::Branch),
                (" +1".into(), Tone::Plus),
                (" -2".into(), Tone::Minus),
                (" ?3".into(), Tone::Minus),
                (" ↑4".into(), Tone::Plus),
                (" ↓1".into(), Tone::Minus),
            ]
        );
        assert_eq!(git.badge(), "dev*");
    }

    #[test]
    fn sanitize_remote_strips_scheme_and_credentials() {
        assert_eq!(
            sanitize_remote("https://github.com/foo/bar.git"),
            "github.com/foo/bar"
        );
        assert_eq!(
            sanitize_remote("https://x:token@github.com/foo/bar.git"),
            "github.com/foo/bar"
        );
        assert_eq!(
            sanitize_remote("git@github.com:foo/bar.git"),
            "github.com/foo/bar"
        );
    }

    #[test]
    fn worktree_add_checks_out_head() {
        let root = std::env::temp_dir().join(format!("lolterm-wt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("tmpdir");
        let git = |args: &[&str]| {
            Command::new("git")
                .args(["-C", &root.to_string_lossy()])
                .args(args)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        assert!(git(&["init"]));
        assert!(git(&["config", "user.email", "lolterm@test"]));
        assert!(git(&["config", "user.name", "lolterm"]));
        std::fs::write(root.join("README"), "x").expect("readme");
        assert!(git(&["add", "README"]));
        assert!(git(&["commit", "-m", "init"]));
        let wt = root.join("agent-wt");
        worktree_add(&root, &wt, "lolterm/test/1").expect("worktree add");
        assert!(wt.join("README").is_file());
        let _ = git(&["worktree", "remove", "--force", &wt.to_string_lossy()]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
