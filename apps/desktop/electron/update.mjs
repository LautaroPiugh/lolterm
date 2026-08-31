import { createHash } from "node:crypto";
import { createWriteStream, readFileSync } from "node:fs";
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

export function pickRpmAsset(assets, arch = process.arch) {
  const list = Array.isArray(assets) ? assets : [];
  const wantArm = arch === "arm64";
  return list.find((asset) => {
    const name = String(asset?.name ?? "");
    if (!name.endsWith(".rpm") || !name.includes("linux")) return false;
    if (wantArm) return name.includes("arm64") || name.includes("aarch64");
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

export async function defaultInstall(pkgPath, type = "deb") {
  // Los rpms todavía no van firmados con GPG, así que dnf exige --nogpgcheck.
  // La integridad ya quedó verificada por SHA256 contra SHA256SUMS.txt.
  const args = type === "rpm"
    ? ["dnf", "install", "-y", "--nogpgcheck", pkgPath]
    : ["apt-get", "install", "-y", pkgPath];
  try {
    await run("pkexec", args);
    return "pkexec";
  } catch {
    await run("xdg-open", [pkgPath]);
    return "xdg-open";
  }
}

const RPM_IDS = new Set([
  "fedora",
  "rhel",
  "centos",
  "rocky",
  "almalinux",
  "ol",
  "amzn",
  "opensuse",
  "opensuse-leap",
  "opensuse-tumbleweed",
  "sles",
  "sle-micro",
]);

function classifyOsRelease(text) {
  const id = /^ID="?([^"\s]+)"?\s*$/m.exec(text)?.[1]?.toLowerCase() ?? "";
  const idLike = /^ID_LIKE="?([^"\n]+)"?/m.exec(text)?.[1]?.toLowerCase() ?? "";
  if (RPM_IDS.has(id) || /\b(rhel|fedora|centos|suse)\b/.test(idLike)) return "rpm";
  return "deb";
}

export function detectPackageType(opts = {}) {
  if (opts.packageType === "rpm" || opts.packageType === "deb") return opts.packageType;
  if (opts.osReleaseText != null) return classifyOsRelease(String(opts.osReleaseText));
  if (process.platform !== "linux") return "deb";
  try {
    return classifyOsRelease(readFileSync("/etc/os-release", "utf8"));
  } catch {
    return "deb";
  }
}

export async function checkLinuxUpdate(opts = {}) {
  if (process.platform !== "linux" && !opts.allowNonLinux) {
    return { available: false, reason: "linux-only" };
  }
  const packageType = detectPackageType(opts);
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
  const picker = packageType === "rpm" ? pickRpmAsset : pickDebAsset;
  const asset = picker(release.assets, opts.arch ?? process.arch);
  const sums = (release.assets ?? []).find((item) => item.name === "SHA256SUMS.txt");
  if (!asset?.browser_download_url || !sums?.browser_download_url) {
    return {
      available: false,
      current,
      latest,
      reason: "no-package",
      packageType,
    };
  }
  return {
    available: compareSemver(latest, current) > 0,
    current,
    latest,
    notes: String(release.body ?? "").slice(0, 400),
    tag: release.tag_name,
    packageType,
    assetName: asset.name,
    assetUrl: asset.browser_download_url,
    sumsUrl: sums.browser_download_url,
  };
}

export async function installLinuxUpdate(opts = {}) {
  const info = await checkLinuxUpdate(opts);
  if (!info.available) {
    throw new Error(info.reason === "no-package" ? "esta release no tiene paquete + SHA256SUMS" : "no hay actualización");
  }
  const destDir = opts.destDir;
  const fetchFn = opts.fetchFn ?? fetch;
  const userAgent = opts.userAgent ?? `LoLTerm/${info.current}`;
  const token = tokenFromEnv(opts.token);
  const pkgPath = path.join(destDir, info.assetName);
  const sumsPath = path.join(destDir, "SHA256SUMS.txt");
  try {
    const digest = await downloadFile(info.assetUrl, pkgPath, fetchFn, userAgent, token);
    await downloadFile(info.sumsUrl, sumsPath, fetchFn, userAgent, token);
    const sumsText = await readFile(sumsPath, "utf8");
    const expected = sha256ForName(sumsText, info.assetName);
    if (!expected || expected !== digest) {
      await unlink(pkgPath).catch(() => {});
      throw new Error(expected ? "SHA256 no coincide; no se instala" : "el paquete no está en SHA256SUMS.txt");
    }
    const install = opts.installFn ?? ((pkg) => defaultInstall(pkg, info.packageType));
    const method = await install(pkgPath);
    if (method !== "xdg-open" && !opts.keepFiles) {
      await unlink(pkgPath).catch(() => {});
    }
    return { ok: true, version: info.latest, path: pkgPath, method: method ?? "pkexec", packageType: info.packageType };
  } finally {
    if (!opts.keepFiles) {
      await unlink(sumsPath).catch(() => {});
    }
  }
}
