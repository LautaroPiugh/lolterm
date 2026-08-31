import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Menu, app, BrowserWindow, clipboard, dialog, ipcMain, shell } from "electron";
import { checkLinuxUpdate, installLinuxUpdate } from "./update.mjs";
import { installDevDesktopEntry } from "./linux-desktop.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.join(here, "..");
const repoRoot = path.join(appRoot, "..", "..");
const INVOKE_MS = 8000;
const BOOT_MS = 25000;
const MAX_RESTARTS = 2;
const THEME_FILL = {
  claro: "#f3f3f3",
  oscuro: "#141414",
  contraste: "#0a0a0a",
  tide: "#071c28",
  ember: "#fcf0e0",
};

process.env.CHROME_DESKTOP = app.isPackaged ? "lolterm.desktop" : "lolterm-dev.desktop";

let child = null;
let seq = 1;
const pending = new Map();
let win = null;
const queued = [];
let quitting = false;
let restarting = false;
let restarts = 0;
let lastOpen;
let coreReady = false;
const waitingReady = [];
let lastReady = null;
let rendererReady = false;

function markCoreReady() {
  coreReady = true;
  for (const item of waitingReady.splice(0)) {
    clearTimeout(item.timer);
    item.resolve();
  }
}

function failWaitingReady(err) {
  coreReady = false;
  for (const item of waitingReady.splice(0)) {
    clearTimeout(item.timer);
    item.reject(err);
  }
}

function waitCoreReady() {
  if (coreReady) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const item = { resolve, reject, timer: null };
    item.timer = setTimeout(() => {
      const at = waitingReady.indexOf(item);
      if (at >= 0) waitingReady.splice(at, 1);
      reject(new Error("core timeout"));
    }, BOOT_MS);
    waitingReady.push(item);
  });
}

function isFile(file) {
  try {
    return existsSync(file) && statSync(file).isFile();
  } catch {
    return false;
  }
}

function githubToken() {
  return process.env.GITHUB_TOKEN || process.env.GH_TOKEN || "";
}

function coreBin() {
  if (process.env.LOLTERM_CORE) return process.env.LOLTERM_CORE;
  if (app.isPackaged) {
    const bundled = path.join(process.resourcesPath, "lolterm-core");
    const nested = path.join(process.resourcesPath, "lolterm-core", "lolterm-core");
    if (isFile(bundled)) return bundled;
    if (isFile(nested)) return nested;
    return bundled;
  }
  const target = process.env.CARGO_TARGET_DIR || path.join(repoRoot, "target");
  const debug = path.join(target, "debug", "lolterm-core");
  const release = path.join(target, "release", "lolterm-core");
  if (isFile(debug)) return debug;
  if (isFile(release)) return release;
  return debug;
}

function isDirectory(file) {
  try {
    return existsSync(file) && statSync(file).isDirectory();
  } catch {
    return false;
  }
}

function openDirArg() {
  for (const arg of process.argv.slice(1)) {
    if (arg.startsWith("-")) continue;
    if (arg.startsWith("/") && isDirectory(arg)) return arg;
  }
  return undefined;
}

function sendEvent(msg) {
  if (win && !win.isDestroyed() && rendererReady) {
    win.webContents.send("core-event", msg);
  } else {
    queued.push(msg);
  }
}

function flushPending(err) {
  for (const item of pending.values()) {
    clearTimeout(item.timer);
    item.reject(err);
  }
  pending.clear();
}

function startCore(openPath) {
  lastOpen = openPath;
  const bin = coreBin();
  const args = openPath ? [openPath] : [];
  const cwd = app.isPackaged ? app.getPath("home") : undefined;
  child = spawn(bin, args, { stdio: ["pipe", "pipe", "inherit"], cwd });
  child.on("error", (err) => {
    const wrapped = new Error(`lolterm-core no arrancó: ${err.message}`);
    console.error("lolterm-core:", bin, err);
    failWaitingReady(wrapped);
    flushPending(wrapped);
    sendEvent({ event: "core-down", params: { error: wrapped.message } });
  });
  let buf = "";
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    buf += chunk;
    let idx;
    while ((idx = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, idx).trim();
      buf = buf.slice(idx + 1);
      if (!line) continue;
      let msg;
      try {
        msg = JSON.parse(line);
      } catch {
        continue;
      }
      if (msg.event === "ready") {
        restarts = 0;
        lastReady = msg.params ?? null;
        markCoreReady();
        const theme = msg.params?.theme;
        if (typeof theme === "string" && theme) saveThemeFill(theme);
      }
      if (msg.id != null && pending.has(msg.id)) {
        const { resolve, reject, timer } = pending.get(msg.id);
        pending.delete(msg.id);
        clearTimeout(timer);
        if (msg.error) reject(new Error(msg.error));
        else resolve(msg.result);
      } else {
        sendEvent(msg);
      }
    }
  });
  child.on("exit", () => {
    child = null;
    coreReady = false;
    failWaitingReady(new Error("core exited"));
    flushPending(new Error("core exited"));
    if (quitting || restarting) return;
    if (restarts >= MAX_RESTARTS) {
      sendEvent({ event: "core-down", params: { error: "lolterm-core se cayó" } });
      return;
    }
    restarting = true;
    restarts += 1;
    sendEvent({ event: "core-down", params: { error: "reconectando…" } });
    setTimeout(() => {
      restarting = false;
      startCore(lastOpen);
    }, 400 * restarts);
  });
}

async function invoke(method, params) {
  await waitCoreReady();
  return new Promise((resolve, reject) => {
    if (!child?.stdin) {
      reject(new Error("core down"));
      return;
    }
    const id = seq++;
    const timer = setTimeout(() => {
      if (!pending.has(id)) return;
      pending.delete(id);
      reject(new Error("core timeout"));
    }, INVOKE_MS);
    pending.set(id, { resolve, reject, timer });
    child.stdin.write(`${JSON.stringify({ id, method, params: params ?? {} })}\n`);
  });
}

function appIconPath() {
  const candidates = [
    path.join(process.resourcesPath, "icon.png"),
    path.join(appRoot, "build", "icon.png"),
    path.join(appRoot, "build", "icons", "512x512.png"),
    path.join(appRoot, "dist", "icon.png"),
    path.join(appRoot, "public", "icon.png"),
  ];
  return candidates.find(isFile);
}

function windowStatePath() {
  return path.join(app.getPath("userData"), "window.json");
}

function themeStatePath() {
  return path.join(app.getPath("userData"), "theme.json");
}

function loadThemeFill() {
  try {
    const raw = JSON.parse(readFileSync(themeStatePath(), "utf8"));
    if (typeof raw.fill === "string" && raw.fill.startsWith("#")) return raw.fill;
    return THEME_FILL[raw.theme] || THEME_FILL.claro;
  } catch {
    return THEME_FILL.claro;
  }
}

function saveThemeFill(theme) {
  const fill = THEME_FILL[theme] || THEME_FILL.claro;
  try {
    writeFileSync(themeStatePath(), JSON.stringify({ theme, fill }));
  } catch {
    // estado local; no bloquear el arranque
  }
}

function loadWindowState() {
  try {
    const raw = JSON.parse(readFileSync(windowStatePath(), "utf8"));
    const width = Math.max(800, Number(raw.width) || 1280);
    const height = Math.max(500, Number(raw.height) || 820);
    return {
      x: Number.isFinite(raw.x) ? raw.x : undefined,
      y: Number.isFinite(raw.y) ? raw.y : undefined,
      width,
      height,
      maximized: Boolean(raw.maximized),
    };
  } catch {
    return { width: 1280, height: 820, maximized: false };
  }
}

function saveWindowState() {
  if (!win || win.isDestroyed()) return;
  const bounds = typeof win.getNormalBounds === "function" ? win.getNormalBounds() : win.getBounds();
  const body = JSON.stringify({
    x: bounds.x,
    y: bounds.y,
    width: bounds.width,
    height: bounds.height,
    maximized: win.isMaximized(),
  });
  try {
    writeFileSync(windowStatePath(), body);
  } catch {
    // estado local; no bloquear el cierre
  }
}

function isLoltermGithubUrl(raw) {
  try {
    const url = new URL(raw);
    return url.protocol === "https:" && url.hostname === "github.com" && url.pathname.startsWith("/LautaroPiugh/lolterm");
  } catch {
    return false;
  }
}

function createWindow() {
  rendererReady = false;
  const iconFile = appIconPath();
  const state = loadWindowState();
  win = new BrowserWindow({
    x: state.x,
    y: state.y,
    width: state.width,
    height: state.height,
    backgroundColor: loadThemeFill(),
    title: `LoLTerm v${app.getVersion()}`,
    icon: iconFile,
    frame: false,
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(here, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  if (state.maximized) win.maximize();
  if (iconFile) win.setIcon(iconFile);
  if (process.env.LOLTERM_DEV) {
    win.loadURL(process.env.LOLTERM_URL || "http://127.0.0.1:5173");
  } else {
    win.loadFile(path.join(app.getAppPath(), "dist", "index.html"));
  }
  win.webContents.on("did-finish-load", () => {
    rendererReady = true;
    for (const msg of queued) win.webContents.send("core-event", msg);
    queued.length = 0;
  });
  let saveTimer;
  const scheduleSave = () => {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(saveWindowState, 400);
  };
  win.on("resize", scheduleSave);
  win.on("move", scheduleSave);
  win.on("maximize", scheduleSave);
  win.on("unmaximize", scheduleSave);
  win.on("close", saveWindowState);
  // Chromium se queda con Ctrl-Tab; hay que interceptarlo aquí o no llega al renderer.
  win.webContents.on("before-input-event", (event, input) => {
    if (input.type !== "keyDown" || input.key !== "Tab" || !input.control) return;
    event.preventDefault();
    win.webContents.send("chord", input.shift ? "ctrl+shift+tab" : "ctrl+tab");
  });
}

process.env.GTK_OVERLAY_SCROLLING = "0";
app.setName("LoLTerm");
app.setDesktopName("lolterm.desktop");
app.commandLine.appendSwitch("class", "LoLTerm");
// En dev el binario de electron no viaja con chrome-sandbox SUID usable; empaquetado sí.
if (!app.isPackaged) app.commandLine.appendSwitch("no-sandbox");
app.commandLine.appendSwitch("disable-features", "OverlayScrollbar,FluentOverlayScrollbar");
app.commandLine.appendSwitch("disable-blink-features", "OverlayScrollbars");
app.commandLine.appendSwitch("log-level", "3");

function focusWindow() {
  if (!win) return;
  if (win.isMinimized()) win.restore();
  win.show();
  win.focus();
}

function consumePendingLaunch() {
  invoke("consumePending")
    .then((snap) => {
      if (snap && win && !win.isDestroyed()) {
        try {
          win.webContents.send("core-event", { event: "ready", params: snap });
        } catch {
          // el frame pudo morir entre el check y el send; no es fatal
        }
      }
    })
    .catch(() => {});
}

const gotLock = app.requestSingleInstanceLock();
if (!gotLock) {
  app.quit();
} else {
  app.on("second-instance", () => {
    focusWindow();
    consumePendingLaunch();
  });

  app.whenReady().then(() => {
    Menu.setApplicationMenu(null);
    if (!app.isPackaged) {
      installDevDesktopEntry({
        electronBin: process.execPath,
        appRoot,
        iconFile: appIconPath(),
      });
    }
    startCore(openDirArg());
    ipcMain.handle("core", (_e, { method, params }) => invoke(method, params));
    ipcMain.handle("core-hello", () => lastReady);
    ipcMain.handle("open-external", (_e, url) => {
      if (typeof url !== "string" || !isLoltermGithubUrl(url)) return;
      return shell.openExternal(url);
    });
    ipcMain.handle("win-minimize", () => {
      win?.minimize();
    });
    ipcMain.handle("win-maximize", () => {
      if (!win) return;
      if (win.isMaximized()) win.unmaximize();
      else win.maximize();
    });
    ipcMain.handle("win-close", () => {
      win?.close();
    });
    ipcMain.handle("clip-read", () => clipboard.readText());
    ipcMain.handle("clip-write", (_e, text) => {
      clipboard.writeText(typeof text === "string" ? text : "");
    });
    ipcMain.handle("open-folder", async () => {
      const picked = await dialog.showOpenDialog(win, { properties: ["openDirectory"] });
      if (picked.canceled || !picked.filePaths[0]) return null;
      return invoke("openProject", { path: picked.filePaths[0] });
    });
    ipcMain.handle("update-check", () =>
      checkLinuxUpdate({
        currentVersion: app.getVersion(),
        userAgent: `LoLTerm/${app.getVersion()}`,
        token: githubToken(),
      }),
    );
    ipcMain.handle("update-install", () =>
      installLinuxUpdate({
        currentVersion: app.getVersion(),
        destDir: app.getPath("temp"),
        userAgent: `LoLTerm/${app.getVersion()}`,
        token: githubToken(),
      }),
    );
    ipcMain.handle("app-relaunch", () => {
      app.relaunch();
      app.quit();
    });
    createWindow();
  });

  app.on("window-all-closed", () => {
    quitting = true;
    invoke("persist", {})
      .catch(() => {})
      .finally(() => {
        child?.kill();
        app.quit();
      });
  });
}
