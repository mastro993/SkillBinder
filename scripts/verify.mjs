import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
const cwd = resolve(import.meta.dirname, "..");
const checks = [
  [process.execPath, "scripts/check-architecture.mjs"],
  [process.execPath, "scripts/registry.mjs", "check"],
  ["cargo", "fmt", "--all", "--check"],
  ["cargo", "clippy", "--workspace", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"],
  ["cargo", "test", "--workspace", "--locked"],
];
for (const [command, ...args] of checks) {
  const result = spawnSync(command, args, {cwd, stdio: "inherit"});
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
