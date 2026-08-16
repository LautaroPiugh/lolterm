use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;

const MAX_FILES: usize = 4000;
const MAX_DEPTH: usize = 8;
const SKIP: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    "__pycache__",
    ".venv",
    "venv",
    ".direnv",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileEntry {
    pub rel: String,
    pub is_dir: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TreeRow {
    pub rel: String,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
    pub mark: Option<char>,
    pub lang: Option<String>,
}

pub fn skipped(name: &str) -> bool {
    SKIP.contains(&name)
}

pub fn list_files(root: &Path) -> Vec<FileEntry> {
    let mut out = Vec::new();
    walk_files(root, root, 0, &mut out);
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

fn walk_files(root: &Path, dir: &Path, depth: usize, out: &mut Vec<FileEntry>) {
    if depth > MAX_DEPTH || out.len() >= MAX_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut kids: Vec<(String, PathBuf, bool)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if skipped(&name) {
                return None;
            }
            if name.starts_with('.') && name != ".env" && name != ".gitignore" {
                return None;
            }
            let path = entry.path();
            let is_dir = path.is_dir();
            Some((name, path, is_dir))
        })
        .collect();
    kids.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    for (name, path, is_dir) in kids {
        if out.len() >= MAX_FILES {
            return;
        }
        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or(name);
        out.push(FileEntry {
            rel: rel.clone(),
            is_dir,
        });
        if is_dir {
            walk_files(root, &path, depth + 1, out);
        }
    }
}

pub fn filter_files(files: &[FileEntry], query: &str) -> Vec<FileEntry> {
    let needle = query.trim();
    if needle.is_empty() {
        return files.to_vec();
    }
    let mut scored: Vec<(u32, FileEntry)> = files
        .iter()
        .filter_map(|entry| {
            let score = fuzzy_score(needle, &entry.rel)?;
            Some((score, entry.clone()))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.rel.cmp(&b.1.rel)));
    scored.into_iter().map(|(_, entry)| entry).collect()
}

fn fuzzy_score(needle: &str, haystack: &str) -> Option<u32> {
    if needle.is_empty() {
        return Some(0);
    }
    let hay: Vec<char> = haystack.chars().map(|ch| ch.to_ascii_lowercase()).collect();
    let mut start = 0;
    let mut score = 0u32;
    let mut prev = None;
    for (index, needle_ch) in needle.chars().map(|ch| ch.to_ascii_lowercase()).enumerate() {
        let found = hay
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, ch)| **ch == needle_ch);
        let (at, _) = found?;
        score += 16;
        if index == 0 && at == 0 {
            score += 32;
        }
        if prev.is_some_and(|prev| at == prev + 1) {
            score += 24;
        }
        prev = Some(at);
        start = at + 1;
    }
    Some(score)
}

pub fn visible_tree(
    root: &Path,
    expanded: &HashSet<String>,
    marks: &HashMap<String, char>,
) -> Vec<TreeRow> {
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(".")
        .to_string();
    let mut rows = vec![TreeRow {
        rel: String::new(),
        name,
        depth: 0,
        is_dir: true,
        expanded: expanded.contains(""),
        mark: None,
        lang: None,
    }];
    if expanded.contains("") {
        push_children(root, "", 1, expanded, marks, &mut rows);
    }
    rows
}

fn push_children(
    dir: &Path,
    prefix: &str,
    depth: usize,
    expanded: &HashSet<String>,
    marks: &HashMap<String, char>,
    rows: &mut Vec<TreeRow>,
) {
    if depth > MAX_DEPTH || rows.len() >= MAX_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut kids: Vec<(String, PathBuf, bool)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if skipped(&name) {
                return None;
            }
            if name.starts_with('.') && name != ".env" && name != ".gitignore" {
                return None;
            }
            let path = entry.path();
            let is_dir = path.is_dir();
            Some((name, path, is_dir))
        })
        .collect();
    kids.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    for (name, path, is_dir) in kids {
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let is_expanded = is_dir && expanded.contains(&rel);
        rows.push(TreeRow {
            mark: marks.get(&rel).copied(),
            rel: rel.clone(),
            lang: if is_dir {
                None
            } else {
                language_id(&name).map(str::to_string)
            },
            name,
            depth,
            is_dir,
            expanded: is_expanded,
        });
        if is_expanded {
            push_children(&path, &rel, depth + 1, expanded, marks, rows);
        }
    }
}

pub fn default_expanded() -> HashSet<String> {
    let mut set = HashSet::new();
    set.insert(String::new());
    set
}

pub fn editor() -> Option<(String, Vec<String>)> {
    if let Ok(raw) = std::env::var("VISUAL").or_else(|_| std::env::var("EDITOR")) {
        let mut parts = raw.split_whitespace();
        if let Some(program) = parts.next() {
            let extra = parts.map(ToString::to_string).collect();
            return Some((program.to_string(), extra));
        }
    }
    for name in ["nvim", "vim", "nano", "vi"] {
        if command_on_path(name) {
            return Some((name.to_string(), Vec::new()));
        }
    }
    None
}

pub fn command_on_path(name: &str) -> bool {
    resolve_command(name).is_some()
}

/// Nombre de programa para `lolterm run`: un binario, sin path ni shell.
pub fn program_ok(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

pub fn user_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
}

/// PATH del proceso más el del shell de login (una sola vez, no por comando).
pub fn effective_path() -> Option<String> {
    let dirs = path_dirs();
    if dirs.is_empty() {
        return std::env::var("PATH").ok();
    }
    std::env::join_paths(dirs).ok()?.into_string().ok()
}

/// Busca el binario en PATH (el del sidecar y el de login). Sin `bash -lc` por comando.
pub fn resolve_command(name: &str) -> Option<PathBuf> {
    let name = name.trim();
    if name.contains('/') {
        let path = PathBuf::from(name);
        return path.is_file().then_some(path);
    }
    if !program_ok(name) {
        return None;
    }
    lookup_in_path(name)
}

fn path_dirs() -> &'static [PathBuf] {
    static DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    DIRS.get_or_init(|| {
        let mut dirs = Vec::new();
        let mut seen = HashSet::new();
        let from_env = std::env::var_os("PATH")
            .map(|raw| std::env::split_paths(&raw).collect::<Vec<_>>())
            .unwrap_or_default();
        for dir in from_env.into_iter().chain(login_path_dirs()) {
            if dir.as_os_str().is_empty() || !seen.insert(dir.clone()) {
                continue;
            }
            dirs.push(dir);
        }
        dirs
    })
}

fn login_path_dirs() -> Vec<PathBuf> {
    let Ok(output) = std::process::Command::new(user_shell())
        .args(["-lc", r#"printf %s "$PATH""#])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let Ok(text) = String::from_utf8(output.stdout) else {
        return Vec::new();
    };
    std::env::split_paths(text.trim())
        .filter(|dir| !dir.as_os_str().is_empty())
        .collect()
}

fn lookup_in_path(name: &str) -> Option<PathBuf> {
    path_dirs().iter().find_map(|dir| {
        let candidate = dir.join(name);
        candidate.is_file().then_some(candidate)
    })
}

/// Qué argv pasarle al PTY: binario resuelto, o el shell de login si no está en PATH.
pub fn spawn_argv(program: Option<&str>, args: &[String]) -> (Option<String>, Vec<String>) {
    let Some(name) = program else {
        return (None, args.to_vec());
    };
    if let Some(path) = resolve_command(name) {
        return (Some(path.to_string_lossy().into_owned()), args.to_vec());
    }
    let mut wrapped = vec!["-lc".into(), "exec \"$0\" \"$@\"".into(), name.to_string()];
    wrapped.extend(args.iter().cloned());
    (Some(user_shell()), wrapped)
}

const STACK_MARKERS: &[(&str, &str)] = &[
    ("Cargo.toml", "rust"),
    ("package.json", "node"),
    ("pyproject.toml", "python"),
    ("requirements.txt", "python"),
    ("go.mod", "go"),
    ("composer.json", "php"),
    ("Gemfile", "ruby"),
    ("mix.exs", "elixir"),
];

/// Stack del proyecto según archivos en la raíz. No recorre el árbol.
pub fn stack_from_names(names: &[&str]) -> Vec<String> {
    let mut stack = Vec::new();
    for (file, label) in STACK_MARKERS {
        if names.contains(file) && !stack.iter().any(|item| item == label) {
            stack.push((*label).to_string());
        }
    }
    stack
}

pub fn detect_stack(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    stack_from_names(&refs)
}

pub fn language_id(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if lower == "dockerfile" || lower.starts_with("dockerfile.") {
        return Some("docker");
    }
    if lower == "makefile" || lower == "gnumakefile" {
        return Some("make");
    }
    if lower == "cmakelists.txt" {
        return Some("cmake");
    }
    let ext = std::path::Path::new(&lower)
        .extension()
        .and_then(|ext| ext.to_str())?;
    Some(match ext {
        "rs" => "rust",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "py" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cc" | "cpp" | "hpp" | "cxx" => "cpp",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "cs" => "csharp",
        "css" => "css",
        "scss" | "sass" => "scss",
        "html" | "htm" => "html",
        "json" | "jsonc" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "md" | "mdx" => "markdown",
        "sh" | "bash" | "zsh" => "shell",
        "sql" => "sql",
        "lua" => "lua",
        "vim" => "vim",
        "xml" => "xml",
        "svg" => "svg",
        _ => return None,
    })
}

pub fn join_root(root: &Path, rel: &str) -> PathBuf {
    if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_build_dirs() {
        assert!(skipped("node_modules"));
        assert!(skipped("target"));
        assert!(!skipped("src"));
    }

    #[test]
    fn filter_finds_main() {
        let files = vec![
            FileEntry {
                rel: "src/main.rs".into(),
                is_dir: false,
            },
            FileEntry {
                rel: "src/app.rs".into(),
                is_dir: false,
            },
        ];
        let hits = filter_files(&files, "main");
        assert_eq!(hits[0].rel, "src/main.rs");
    }

    #[test]
    fn language_from_extension_and_filename() {
        assert_eq!(language_id("main.rs"), Some("rust"));
        assert_eq!(language_id("App.tsx"), Some("tsx"));
        assert_eq!(language_id("Dockerfile"), Some("docker"));
        assert_eq!(language_id("notes.txt"), None);
    }

    #[test]
    fn stack_from_root_markers() {
        assert_eq!(
            stack_from_names(&["Cargo.toml", "package.json", "README.md"]),
            vec!["rust", "node"]
        );
        assert_eq!(
            stack_from_names(&["pyproject.toml", "requirements.txt"]),
            vec!["python"]
        );
        assert!(stack_from_names(&["src"]).is_empty());
    }

    #[test]
    fn program_ok_rejects_paths_and_flags() {
        assert!(program_ok("nvim"));
        assert!(program_ok("claude"));
        assert!(!program_ok(""));
        assert!(!program_ok("-rf"));
        assert!(!program_ok("../bin/sh"));
        assert!(!program_ok("foo;bar"));
    }

    #[test]
    fn resolve_command_finds_sh() {
        assert!(resolve_command("sh").is_some() || resolve_command("bash").is_some());
        assert!(resolve_command("definitely-not-a-lolterm-bin").is_none());
    }

    #[test]
    fn spawn_argv_uses_resolved_binary_not_login_shell() {
        let (bin, args) = spawn_argv(Some("sh"), &[]);
        let bin = bin.expect("sh on PATH");
        assert!(bin.ends_with("sh") || bin.ends_with("bash"));
        assert!(!args.iter().any(|arg| arg == "-lc"));
    }

    #[test]
    fn spawn_argv_falls_back_to_login_shell() {
        let (bin, args) = spawn_argv(Some("definitely-not-a-lolterm-bin"), &[]);
        assert_eq!(bin.as_deref(), Some(user_shell().as_str()));
        assert_eq!(args[0], "-lc");
        assert_eq!(args[2], "definitely-not-a-lolterm-bin");
    }
}
