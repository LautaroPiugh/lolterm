use std::path::{Path, PathBuf};
use std::process::Command;

pub fn toplevel(dir: &Path) -> Option<PathBuf> {
    let stdout = git_output(dir, &["rev-parse", "--show-toplevel"])?;
    let root = PathBuf::from(stdout);
    root.is_dir().then_some(root)
}

pub fn branch_label(root: &Path) -> Option<String> {
    let abbrev = git_output(root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let mut label = if abbrev == "HEAD" {
        let sha = git_output(root, &["rev-parse", "--short", "HEAD"])?;
        format!("detached@{sha}")
    } else {
        abbrev
    };
    if is_dirty(root) {
        label.push('*');
    }
    Some(label)
}

fn is_dirty(root: &Path) -> bool {
    git_output(root, &["status", "--porcelain"]).is_some_and(|out| !out.is_empty())
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
    fn unknown_dir_has_no_branch() {
        assert!(branch_label(Path::new("/no/such/lolterm-git-root")).is_none());
    }

    #[test]
    fn this_repo_has_a_branch_label() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let label = branch_label(root).expect("lolterm should be a git checkout");
        assert!(!label.is_empty());
        assert!(!label.contains('\n'));
    }
}
