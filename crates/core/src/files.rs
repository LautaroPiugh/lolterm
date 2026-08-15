use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
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
}
