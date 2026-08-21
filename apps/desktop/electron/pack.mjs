import { spawn } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { syncAppIcon } from "./sync-icon.mjs";

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

syncAppIcon(appRoot, { installDesktop: false });

await run("npx", ["vite", "build"], appRoot);
await run("npx", ["electron-builder", "--linux", "deb"], appRoot);

console.log("paquetes en apps/desktop/release/");
