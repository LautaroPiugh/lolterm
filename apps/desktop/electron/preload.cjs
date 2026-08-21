const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("lolterm", {
  invoke: (method, params) => ipcRenderer.invoke("core", { method, params }),
  onEvent: (cb) => {
    const listener = (_e, msg) => cb(msg);
    ipcRenderer.on("core-event", listener);
    ipcRenderer.invoke("core-hello").then((params) => {
      if (params && typeof params === "object") cb({ event: "ready", params });
    }).catch(() => {});
    return () => ipcRenderer.removeListener("core-event", listener);
  },
  onChord: (cb) => {
    const listener = (_e, chord) => cb(chord);
    ipcRenderer.on("chord", listener);
    return () => ipcRenderer.removeListener("chord", listener);
  },
  openExternal: (url) => ipcRenderer.invoke("open-external", url),
  openFolder: () => ipcRenderer.invoke("open-folder"),
  window: {
    minimize: () => ipcRenderer.invoke("win-minimize"),
    maximize: () => ipcRenderer.invoke("win-maximize"),
    close: () => ipcRenderer.invoke("win-close"),
  },
  clipboard: {
    read: () => ipcRenderer.invoke("clip-read"),
    write: (text) => ipcRenderer.invoke("clip-write", text ?? ""),
  },
  update: {
    check: () => ipcRenderer.invoke("update-check"),
    install: () => ipcRenderer.invoke("update-install"),
    relaunch: () => ipcRenderer.invoke("app-relaunch"),
  },
});
