import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const hereDir = path.dirname(fileURLToPath(import.meta.url));

/** Binario de Electron (dependencia del proyecto); evita depender de ImageMagick. */
export function electronBin() {
  try {
    return require("electron");
  } catch {
    return "";
  }
}

/** Copia `public/icon.png` a `build/` y genera tamaños para el `.desktop`/hicolor. */
export function syncAppIcon(appRoot, { installDesktop = false } = {}) {
  const iconSrc = path.join(appRoot, "public", "icon.png");
  if (!existsSync(iconSrc)) return;
  const iconDir = path.join(appRoot, "build", "icons");
  mkdirSync(path.join(appRoot, "build"), { recursive: true });
  mkdirSync(iconDir, { recursive: true });
  copyFileSync(iconSrc, path.join(appRoot, "build", "icon.png"));
  copyFileSync(iconSrc, path.join(iconDir, "icon.png"));

  const dest256 = path.join(iconDir, "256x256.png");
  let needResize = true;
  try {
    needResize = !(existsSync(dest256) && statSync(dest256).mtimeMs >= statSync(iconSrc).mtimeMs);
  } catch {
    needResize = true;
  }
  if (needResize) {
    // Fallar ruidosamente: un ícono desactualizado en el .deb es peor que cortar el pack.
    const electron = electronBin();
    if (!electron) throw new Error("no se encontró el binario de Electron para regenerar íconos");
    const script = path.join(hereDir, "resize-icon.mjs");
    const result = spawnSync(electron, [script, iconSrc, iconDir], {
      stdio: "inherit",
      env: { ...process.env, ELECTRON_DISABLE_SANDBOX: "1" },
    });
    if (result.status !== 0) {
      throw new Error(`resize-icon falló con código ${result.status}`);
    }
  }

  const sized512 = path.join(iconDir, "512x512.png");
  const sized256 = path.join(iconDir, "256x256.png");
  copyFileSync(sized512, path.join(iconDir, "lolterm.png"));
  if (installDesktop && process.platform === "linux") {
    const dataHome = process.env.XDG_DATA_HOME || path.join(homedir(), ".local/share");
    for (const [dir, src] of [
      ["256x256", sized256],
      ["512x512", sized512],
    ]) {
      const destDir = path.join(dataHome, "icons/hicolor", dir, "apps");
      mkdirSync(destDir, { recursive: true });
      copyFileSync(src, path.join(destDir, "lolterm.png"));
    }
    spawnSync("gtk-update-icon-cache", ["-f", "-t", path.join(dataHome, "icons/hicolor")], {
      stdio: "ignore",
    });
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(hereDir, "sync-icon.mjs")) {
  const appRoot = path.join(hereDir, "..");
  syncAppIcon(appRoot, { installDesktop: true });
  console.log("icono sincronizado desde public/icon.png");
}
