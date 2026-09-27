import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";

const expected = {
  apps: ["frontend", "tauri"],
  crates: ["app", "core", "db", "platform"],
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

const commandFiles = readdirSync("apps/tauri/src", { recursive: true })
  .filter((entry) => entry.endsWith(".rs"))
  .map((entry) => path.join("apps/tauri/src", entry));
for (const file of commandFiles) {
  const attributes = readFileSync(file, "utf8").match(
    /#\[tauri::command[^\]]*\]/g,
  );
  for (const attribute of attributes ?? []) {
    if (!attribute.includes("(async)")) {
      throw new Error(
        `${file} registers a bare ${attribute}. Without (async) the command body runs on the webview thread, so a query, a file read, or a Git subprocess freezes the window for its whole duration.`,
      );
    }
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
const declared = [...grantedByHandler.values(), "pilot:default"].sort();
if (customPermissions.join("\n") !== declared.join("\n")) {
  throw new Error(
    `Main capability permissions diverged from declared app and plugin permissions: ${customPermissions.join(", ")}`,
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

// AGENTS.md names the layout, so the layout is a gate rather than a convention a reader has to spot.
const frontendSource = {
  requiredDirectories: [
    "commands",
    "components",
    "features",
    "lib",
    "routes",
    "test",
    "types",
  ],
  requiredFiles: ["main.tsx", "routeTree.gen.ts", "router.tsx", "styles.css"],
  // `test/` holds the vitest setup and its stub client, which are neither a primitive nor a type.
  allowedDirectories: [
    "commands",
    "components",
    "features",
    "lib",
    "routes",
    "test",
    "types",
  ],
};

const entries = (root) =>
  readdirSync(root, { withFileTypes: true }).map((entry) => ({
    name: entry.name,
    directory: entry.isDirectory(),
  }));

const source = entries("apps/frontend/src");
const sourceNames = new Set(source.map((entry) => entry.name));
for (const name of frontendSource.requiredDirectories) {
  if (!sourceNames.has(name))
    throw new Error(
      `apps/frontend/src/${name} is missing. AGENTS.md names it; create it or restore the move.`,
    );
}
for (const name of frontendSource.requiredFiles) {
  if (!sourceNames.has(name))
    throw new Error(`apps/frontend/src/${name} is missing.`);
}
for (const entry of source) {
  if (!entry.directory) continue;
  if (!frontendSource.allowedDirectories.includes(entry.name)) {
    throw new Error(
      `apps/frontend/src/${entry.name} is not part of the documented frontend layout. Move it under one of: ${frontendSource.allowedDirectories.join(", ")}.`,
    );
  }
}

const featureFolders = [
  "components",
  "commands",
  "hooks",
  "lib",
  "screens",
  "types",
  "__tests__",
];
for (const feature of entries("apps/frontend/src/features")) {
  if (!feature.directory) {
    throw new Error(
      `apps/frontend/src/features/${feature.name} is a file. A feature is a folder module.`,
    );
  }
  for (const entry of entries(`apps/frontend/src/features/${feature.name}`)) {
    if (!entry.directory) {
      throw new Error(
        `apps/frontend/src/features/${feature.name}/${entry.name} sits at the feature root. Feature code belongs under one of: ${featureFolders.join(", ")}.`,
      );
    }
    if (!featureFolders.includes(entry.name)) {
      throw new Error(
        `apps/frontend/src/features/${feature.name}/${entry.name} is not a documented feature folder.`,
      );
    }
  }
}

const testFiles = readdirSync("apps/frontend/src", { recursive: true }).filter(
  (entry) => /\.test\.[tj]sx?$/.test(entry),
);
for (const file of testFiles) {
  const parts = path
    .dirname(path.join("apps/frontend/src", file))
    .split(path.sep);
  if (!parts.includes("__tests__")) {
    throw new Error(
      `${path.join("apps/frontend/src", file)} is not in a __tests__ folder. AGENTS.md puts tests in the nearest folder-specific __tests__ directory.`,
    );
  }
}

const tauriSource = entries("apps/tauri/src").map((entry) => entry.name);
if (!tauriSource.includes("commands")) {
  throw new Error(
    "apps/tauri/src/commands is missing. AGENTS.md places the Tauri commands there, wired in mod.rs.",
  );
}
if (tauriSource.includes("features")) {
  throw new Error(
    "apps/tauri/src/features still exists. The Tauri commands live in apps/tauri/src/commands.",
  );
}
for (const required of [
  "apps/tauri/src/commands/mod.rs",
  "crates/app/src/lib.rs",
]) {
  try {
    readFileSync(required);
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT")
      throw new Error(`${required} is missing. AGENTS.md names it.`);
    throw error;
  }
}

for (const required of [
  "crates/db/migrations",
  "crates/db/src/repositories",
  "docs/agents",
]) {
  try {
    readdirSync(required);
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT")
      throw new Error(`${required} is missing. AGENTS.md names it.`);
    throw error;
  }
}

console.log("Architecture check passed.");
