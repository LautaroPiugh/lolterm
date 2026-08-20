use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use color_eyre::Result;
use lolterm_core::layout::{NavDir, SplitDir};
use lolterm_core::mux::Mux;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
struct Request {
    id: u64,
    method: String,
    #[serde(default)]
    params: Value,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let open = std::env::args().nth(1).map(PathBuf::from);
    let (tx, rx) = mpsc::channel::<(u64, Vec<u8>)>();
    let mux = Arc::new(Mutex::new(Mux::boot(open, tx)?));
    lolterm_core::ctl::serve(Arc::clone(&mux));
    let out = Arc::new(Mutex::new(std::io::stdout()));

    {
        let out = Arc::clone(&out);
        std::thread::spawn(move || {
            while let Ok((pane, bytes)) = rx.recv() {
                let line = if bytes.is_empty() {
                    json!({ "event": "exit", "params": { "pane": pane } })
                } else {
                    json!({
                        "event": "data",
                        "params": {
                            "pane": pane,
                            "b64": encode_b64(&bytes),
                        }
                    })
                };
                let mut out = out.lock().expect("stdout");
                let _ = writeln!(out, "{line}");
                let _ = out.flush();
            }
        });
    }

    emit(
        &out,
        json!({ "event": "ready", "params": mux.lock().expect("mux").snapshot() }),
    );

    let stdin = BufReader::new(std::io::stdin());
    for line in stdin.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Request = match parse_request(&line) {
            Ok(req) => req,
            Err(err) => {
                emit(
                    &out,
                    json!({ "event": "core-error", "params": { "error": err } }),
                );
                continue;
            }
        };
        let result = handle(&mux, &req);
        match result {
            Ok(value) => emit(&out, json!({ "id": req.id, "result": value })),
            Err(err) => emit(&out, json!({ "id": req.id, "error": err.to_string() })),
        }
    }
    mux.lock().expect("mux").persist();
    Ok(())
}

fn emit(out: &Arc<Mutex<std::io::Stdout>>, value: Value) {
    let mut out = out.lock().expect("stdout");
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}

fn handle(mux: &Arc<Mutex<Mux>>, req: &Request) -> Result<Value> {
    let mut mux = mux.lock().expect("mux");
    mux.reap();
    let params = &req.params;
    let value = match req.method.as_str() {
        "snapshot" => serde_json::to_value(mux.snapshot())?,
        "context" => serde_json::to_value(mux.context())?,
        "hud" => serde_json::to_value(mux.hud())?,
        "music" => {
            let action = params["action"].as_str().unwrap_or("playPause");
            let volume = params["volume"].as_f64();
            serde_json::to_value(lolterm_core::hud::after_music(
                &mux.process_names(),
                action,
                volume,
            ))?
        }
        "write" => {
            let pane = params["pane"].as_u64().unwrap_or(0);
            let bytes = b64(params["b64"].as_str().unwrap_or(""));
            mux.write(pane, &bytes)?;
            json!({ "ok": true })
        }
        "resize" => {
            let pane = params["pane"].as_u64().unwrap_or(0);
            let cols = params["cols"].as_u64().unwrap_or(80) as u16;
            let rows = params["rows"].as_u64().unwrap_or(24) as u16;
            mux.resize(pane, cols, rows)?;
            json!({ "ok": true })
        }
        "focus" => {
            mux.focus(params["pane"].as_u64().unwrap_or(0));
            serde_json::to_value(mux.snapshot())?
        }
        "selectTab" => {
            mux.select_tab(params["index"].as_u64().unwrap_or(0) as usize);
            serde_json::to_value(mux.snapshot())?
        }
        "newTab" => {
            let program = params["program"]
                .as_str()
                .map(str::trim)
                .filter(|name| !name.is_empty());
            match program {
                None | Some("shell") => {
                    mux.new_tab(None, None, &[], true)?;
                }
                Some("ssh") | Some("tailscale") => {}
                Some(name) => {
                    mux.new_tab(Some(name), None, &[], false)?;
                }
            }
            serde_json::to_value(mux.snapshot())?
        }
        "duplicateTab" => {
            let index = params["index"]
                .as_u64()
                .map(|n| n as usize)
                .unwrap_or_else(|| mux.active_index());
            mux.duplicate_tab(index)?;
            serde_json::to_value(mux.snapshot())?
        }
        "applyPreset" => {
            mux.apply_preset(params["id"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "addStartup" => {
            let args: Vec<String> = params["args"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(ToString::to_string))
                        .collect()
                })
                .unwrap_or_default();
            mux.add_startup(params["program"].as_str().unwrap_or(""), &args)?;
            serde_json::to_value(mux.snapshot())?
        }
        "removeStartup" => {
            mux.remove_startup(params["program"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "setEnv" => {
            mux.set_env(
                params["key"].as_str().unwrap_or(""),
                params["value"].as_str().unwrap_or(""),
            )?;
            serde_json::to_value(mux.snapshot())?
        }
        "removeEnv" => {
            mux.remove_env(params["key"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "renameWorkspace" => {
            mux.rename_workspace(params["name"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "setNotes" => {
            mux.set_notes(params["notes"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "forgetWorkspace" => {
            mux.forget_workspace(PathBuf::from(params["path"].as_str().unwrap_or("")).as_path())?;
            serde_json::to_value(mux.snapshot())?
        }
        "closeTab" => {
            mux.close_tab(params["index"].as_u64().unwrap_or(0) as usize)?;
            serde_json::to_value(mux.snapshot())?
        }
        "split" => {
            let dir = if params["dir"].as_str() == Some("rows") {
                SplitDir::Rows
            } else {
                SplitDir::Columns
            };
            mux.split(dir, None, &[])?;
            serde_json::to_value(mux.snapshot())?
        }
        "setSplit" => {
            mux.set_split(
                params["pane"].as_u64().unwrap_or(0),
                params["other"].as_u64().unwrap_or(0),
                params["percent"].as_u64().unwrap_or(50),
            );
            serde_json::to_value(mux.snapshot())?
        }
        "closePane" => {
            mux.close_pane(params["pane"].as_u64().unwrap_or(0))?;
            serde_json::to_value(mux.snapshot())?
        }
        "run" => {
            let program = params["program"].as_str().unwrap_or("");
            let args: Vec<String> = params["args"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(ToString::to_string))
                        .collect()
                })
                .unwrap_or_default();
            mux.run(program, &args)?;
            serde_json::to_value(mux.snapshot())?
        }
        "openFile" => {
            mux.open_file(params["rel"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "setPaneTitle" => {
            mux.set_pane_title(
                params["pane"].as_u64().unwrap_or(0),
                params["title"].as_str().unwrap_or(""),
            );
            serde_json::to_value(mux.snapshot())?
        }
        "toggleExpand" => {
            mux.toggle_expand(params["rel"].as_str().unwrap_or(""));
            serde_json::to_value(mux.snapshot())?
        }
        "openProject" => {
            let path = PathBuf::from(params["path"].as_str().unwrap_or("."));
            mux.open_project(&path)?;
            serde_json::to_value(mux.snapshot())?
        }
        "ssh" => {
            mux.ssh(params["dest"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "tsSsh" => {
            mux.ts_ssh(
                params["target"].as_str().unwrap_or(""),
                params["user"].as_str().filter(|user| !user.is_empty()),
            )?;
            serde_json::to_value(mux.snapshot())?
        }
        "addMachine" => {
            mux.add_machine(
                params["name"].as_str().unwrap_or(""),
                params["target"].as_str().unwrap_or(""),
                params["kind"].as_str().unwrap_or("ssh"),
                params["user"].as_str(),
            )?;
            serde_json::to_value(mux.snapshot())?
        }
        "forgetMachine" => {
            mux.forget_machine(params["target"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "connectMachine" => {
            mux.connect_machine(
                params["target"].as_str().unwrap_or(""),
                params["user"].as_str().filter(|user| !user.is_empty()),
            )?;
            serde_json::to_value(mux.snapshot())?
        }
        "consumePending" => {
            mux.consume_pending()?;
            serde_json::to_value(mux.snapshot())?
        }
        "setRemoteTmux" => {
            mux.set_remote_tmux(params["tmux"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "searchFiles" => {
            serde_json::to_value(mux.search_files(params["query"].as_str().unwrap_or("")))?
        }
        "sshHosts" => serde_json::to_value(mux.ssh_hosts())?,
        "tsPeers" => serde_json::to_value(mux.ts_peers())?,
        "projects" => serde_json::to_value(mux.recent_projects())?,
        "commands" => serde_json::to_value(mux.commands(params["query"].as_str().unwrap_or("")))?,
        "persist" => {
            mux.persist();
            json!({ "ok": true })
        }
        "setTheme" => {
            mux.set_theme(params["theme"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "setNewTab" => {
            mux.set_new_tab(params["kind"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "saveExtCommand" => {
            let draft: lolterm_core::ext::CommandDraft =
                serde_json::from_value(params.clone()).unwrap_or_default();
            mux.save_ext_command(draft)?;
            serde_json::to_value(mux.snapshot())?
        }
        "removeExtCommand" => {
            mux.remove_ext_command(params["id"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "setKeybinding" => {
            mux.set_keybinding(
                params["chord"].as_str().unwrap_or(""),
                params["command"].as_str().unwrap_or(""),
            )?;
            serde_json::to_value(mux.snapshot())?
        }
        "resetKeybindings" => {
            mux.reset_keybindings()?;
            serde_json::to_value(mux.snapshot())?
        }
        "openConfig" => {
            mux.open_config(params["file"].as_str().unwrap_or("commands"))?;
            serde_json::to_value(mux.snapshot())?
        }
        "dispatch" => {
            mux.dispatch(params["id"].as_str().unwrap_or(""))?;
            serde_json::to_value(mux.snapshot())?
        }
        "zoom" => {
            mux.toggle_zoom();
            serde_json::to_value(mux.snapshot())?
        }
        "swapNav" => {
            if let Some(dir) =
                lolterm_core::layout::NavDir::parse(params["dir"].as_str().unwrap_or(""))
            {
                mux.swap_nav(dir);
            }
            serde_json::to_value(mux.snapshot())?
        }
        "focusNav" => {
            if let Some(dir) =
                lolterm_core::layout::NavDir::parse(params["dir"].as_str().unwrap_or(""))
            {
                mux.focus_nav(dir);
            }
            serde_json::to_value(mux.snapshot())?
        }
        "renameTab" => {
            mux.rename_tab(
                params["index"].as_u64().unwrap_or(0) as usize,
                params["name"].as_str().unwrap_or(""),
            );
            serde_json::to_value(mux.snapshot())?
        }
        "moveTab" => {
            mux.move_tab(
                params["from"].as_u64().unwrap_or(0) as usize,
                params["to"].as_u64().unwrap_or(0) as usize,
            );
            serde_json::to_value(mux.snapshot())?
        }
        "dockTab" => {
            let from = params["from"].as_u64().unwrap_or(0) as usize;
            let edge =
                NavDir::parse(params["edge"].as_str().unwrap_or("")).unwrap_or(NavDir::Right);
            mux.dock_tab(from, edge);
            serde_json::to_value(mux.snapshot())?
        }
        "restartPane" => {
            mux.restart_pane(params["pane"].as_u64().unwrap_or(0))?;
            serde_json::to_value(mux.snapshot())?
        }
        other => return Err(color_eyre::eyre::eyre!("unknown method {other}")),
    };
    Ok(value)
}

fn parse_request(line: &str) -> std::result::Result<Request, String> {
    serde_json::from_str(line).map_err(|err| err.to_string())
}

fn b64(text: &str) -> Vec<u8> {
    decode_b64(text)
}

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode_b64(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(B64[((n >> 18) & 63) as usize] as char);
        out.push(B64[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn decode_b64(text: &str) -> Vec<u8> {
    let mut vals = Vec::new();
    for ch in text.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => continue,
            _ => continue,
        };
        vals.push(v);
    }
    let mut out = Vec::new();
    for chunk in vals.chunks(4) {
        let a = *chunk.first().unwrap_or(&0) as u32;
        let b = *chunk.get(1).unwrap_or(&0) as u32;
        let c = *chunk.get(2).unwrap_or(&0) as u32;
        let d = *chunk.get(3).unwrap_or(&0) as u32;
        let n = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_request_reads_id_and_method() {
        let req = parse_request(r#"{"id":7,"method":"snapshot"}"#).expect("json");
        assert_eq!(req.id, 7);
        assert_eq!(req.method, "snapshot");
    }

    #[test]
    fn parse_request_rejects_garbage_without_fake_id() {
        let err = parse_request("not-json").expect_err("garbage");
        assert!(!err.is_empty());
    }
}
