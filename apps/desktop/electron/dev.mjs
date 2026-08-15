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
    [".", "--no-sandbox", "--disable-gpu-sandbox"],
    {
      cwd: appRoot,
      stdio: "inherit",
      env: {
        ...process.env,
        LOLTERM_CORE: path.join(target, "debug", "lolterm-core"),
        LOLTERM_DEV: "1",
        LOLTERM_URL: "http://127.0.0.1:5173",
      },
    },
  );
  electron.on("exit", (code) => {
    server.close();
    process.exit(code ?? 0);
  });
});
