import { spawn, spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.join(here, "..");
const repoRoot = path.join(appRoot, "..", "..");

function run(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: "inherit", env: process.env });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} ${args.join(" ")} → ${code}`));
    });
  });
}

const target = process.env.CARGO_TARGET_DIR || path.join(repoRoot, "target");
const core = path.join(target, "release", "lolterm-core");
const sidecarDir = path.join(appRoot, "sidecar");
const sidecar = path.join(sidecarDir, "lolterm-core");

await run("cargo", ["build", "-p", "lolterm-core", "--release"], repoRoot);
if (!existsSync(core)) {
  throw new Error(`no se generó ${core}`);
}
mkdirSync(sidecarDir, { recursive: true });
copyFileSync(core, sidecar);
chmodSync(sidecar, 0o755);

const iconSrc = path.join(appRoot, "public", "icon.png");
const iconDir = path.join(appRoot, "build", "icons");
mkdirSync(iconDir, { recursive: true });
if (existsSync(iconSrc)) {
  copyFileSync(iconSrc, path.join(appRoot, "build", "icon.png"));
  copyFileSync(iconSrc, path.join(iconDir, "icon.png"));
  const whichMagick = spawnSync("bash", ["-lc", "command -v magick || command -v convert"], { encoding: "utf8" });
  const magick = whichMagick.status === 0 ? whichMagick.stdout.trim().split("\n")[0] : "";
  if (magick) {
    for (const size of [16, 24, 32, 48, 64, 128, 256, 512, 1024]) {
      spawnSync(magick, [iconSrc, "-resize", `${size}x${size}`, path.join(iconDir, `${size}x${size}.png`)], {
        stdio: "inherit",
      });
    }
  }
  const sized = existsSync(path.join(iconDir, "512x512.png"))
    ? path.join(iconDir, "512x512.png")
    : iconSrc;
  copyFileSync(sized, path.join(iconDir, "lolterm.png"));
}

await run("npx", ["vite", "build"], appRoot);
await run("npx", ["electron-builder", "--linux", "deb"], appRoot);

console.log("paquetes en apps/desktop/release/");
