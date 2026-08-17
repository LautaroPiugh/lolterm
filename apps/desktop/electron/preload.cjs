const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("lolterm", {
  invoke: (method, params) => ipcRenderer.invoke("core", { method, params }),
  onEvent: (cb) => {
    const listener = (_e, msg) => cb(msg);
    ipcRenderer.on("core-event", listener);
    return () => ipcRenderer.removeListener("core-event", listener);
  },
  onChord: (cb) => {
    const listener = (_e, chord) => cb(chord);
    ipcRenderer.on("chord", listener);
    return () => ipcRenderer.removeListener("chord", listener);
  },
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
});
