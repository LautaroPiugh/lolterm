import { spawn } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
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

const pkg = JSON.parse(readFileSync(path.join(appRoot, "package.json"), "utf8"));
const metainfoDir = path.join(appRoot, "build", "metainfo");
mkdirSync(metainfoDir, { recursive: true });
writeFileSync(
  path.join(metainfoDir, "lolterm.metainfo.xml"),
  readFileSync(path.join(appRoot, "build", "metainfo.template.xml"), "utf8")
    .replaceAll("{{VERSION}}", pkg.version)
    .replaceAll("{{DATE}}", new Date().toISOString().slice(0, 10)),
);

await run("pnpm", ["exec", "vite", "build"], appRoot);
await run("pnpm", ["exec", "electron-builder", "--linux", "deb", "rpm", "--publish", "never"], appRoot);

console.log("paquetes en apps/desktop/release/");
