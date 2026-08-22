// Corre dentro de Electron: recibe <src> <destDir> y genera {size}x{size}.png.
import { nativeImage } from "electron";
import { writeFileSync } from "node:fs";
import path from "node:path";

const [src, destDir] = process.argv.slice(2);
if (!src || !destDir) {
  console.error("uso: electron resize-icon.mjs <src> <destDir>");
  process.exit(1);
}

const img = nativeImage.createFromPath(src);
if (img.isEmpty()) {
  console.error(`no se pudo leer ${src}`);
  process.exit(1);
}

for (const size of [16, 24, 32, 48, 64, 128, 256, 512, 1024]) {
  const resized = img.resize({ width: size, height: size });
  writeFileSync(path.join(destDir, `${size}x${size}.png`), resized.toPNG());
}
process.exit(0);
