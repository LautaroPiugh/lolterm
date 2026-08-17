import { existsSync, statSync } from "node:fs";
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Menu, app, BrowserWindow, clipboard, dialog, ipcMain, nativeImage } from "electron";

const here = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.join(here, "..");
const repoRoot = path.join(appRoot, "..", "..");

let child = null;
let seq = 1;
const pending = new Map();
let win = null;
const queued = [];

function isFile(file) {
  try {
    return existsSync(file) && statSync(file).isFile();
  } catch {
    return false;
  }
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

function startCore(openPath) {
  const bin = coreBin();
  const args = openPath ? [openPath] : [];
  const cwd = app.isPackaged ? app.getPath("home") : undefined;
  child = spawn(bin, args, { stdio: ["pipe", "pipe", "inherit"], cwd });
  child.on("error", (err) => {
    console.error("lolterm-core:", bin, err);
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
      if (msg.id != null && pending.has(msg.id)) {
        const { resolve, reject } = pending.get(msg.id);
        pending.delete(msg.id);
        if (msg.error) reject(new Error(msg.error));
        else resolve(msg.result);
      } else if (win) {
        win.webContents.send("core-event", msg);
      } else {
        queued.push(msg);
      }
    }
  });
  child.on("exit", () => {
    child = null;
  });
}

function invoke(method, params) {
  return new Promise((resolve, reject) => {
    if (!child?.stdin) {
      reject(new Error("core down"));
      return;
    }
    const id = seq++;
    pending.set(id, { resolve, reject });
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

function appIconImage() {
  const file = appIconPath();
  if (!file) return undefined;
  const image = nativeImage.createFromPath(file);
  return image.isEmpty() ? undefined : image;
}

function createWindow() {
  const icon = appIconImage();
  win = new BrowserWindow({
    width: 1280,
    height: 820,
    backgroundColor: "#ECF2EC",
    title: `LoLTerm v${app.getVersion()}`,
    icon,
    frame: false,
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(here, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  if (icon) win.setIcon(icon);
  if (process.env.LOLTERM_DEV) {
    win.loadURL(process.env.LOLTERM_URL || "http://127.0.0.1:5173");
  } else {
    win.loadFile(path.join(app.getAppPath(), "dist", "index.html"));
  }
  win.webContents.on("did-finish-load", () => {
    for (const msg of queued) win.webContents.send("core-event", msg);
    queued.length = 0;
  });
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
app.commandLine.appendSwitch("no-sandbox");
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
      if (snap && win) win.webContents.send("core-event", { event: "ready", params: snap });
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
    startCore(openDirArg());
    ipcMain.handle("core", (_e, { method, params }) => invoke(method, params));
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
    createWindow();
  });

  app.on("window-all-closed", () => {
    invoke("persist", {}).catch(() => {});
    child?.kill();
    app.quit();
  });
}
