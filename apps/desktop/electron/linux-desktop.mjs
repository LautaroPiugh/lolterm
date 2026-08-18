import { copyFileSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";

function quote(value) {
  return `"${String(value).replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
}

export function installDevDesktopEntry({ electronBin, appRoot, iconFile }) {
  if (process.platform !== "linux") return;
  if (!iconFile || !existsSync(iconFile) || !electronBin || !existsSync(electronBin)) return;
  const dataHome = process.env.XDG_DATA_HOME || path.join(homedir(), ".local/share");
  const appsDir = path.join(dataHome, "applications");
  const hicolor = path.join(dataHome, "icons/hicolor/256x256/apps");
  mkdirSync(appsDir, { recursive: true });
  mkdirSync(hicolor, { recursive: true });
  copyFileSync(iconFile, path.join(hicolor, "lolterm.png"));
  const body = `[Desktop Entry]
Type=Application
Name=LoLTerm
Comment=Multiplexor gráfico de terminales
Exec=${quote(electronBin)} ${quote(appRoot)}
Path=${appRoot}
Icon=${iconFile}
Terminal=false
Categories=Development;
StartupWMClass=LoLTerm
StartupNotify=true
`;
  writeFileSync(path.join(appsDir, "lolterm-dev.desktop"), body);
}
