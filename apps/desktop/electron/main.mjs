import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Menu, app, BrowserWindow, clipboard, dialog, ipcMain } from "electron";

const here = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.join(here, "..");
const repoRoot = path.join(appRoot, "..", "..");

let child = null;
let seq = 1;
const pending = new Map();
let win = null;
const queued = [];

function coreBin() {
  if (process.env.LOLTERM_CORE) return process.env.LOLTERM_CORE;
  const target = process.env.CARGO_TARGET_DIR || path.join(repoRoot, "target");
  return path.join(target, "debug", "lolterm-core");
}

function startCore(openPath) {
  const args = openPath ? [openPath] : [];
  child = spawn(coreBin(), args, { stdio: ["pipe", "pipe", "inherit"] });
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

function createWindow() {
  win = new BrowserWindow({
    width: 1280,
    height: 820,
    backgroundColor: "#ECF2EC",
    title: `LoLTerm v${app.getVersion()}`,
    frame: false,
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(here, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  if (process.env.LOLTERM_DEV) {
    win.loadURL(process.env.LOLTERM_URL || "http://127.0.0.1:5173");
  } else {
    win.loadFile(path.join(appRoot, "dist", "index.html"));
  }
  win.webContents.on("did-finish-load", () => {
    for (const msg of queued) win.webContents.send("core-event", msg);
    queued.length = 0;
  });
}

app.commandLine.appendSwitch("no-sandbox");
app.commandLine.appendSwitch("log-level", "3");

app.whenReady().then(() => {
  Menu.setApplicationMenu(null);
  startCore(process.argv.find((arg) => arg.startsWith("/")));
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
