import { spawn } from "node:child_process";

const task = process.argv[2];

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: "inherit", ...options });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} exited with ${signal ?? code}`));
    });
  });
}

async function waitForFrontend(child) {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    if (child.exitCode !== null)
      throw new Error("Vite exited before becoming ready");
    try {
      const response = await fetch("http://127.0.0.1:1420");
      if (response.ok) return;
    } catch {
      // Vite is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error("Vite did not become ready at http://127.0.0.1:1420");
}

async function dev() {
  const frontend = spawn("pnpm", ["--filter", "@skillbinder/frontend", "dev"], {
    stdio: "inherit",
  });
  const stop = () => frontend.kill("SIGTERM");
  process.once("SIGINT", stop);
  process.once("SIGTERM", stop);
  try {
    await waitForFrontend(frontend);
    await run("pnpm", ["tauri", "dev"], { cwd: "apps/tauri" });
  } finally {
    stop();
  }
}

async function build() {
  await run("node", ["scripts/contracts.mjs", "generate"]);
  await run("pnpm", ["--filter", "@skillbinder/frontend", "build"]);
  await run("pnpm", ["tauri", "build", "--no-bundle"], { cwd: "apps/tauri" });
}

try {
  if (task === "dev") await dev();
  else if (task === "build") await build();
  else throw new Error(`Unknown task: ${task ?? "missing"}`);
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
}
