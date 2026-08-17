import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const here = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.join(here, "..");
const repoRoot = path.join(appRoot, "..", "..");

const cargo = spawn("cargo", ["build", "-p", "lolterm-core"], {
  cwd: repoRoot,
  stdio: "inherit",
  env: process.env,
});

function isChromiumNoise(line) {
  return (
    line.includes("GetVSyncParametersIfAvailable") ||
    line.includes("Add chromium/from-privileged to kAtomsToCache")
  );
}

function forwardFiltered(stream, dest) {
  let buf = "";
  stream.setEncoding("utf8");
  stream.on("data", (chunk) => {
    buf += chunk;
    let idx;
    while ((idx = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, idx);
      buf = buf.slice(idx + 1);
      if (!line.trim() || isChromiumNoise(line)) continue;
      dest.write(`${line}\n`);
    }
  });
  stream.on("end", () => {
    if (buf.trim() && !isChromiumNoise(buf)) dest.write(buf);
  });
}

cargo.on("exit", async (code) => {
  if (code !== 0) process.exit(code ?? 1);
  const server = await createServer({
    root: appRoot,
    configFile: path.join(appRoot, "vite.config.ts"),
  });
  await server.listen();
  const target = process.env.CARGO_TARGET_DIR || path.join(repoRoot, "target");
  const electron = spawn(
    path.join(appRoot, "node_modules", ".bin", "electron"),
    [".", "--no-sandbox", "--disable-gpu-sandbox", "--log-level=3", "--class=LoLTerm"],
    {
      cwd: appRoot,
      stdio: ["inherit", "inherit", "pipe"],
      env: {
        ...process.env,
        LOLTERM_CORE: path.join(target, "debug", "lolterm-core"),
        LOLTERM_DEV: "1",
        LOLTERM_URL: "http://127.0.0.1:5173",
      },
    },
  );
  forwardFiltered(electron.stderr, process.stderr);
  electron.on("exit", (code) => {
    server.close();
    process.exit(code ?? 0);
  });
});
