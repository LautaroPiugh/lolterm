use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

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
}
