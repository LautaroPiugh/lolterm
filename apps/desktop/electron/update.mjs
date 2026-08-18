import { createHash } from "node:crypto";
import { createWriteStream } from "node:fs";
import { readFile, unlink } from "node:fs/promises";
import { spawn } from "node:child_process";
import path from "node:path";
import { pipeline } from "node:stream/promises";
import { Readable } from "node:stream";

const DEFAULT_REPO = "LautaroPiugh/lolterm";
const ALLOWED_HOSTS = new Set([
  "github.com",
  "api.github.com",
  "objects.githubusercontent.com",
  "release-assets.githubusercontent.com",
  "github-releases.githubusercontent.com",
]);

export function compareSemver(a, b) {
  const pa = String(a)
    .replace(/^v/i, "")
    .split(".")
    .map((n) => Number.parseInt(n, 10) || 0);
  const pb = String(b)
    .replace(/^v/i, "")
    .split(".")
    .map((n) => Number.parseInt(n, 10) || 0);
  for (let i = 0; i < 3; i += 1) {
    const left = pa[i] ?? 0;
    const right = pb[i] ?? 0;
    if (left > right) return 1;
    if (left < right) return -1;
  }
  return 0;
}

export function sha256ForName(sumsText, filename) {
  const base = path.basename(filename);
  for (const line of String(sumsText).split(/\r?\n/)) {
    const match = line.match(/^([a-fA-F0-9]{64})\s+\*?(\S+)\s*$/);
    if (match && path.basename(match[2]) === base) return match[1].toLowerCase();
  }
  return null;
}

export function pickDebAsset(assets, arch = process.arch) {
  const list = Array.isArray(assets) ? assets : [];
  const wantArm = arch === "arm64";
  return list.find((asset) => {
    const name = String(asset?.name ?? "");
    if (!name.endsWith(".deb") || !name.includes("linux")) return false;
    if (wantArm) return name.includes("arm64");
    return name.includes("amd64") || name.includes("x64") || name.includes("x86_64");
  });
}

export function githubAuthHeaders(token) {
  const headers = {};
  const value = String(token ?? "").trim();
  if (value) headers.Authorization = `Bearer ${value}`;
  return headers;
}

function tokenFromEnv(explicit) {
  if (explicit) return String(explicit).trim();
  return String(process.env.GITHUB_TOKEN || process.env.GH_TOKEN || "").trim();
}

function assertGithubUrl(raw) {
  let url;
  try {
    url = new URL(raw);
  } catch {
    throw new Error("url de descarga inválida");
  }
  if (url.protocol !== "https:") throw new Error("solo https");
  if (!ALLOWED_HOSTS.has(url.hostname)) {
    throw new Error(`origen no confiable: ${url.hostname}`);
  }
  return url.toString();
}

async function githubJson(url, fetchFn, userAgent, token) {
  const res = await fetchFn(assertGithubUrl(url), {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": userAgent,
      "X-GitHub-Api-Version": "2022-11-28",
      ...githubAuthHeaders(token),
    },
  });
  if (res.status === 404 || res.status === 403) return { status: res.status, body: null };
  if (!res.ok) throw new Error(`GitHub ${res.status}`);
  return { status: res.status, body: await res.json() };
}

async function downloadFile(url, dest, fetchFn, userAgent, token) {
  const res = await fetchFn(assertGithubUrl(url), {
    headers: {
      "User-Agent": userAgent,
      Accept: "application/octet-stream",
      ...githubAuthHeaders(token),
    },
    redirect: "follow",
  });
  if (!res.ok || !res.body) throw new Error(`descarga ${res.status}`);
  const hash = createHash("sha256");
  const file = createWriteStream(dest);
  const hashed = Readable.fromWeb(res.body).on("data", (chunk) => hash.update(chunk));
  await pipeline(hashed, file);
  return hash.digest("hex");
}

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"] });
    let err = "";
    child.stderr?.on("data", (chunk) => {
      err += chunk;
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) resolve();
      else reject(new Error(err.trim() || `${command} salió ${code}`));
    });
  });
}

export async function defaultInstallDeb(debPath) {
  try {
    await run("pkexec", ["apt-get", "install", "-y", debPath]);
    return "pkexec";
  } catch {
    await run("xdg-open", [debPath]);
    return "xdg-open";
  }
}

export async function checkLinuxDebUpdate(opts = {}) {
  if (process.platform !== "linux" && !opts.allowNonLinux) {
    return { available: false, reason: "linux-only" };
  }
  const current = String(opts.currentVersion ?? "0.0.0").replace(/^v/i, "");
  const repo = opts.repo ?? DEFAULT_REPO;
  const fetchFn = opts.fetchFn ?? fetch;
  const userAgent = opts.userAgent ?? `LoLTerm/${current}`;
  const token = tokenFromEnv(opts.token);
  const fetched = await githubJson(
    `https://api.github.com/repos/${repo}/releases/latest`,
    fetchFn,
    userAgent,
    token,
  );
  if (!fetched.body) {
    return {
      available: false,
      current,
      reason: fetched.status === 403 ? "github-403" : "github-404",
    };
  }
  const release = fetched.body;
  const latest = String(release.tag_name ?? release.name ?? "").replace(/^v/i, "");
  if (!latest) return { available: false, reason: "no-tag" };
  const deb = pickDebAsset(release.assets, opts.arch ?? process.arch);
  const sums = (release.assets ?? []).find((asset) => asset.name === "SHA256SUMS.txt");
  if (!deb?.browser_download_url || !sums?.browser_download_url) {
    return {
      available: false,
      current,
      latest,
      reason: "no-deb",
    };
  }
  return {
    available: compareSemver(latest, current) > 0,
    current,
    latest,
    notes: String(release.body ?? "").slice(0, 400),
    tag: release.tag_name,
    debName: deb.name,
    debUrl: deb.browser_download_url,
    sumsUrl: sums.browser_download_url,
  };
}

export async function installLinuxDebUpdate(opts) {
  const info = await checkLinuxDebUpdate(opts);
  if (!info.available) {
    throw new Error(info.reason === "no-deb" ? "esta release no tiene .deb + SHA256SUMS" : "no hay actualización");
  }
  const destDir = opts.destDir;
  const fetchFn = opts.fetchFn ?? fetch;
  const userAgent = opts.userAgent ?? `LoLTerm/${info.current}`;
  const token = tokenFromEnv(opts.token);
  const debPath = path.join(destDir, info.debName);
  const sumsPath = path.join(destDir, "SHA256SUMS.txt");
  try {
    const digest = await downloadFile(info.debUrl, debPath, fetchFn, userAgent, token);
    await downloadFile(info.sumsUrl, sumsPath, fetchFn, userAgent, token);
    const sumsText = await readFile(sumsPath, "utf8");
    const expected = sha256ForName(sumsText, info.debName);
    if (!expected || expected !== digest) {
      await unlink(debPath).catch(() => {});
      throw new Error(expected ? "SHA256 no coincide; no se instala" : "el .deb no está en SHA256SUMS.txt");
    }
    const install = opts.installFn ?? defaultInstallDeb;
    const method = await install(debPath);
    if (method !== "xdg-open" && !opts.keepFiles) {
      await unlink(debPath).catch(() => {});
    }
    return { ok: true, version: info.latest, path: debPath, method: method ?? "pkexec" };
  } finally {
    if (!opts.keepFiles) {
      await unlink(sumsPath).catch(() => {});
    }
  }
}
