import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";

const expected = {
  apps: ["frontend", "tauri"],
  crates: ["core", "db", "platform"],
};

for (const [root, allowed] of Object.entries(expected)) {
  const actual = readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  if (actual.join("\n") !== [...allowed].sort().join("\n")) {
    throw new Error(
      `${root}/ must contain only: ${allowed.join(", ")}. Found: ${actual.join(", ")}`,
    );
  }
}

const forbidden = [
  "crates/ui",
  "crates/contracts",
  "crates/desktop-client",
  "crates/ipc-contracts",
];
for (const directory of forbidden) {
  try {
    readdirSync(path.resolve(directory));
    throw new Error(`Forbidden package exists: ${directory}`);
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT")
      continue;
    throw error;
  }
}

const capability = JSON.parse(
  readFileSync("apps/tauri/capabilities/main.json", "utf8"),
);

// The handlers are the source of truth; a permission and a capability entry must follow each one.
const handlerBlocks = [
  ...readFileSync("apps/tauri/src/lib.rs", "utf8").matchAll(
    /generate_handler!\[([^\]]*)\]/g,
  ),
];
if (handlerBlocks.length !== 1) {
  throw new Error(
    `Expected one generate_handler! block in apps/tauri/src/lib.rs, found ${handlerBlocks.length}`,
  );
}
const handlers = handlerBlocks[0][1]
  .split(",")
  .map((handler) => handler.trim().split("::").pop())
  .filter(Boolean);
if (handlers.length === 0) {
  throw new Error("generate_handler! listed no commands");
}

const permissionBlocks = [
  ...readFileSync("apps/tauri/permissions/default.toml", "utf8").matchAll(
    /\[\[permission\]\]\nidentifier = "([^"]+)"[\s\S]*?commands\.allow = \[([^\]]*)\]/g,
  ),
];
if (permissionBlocks.length !== handlers.length) {
  throw new Error(
    `${handlers.length} handlers but ${permissionBlocks.length} permission definitions`,
  );
}

const grantedByHandler = new Map();
for (const [, identifier, commands] of permissionBlocks) {
  const allowed = commands
    .split(",")
    .map((command) => command.trim().replaceAll('"', ""))
    .filter(Boolean);
  if (allowed.length !== 1) {
    throw new Error(`${identifier} must allow exactly one command`);
  }
  const [command] = allowed;
  if (!handlers.includes(command)) {
    throw new Error(
      `${identifier} allows ${command}, which no handler registers`,
    );
  }
  if (identifier !== `allow-${command.replaceAll("_", "-")}`) {
    throw new Error(`${identifier} does not name its command ${command}`);
  }
  grantedByHandler.set(command, identifier);
}
for (const handler of handlers) {
  if (!grantedByHandler.has(handler)) {
    throw new Error(`Handler ${handler} has no permission in default.toml`);
  }
}

const customPermissions = capability.permissions
  .filter((permission) => !permission.startsWith("core:"))
  .sort();
const declared = [...grantedByHandler.values()].sort();
if (customPermissions.join("\n") !== declared.join("\n")) {
  throw new Error(
    `Main capability permissions diverged from permissions/default.toml: ${customPermissions.join(", ")}`,
  );
}
if (
  capability.permissions.some((permission) => permission.startsWith("core:"))
) {
  throw new Error("Main capability must not grant core permissions");
}
if (capability.windows.join("\n") !== "main" || capability.remote) {
  throw new Error(
    "Native command capability must target only the local main window",
  );
}

console.log("Architecture check passed.");
