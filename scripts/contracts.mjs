import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";

const task = process.argv[2];
const generated = path.resolve("apps/frontend/src/types");

function snapshot() {
  return new Map(
    readdirSync(generated, { withFileTypes: true })
      .filter(
        (entry) =>
          entry.isFile() &&
          entry.name.endsWith(".ts") &&
          entry.name !== "index.ts",
      )
      .map((entry) => [
        entry.name,
        readFileSync(path.join(generated, entry.name), "utf8"),
      ]),
  );
}

function equal(left, right) {
  return (
    left.size === right.size &&
    [...left].every(([name, content]) => right.get(name) === content)
  );
}

if (task !== "generate" && task !== "check") {
  throw new Error("Usage: node scripts/contracts.mjs <generate|check>");
}

const before = task === "check" ? snapshot() : undefined;
execFileSync(
  "cargo",
  [
    "test",
    "-p",
    "skillbinder",
    "--lib",
    "transport::tests::export_bindings",
    "--",
    "--ignored",
  ],
  {
    stdio: "inherit",
  },
);

if (before && !equal(before, snapshot())) {
  console.error(
    "Generated IPC contracts are stale. Run `pnpm contracts:generate`.",
  );
  process.exitCode = 1;
}
