//! Cliente HTTP mínimo para archivos `.http` / `.rest` del workspace.
//! Usa `curl` del host (no hay runtime HTTP propio ni tokens de LoLTerm).

use std::process::Command;
use std::time::Duration;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct RestResult {
    pub ok: bool,
    pub status: String,
    pub headers: String,
    pub body: String,
}

pub fn looks_like(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.ends_with(".http") || lower.ends_with(".rest")
}

pub fn send(text: &str, env: &[(&str, &str)]) -> Result<RestResult, String> {
    let req = parse(text)?;
    let url = expand(&req.url, env);
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("la URL tiene que ser http(s)".into());
    }
    let mut cmd = Command::new("curl");
    cmd.args([
        "-sS",
        "-D",
        "-",
        "--max-time",
        "30",
        "-X",
        &req.method,
        &url,
    ]);
    for (key, value) in &req.headers {
        cmd.args(["-H", &format!("{key}: {}", expand(value, env))]);
    }
    if !req.body.is_empty() {
        cmd.args(["--data-binary", &expand(&req.body, env)]);
    }
    let output = cmd
        .output()
        .map_err(|err| format!("curl no está o falló: {err}"))?;
    let raw = String::from_utf8_lossy(&output.stdout);
    let (headers, body) = split_headers(&raw);
    let status = headers
        .lines()
        .next()
        .unwrap_or(if output.status.success() {
            "HTTP"
        } else {
            "error"
        })
        .to_string();
    let _ = Duration::from_secs(30);
    Ok(RestResult {
        ok: output.status.success(),
        status,
        headers,
        body,
    })
}

struct Parsed {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: String,
}

fn parse(text: &str) -> Result<Parsed, String> {
    let mut lines = text.lines().peekable();
    while matches!(lines.peek(), Some(line) if line.trim().is_empty() || line.trim_start().starts_with('#'))
    {
        lines.next();
    }
    let first = lines.next().ok_or("archivo HTTP vacío")?.trim();
    let (method, url) = if let Some((left, right)) = first.split_once(' ') {
        let method = left.trim().to_ascii_uppercase();
        if matches!(
            method.as_str(),
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
        ) {
            (method, right.trim().to_string())
        } else {
            ("GET".into(), first.to_string())
        }
    } else {
        ("GET".into(), first.to_string())
    };
    let mut headers = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_body = false;
    for line in lines {
        if !in_body && line.trim().is_empty() {
            in_body = true;
            continue;
        }
        if !in_body {
            if let Some((key, value)) = line.split_once(':') {
                headers.push((key.trim().to_string(), value.trim().to_string()));
            }
        } else {
            body_lines.push(line);
        }
    }
    Ok(Parsed {
        method,
        url,
        headers,
        body: body_lines.join("\n"),
    })
}

fn expand(raw: &str, env: &[(&str, &str)]) -> String {
    let mut out = raw.to_string();
    for (key, value) in env {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
        out = out.replace(&format!("${key}"), value);
    }
    out
}

fn split_headers(raw: &str) -> (String, String) {
    if let Some((head, body)) = raw.split_once("\r\n\r\n") {
        return (head.to_string(), body.to_string());
    }
    if let Some((head, body)) = raw.split_once("\n\n") {
        return (head.to_string(), body.to_string());
    }
    (String::new(), raw.to_string())
}

pub fn dotenv_pairs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        out.push((key.to_string(), value.to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_get_with_header() {
        let req = parse("GET https://example.com/x\nAccept: application/json\n\n").unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com/x");
        assert_eq!(req.headers[0].0, "Accept");
    }

    #[test]
    fn expand_mustache_and_dollar() {
        let env = [("TOKEN", "abc")];
        assert_eq!(expand("Bearer {{TOKEN}}", &env), "Bearer abc");
        assert_eq!(expand("$TOKEN", &env), "abc");
    }

    #[test]
    fn dotenv_skips_comments() {
        let pairs = dotenv_pairs("# x\nFOO=bar\n");
        assert_eq!(pairs, vec![("FOO".into(), "bar".into())]);
    }
}
