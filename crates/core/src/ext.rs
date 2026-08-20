//! Extensiones locales: TOML en config, sin JS ni plugins remotos.
//! Un comando custom solo puede `run` un binario con `program_ok`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::config;
use crate::context;
use crate::files;
use crate::session;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtCommand {
    pub id: String,
    pub slash: String,
    pub hint: String,
    pub run: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtHook {
    pub on: String,
    pub run: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemePack {
    pub id: String,
    pub label: String,
    pub hint: String,
    pub vars: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusItem {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct Bundle {
    pub commands: Vec<ExtCommand>,
    pub hooks: Vec<ExtHook>,
    pub themes: Vec<ThemePack>,
    pub status: Vec<StatusSpec>,
    pub context_files: Vec<PathBuf>,
    pub extensions: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct StatusSpec {
    pub id: String,
    pub file: Option<PathBuf>,
    pub run: Option<String>,
}

#[derive(Default, Deserialize)]
struct FileList {
    #[serde(default)]
    command: Vec<RawCommand>,
    #[serde(default)]
    hook: Vec<RawHook>,
    #[serde(default)]
    status: Vec<RawStatus>,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    theme: Option<RawTheme>,
}

#[derive(Default, Deserialize)]
struct RawCommand {
    #[serde(default)]
    id: String,
    #[serde(default)]
    slash: String,
    #[serde(default)]
    hint: String,
    #[serde(default)]
    run: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Default, Deserialize)]
struct RawHook {
    #[serde(default)]
    on: String,
    #[serde(default)]
    run: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Default, Deserialize)]
struct RawStatus {
    #[serde(default)]
    id: String,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    run: Option<String>,
}

#[derive(Default, Deserialize)]
struct RawTheme {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    hint: String,
    #[serde(default)]
    bg: Option<String>,
    #[serde(default)]
    fg: Option<String>,
    #[serde(default)]
    accent: Option<String>,
    #[serde(default)]
    bar: Option<String>,
    #[serde(default)]
    pane: Option<String>,
}

pub fn commands_path() -> PathBuf {
    config::config_dir().join("commands.toml")
}

/// Solo `commands.toml` del usuario. Los packs en `extensions/` no se reescriben.
pub fn user_commands() -> Vec<ExtCommand> {
    user_commands_at(&commands_path())
}

pub fn user_commands_at(path: &Path) -> Vec<ExtCommand> {
    let mut bundle = Bundle::default();
    ingest_file(&mut bundle, path, None);
    bundle.commands
}

pub fn upsert_user_command(draft: CommandDraft) -> Result<ExtCommand, String> {
    upsert_user_command_at(&commands_path(), draft)
}

pub fn upsert_user_command_at(path: &Path, draft: CommandDraft) -> Result<ExtCommand, String> {
    let cmd = sanitize_command(RawCommand {
        id: draft.id.unwrap_or_default(),
        slash: draft.slash,
        hint: draft.hint,
        run: draft.run,
        args: draft.args,
    })
    .ok_or_else(|| {
        "comando inválido: `run` es un binario (htop), `slash` tipo htop, id `ext.<slug>`"
            .to_string()
    })?;
    let mut cmds = user_commands_at(path);
    cmds.retain(|seen| seen.id != cmd.id && seen.slash != cmd.slash);
    cmds.push(cmd.clone());
    write_user_commands_at(path, &cmds).map_err(|err| err.to_string())?;
    Ok(cmd)
}

pub fn remove_user_command(id: &str) -> Result<(), String> {
    remove_user_command_at(&commands_path(), id)
}

pub fn remove_user_command_at(path: &Path, id: &str) -> Result<(), String> {
    let needle = id.trim().trim_start_matches('/');
    let mut cmds = user_commands_at(path);
    let before = cmds.len();
    cmds.retain(|cmd| cmd.id != needle && cmd.slash != needle);
    if cmds.len() == before {
        return Err(format!("no está en commands.toml: {needle}"));
    }
    write_user_commands_at(path, &cmds).map_err(|err| err.to_string())
}

fn write_user_commands_at(path: &Path, cmds: &[ExtCommand]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = UserCommandsFile {
        command: cmds.to_vec(),
    };
    let body = toml::to_string_pretty(&file).unwrap_or_default();
    std::fs::write(
        path,
        format!("# Comandos custom (ext.<slug>). También se editan en LoLTerm → /commands\n{body}"),
    )
}

#[derive(Serialize)]
struct UserCommandsFile {
    command: Vec<ExtCommand>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CommandDraft {
    pub id: Option<String>,
    #[serde(default)]
    pub slash: String,
    #[serde(default)]
    pub hint: String,
    #[serde(default)]
    pub run: String,
    #[serde(default)]
    pub args: Vec<String>,
}

pub fn load() -> Bundle {
    let mut bundle = Bundle::default();
    ingest_file(
        &mut bundle,
        &config::config_dir().join("commands.toml"),
        None,
    );
    ingest_file(&mut bundle, &config::config_dir().join("hooks.toml"), None);
    ingest_file(&mut bundle, &config::config_dir().join("status.toml"), None);
    ingest_file(
        &mut bundle,
        &config::config_dir().join("context.toml"),
        None,
    );
    if let Ok(entries) = std::fs::read_dir(config::config_dir().join("themes")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
                ingest_theme_file(&mut bundle, &path);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(config::config_dir().join("extensions")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest = path.join("extension.toml");
            ingest_file(&mut bundle, &manifest, Some(&path));
        }
    }
    bundle
}

pub fn all_themes() -> Vec<ThemePack> {
    let mut out = builtin_themes();
    for theme in load().themes {
        out.retain(|item| item.id != theme.id);
        out.push(theme);
    }
    out
}

pub fn builtin_themes() -> Vec<ThemePack> {
    vec![
        pack_orq(
            "claro",
            "Claro",
            "papel frío",
            [242, 246, 250],
            [32, 38, 46],
            [148, 160, 174],
            [226, 232, 238],
            [252, 253, 254],
        ),
        pack_orq(
            "oscuro",
            "Oscuro",
            "carbón",
            [13, 17, 15],
            [214, 218, 214],
            [50, 62, 56],
            [22, 30, 26],
            [7, 9, 8],
        ),
        pack_orq(
            "tide",
            "Tide",
            "mar de noche",
            [7, 28, 40],
            [196, 228, 234],
            [22, 78, 100],
            [10, 40, 54],
            [4, 16, 24],
        ),
        pack_orq(
            "ember",
            "Ember",
            "cobre",
            [252, 240, 224],
            [56, 32, 18],
            [196, 142, 82],
            [244, 222, 192],
            [255, 250, 244],
        ),
    ]
}

pub fn command(name: &str) -> Option<ExtCommand> {
    let needle = name.trim().trim_start_matches('/');
    load()
        .commands
        .into_iter()
        .find(|cmd| cmd.id == needle || cmd.slash == needle)
}

pub fn hooks_for(event: &str) -> Vec<ExtHook> {
    load()
        .hooks
        .into_iter()
        .filter(|hook| hook.on == event)
        .collect()
}

struct StatusCache {
    at: Instant,
    root: PathBuf,
    items: Vec<StatusItem>,
}

static STATUS_CACHE: Mutex<Option<StatusCache>> = Mutex::new(None);

pub fn status_items(root: &Path) -> Vec<StatusItem> {
    if let Ok(guard) = STATUS_CACHE.lock()
        && let Some(cache) = guard.as_ref()
        && cache.root == root
        && cache.at.elapsed() < Duration::from_secs(8)
    {
        return cache.items.clone();
    }
    let items = compute_status_items(root);
    if let Ok(mut guard) = STATUS_CACHE.lock() {
        *guard = Some(StatusCache {
            at: Instant::now(),
            root: root.to_path_buf(),
            items: items.clone(),
        });
    }
    items
}

fn compute_status_items(root: &Path) -> Vec<StatusItem> {
    let mut out = Vec::new();
    for spec in load().status {
        let text = spec
            .file
            .as_ref()
            .and_then(|path| read_status_file(root, path))
            .or_else(|| spec.run.as_deref().and_then(run_status_line));
        let Some(text) = text else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        out.push(StatusItem { id: spec.id, text });
    }
    out
}

pub fn extra_context(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for path in load().context_files {
        let resolved = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        merge_context_file(&mut out, &resolved);
    }
    out
}

pub fn theme_known(id: &str) -> bool {
    all_themes().iter().any(|theme| theme.id == id)
}

fn ingest_file(bundle: &mut Bundle, path: &Path, base: Option<&Path>) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(file) = toml::from_str::<FileList>(&text) else {
        return;
    };
    if let Some(name) = file
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        bundle.extensions.push(name.to_string());
    } else if let Some(dir) = base.and_then(|path| path.file_name()) {
        bundle.extensions.push(dir.to_string_lossy().into_owned());
    }
    for raw in file.command {
        if let Some(cmd) = sanitize_command(raw) {
            bundle
                .commands
                .retain(|seen| seen.id != cmd.id && seen.slash != cmd.slash);
            bundle.commands.push(cmd);
        }
    }
    for raw in file.hook {
        if let Some(hook) = sanitize_hook(raw) {
            bundle.hooks.push(hook);
        }
    }
    for raw in file.status {
        if let Some(spec) = sanitize_status(raw, base) {
            bundle.status.retain(|seen| seen.id != spec.id);
            bundle.status.push(spec);
        }
    }
    for rel in file.files {
        let path = resolve_ext_path(base, &rel);
        if let Some(path) = path {
            bundle.context_files.push(path);
        }
    }
    if let Some(raw) = file.theme
        && let Some(theme) = sanitize_theme(raw)
    {
        bundle.themes.retain(|seen| seen.id != theme.id);
        bundle.themes.push(theme);
    }
}

fn ingest_theme_file(bundle: &mut Bundle, path: &Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(raw) = toml::from_str::<RawTheme>(&text) else {
        return;
    };
    if let Some(theme) = sanitize_theme(raw) {
        bundle.themes.retain(|seen| seen.id != theme.id);
        bundle.themes.push(theme);
    }
}

fn sanitize_command(raw: RawCommand) -> Option<ExtCommand> {
    if !files::program_ok(&raw.run) {
        return None;
    }
    let args: Vec<String> = raw.args.into_iter().filter(|arg| arg_ok(arg)).collect();
    let slash = slug(&raw.slash).or_else(|| slug(&raw.run))?;
    let id = id_ok(&raw.id).unwrap_or_else(|| format!("ext.{slash}"));
    if !id.starts_with("ext.") {
        return None;
    }
    Some(ExtCommand {
        id,
        slash,
        hint: truncate(&raw.hint, 80),
        run: raw.run.trim().to_string(),
        args,
    })
}

fn sanitize_hook(raw: RawHook) -> Option<ExtHook> {
    if !files::program_ok(&raw.run) {
        return None;
    }
    let on = raw.on.trim();
    if !matches!(on, "workspace.open") {
        return None;
    }
    Some(ExtHook {
        on: on.into(),
        run: raw.run.trim().to_string(),
        args: raw.args.into_iter().filter(|arg| arg_ok(arg)).collect(),
    })
}

fn sanitize_status(raw: RawStatus, base: Option<&Path>) -> Option<StatusSpec> {
    let id = slug(&raw.id)?;
    let file = raw
        .file
        .as_deref()
        .and_then(|rel| resolve_ext_path(base, rel));
    let run = raw
        .run
        .as_deref()
        .map(str::trim)
        .filter(|name| files::program_ok(name))
        .map(str::to_string);
    if file.is_none() && run.is_none() {
        return None;
    }
    Some(StatusSpec { id, file, run })
}

fn sanitize_theme(raw: RawTheme) -> Option<ThemePack> {
    let id = slug(&raw.id)?;
    if reserved_theme(&id) {
        return None;
    }
    let bg = color_ok(raw.bg.as_deref()).unwrap_or("#ecf2ec");
    let fg = color_ok(raw.fg.as_deref()).unwrap_or("#28302a");
    let accent = color_ok(raw.accent.as_deref()).unwrap_or("#488c58");
    let bar = color_ok(raw.bar.as_deref()).unwrap_or(bg);
    let pane = color_ok(raw.pane.as_deref()).unwrap_or(bg);
    Some(pack(
        &id,
        if raw.label.trim().is_empty() {
            &id
        } else {
            raw.label.trim()
        },
        raw.hint.trim(),
        Colors {
            fill: bg,
            text: fg,
            brand: accent,
            bar,
            pane,
        },
    ))
}

struct Colors<'a> {
    fill: &'a str,
    text: &'a str,
    brand: &'a str,
    bar: &'a str,
    pane: &'a str,
}

fn reserved_theme(id: &str) -> bool {
    matches!(id, "claro" | "oscuro" | "tide" | "ember")
}

fn rgb(n: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", n[0], n[1], n[2])
}

#[allow(clippy::too_many_arguments)]
fn pack_orq(
    id: &str,
    label: &str,
    hint: &str,
    n900: [u8; 3],
    n100: [u8; 3],
    n600: [u8; 3],
    n800: [u8; 3],
    n950: [u8; 3],
) -> ThemePack {
    let fill = rgb(n900);
    let text = rgb(n100);
    let brand = rgb(n600);
    let bar = rgb(n800);
    let pane = rgb(n950);
    pack(
        id,
        label,
        hint,
        Colors {
            fill: &fill,
            text: &text,
            brand: &brand,
            bar: &bar,
            pane: &pane,
        },
    )
}

fn pack(id: &str, label: &str, hint: &str, colors: Colors<'_>) -> ThemePack {
    let mut vars = BTreeMap::new();
    vars.insert("fill".into(), colors.fill.into());
    vars.insert("text".into(), colors.text.into());
    vars.insert("brand".into(), colors.brand.into());
    vars.insert("bar".into(), colors.bar.into());
    vars.insert("pane".into(), colors.pane.into());
    vars.insert("muted".into(), colors.text.into());
    vars.insert("focus".into(), colors.brand.into());
    vars.insert("border".into(), colors.bar.into());
    vars.insert("err".into(), "#b04040".into());
    vars.insert("ok".into(), colors.brand.into());
    ThemePack {
        id: id.into(),
        label: truncate(label, 24),
        hint: truncate(hint, 40),
        vars,
    }
}

fn arg_ok(arg: &str) -> bool {
    let arg = arg.trim();
    !arg.is_empty()
        && !arg.starts_with('-')
        && !arg.contains("..")
        && !arg.contains('\0')
        && !arg.contains('/')
}

fn slug(raw: &str) -> Option<String> {
    let raw = raw.trim().to_ascii_lowercase();
    if raw.is_empty()
        || raw.len() > 32
        || !raw
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        || !raw.chars().next()?.is_ascii_lowercase()
    {
        return None;
    }
    Some(raw)
}

fn id_ok(raw: &str) -> Option<String> {
    let raw = raw.trim().to_ascii_lowercase();
    let rest = raw.strip_prefix("ext.")?;
    slug(rest).map(|slug| format!("ext.{slug}"))
}

fn color_ok(raw: Option<&str>) -> Option<&str> {
    let raw = raw?.trim();
    if raw.len() == 7 && raw.starts_with('#') && raw[1..].bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(raw)
    } else {
        None
    }
}

fn resolve_ext_path(base: Option<&Path>, rel: &str) -> Option<PathBuf> {
    let rel = rel.trim();
    if rel.is_empty() || rel.contains("..") || rel.starts_with('/') {
        return None;
    }
    Some(match base {
        Some(dir) => dir.join(rel),
        None => PathBuf::from(rel),
    })
}

fn read_status_file(root: &Path, path: &Path) -> Option<String> {
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let text = std::fs::read_to_string(resolved).ok()?;
    Some(truncate(text.lines().next().unwrap_or("").trim(), 40))
}

fn run_status_line(program: &str) -> Option<String> {
    if !files::program_ok(program) {
        return None;
    }
    let output = std::process::Command::new(program)
        .env("PATH", files::effective_path().unwrap_or_default())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(truncate(text.lines().next().unwrap_or("").trim(), 40))
}

fn merge_context_file(out: &mut BTreeMap<String, String>, path: &Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let Some(map) = value.as_object() else {
        return;
    };
    for (key, value) in map {
        if !session::env_key_ok(key) || context::looks_secret(key) {
            continue;
        }
        let Some(text) = value.as_str() else {
            continue;
        };
        if text.len() > 200 || context::looks_secret(text) {
            continue;
        }
        out.insert(key.clone(), truncate(text, 200));
    }
}

fn truncate(text: &str, max: usize) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if out.len() >= max {
            break;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_and_flags_in_args() {
        assert!(
            sanitize_command(RawCommand {
                run: "/bin/htop".into(),
                slash: "htop".into(),
                ..RawCommand::default()
            })
            .is_none()
        );
        let cmd = sanitize_command(RawCommand {
            run: "htop".into(),
            slash: "htop".into(),
            hint: "monitor".into(),
            args: vec!["-c".into(), "README.md".into()],
            ..RawCommand::default()
        })
        .expect("htop");
        assert_eq!(cmd.id, "ext.htop");
        assert_eq!(cmd.args, vec!["README.md"]);
    }

    #[test]
    fn color_and_theme_id() {
        assert_eq!(color_ok(Some("#88c0d0")), Some("#88c0d0"));
        assert_eq!(color_ok(Some("red")), None);
        assert!(
            sanitize_theme(RawTheme {
                id: "nord".into(),
                accent: Some("#88c0d0".into()),
                ..RawTheme::default()
            })
            .is_some()
        );
        assert!(
            sanitize_theme(RawTheme {
                id: "claro".into(),
                ..RawTheme::default()
            })
            .is_none()
        );
    }

    #[test]
    fn upsert_rewrites_user_commands_file() {
        let dir = std::env::temp_dir().join(format!(
            "lolterm-ext-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("commands.toml");
        let cmd = upsert_user_command_at(
            &path,
            CommandDraft {
                slash: "htop".into(),
                hint: "monitor".into(),
                run: "htop".into(),
                ..CommandDraft::default()
            },
        )
        .expect("upsert");
        assert_eq!(cmd.id, "ext.htop");
        assert_eq!(user_commands_at(&path).len(), 1);
        remove_user_command_at(&path, "ext.htop").expect("remove");
        assert!(user_commands_at(&path).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_workspace_open_hooks() {
        assert!(
            sanitize_hook(RawHook {
                on: "workspace.open".into(),
                run: "lazygit".into(),
                ..RawHook::default()
            })
            .is_some()
        );
        assert!(
            sanitize_hook(RawHook {
                on: "shutdown".into(),
                run: "lazygit".into(),
                ..RawHook::default()
            })
            .is_none()
        );
    }
}
