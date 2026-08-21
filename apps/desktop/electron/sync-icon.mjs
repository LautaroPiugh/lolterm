import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

export function magickBin() {
  const which = spawnSync("bash", ["-lc", "command -v magick || command -v convert"], { encoding: "utf8" });
  return which.status === 0 ? which.stdout.trim().split("\n")[0] : "";
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
  const magick = magickBin();
  const dest256 = path.join(iconDir, "256x256.png");
  let needResize = true;
  try {
    needResize = !(existsSync(dest256) && statSync(dest256).mtimeMs >= statSync(iconSrc).mtimeMs);
  } catch {
    needResize = true;
  }
  if (magick && needResize) {
    for (const size of [16, 24, 32, 48, 64, 128, 256, 512, 1024]) {
      spawnSync(magick, [iconSrc, "-resize", `${size}x${size}`, path.join(iconDir, `${size}x${size}.png`)], {
        stdio: "ignore",
      });
    }
  }
  const sized512 = existsSync(path.join(iconDir, "512x512.png"))
    ? path.join(iconDir, "512x512.png")
    : iconSrc;
  const sized256 = existsSync(path.join(iconDir, "256x256.png"))
    ? path.join(iconDir, "256x256.png")
    : sized512;
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

const here = path.dirname(fileURLToPath(import.meta.url));
if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(here, "sync-icon.mjs")) {
  const appRoot = path.join(here, "..");
  syncAppIcon(appRoot, { installDesktop: true });
  console.log("icono sincronizado desde public/icon.png");
}
