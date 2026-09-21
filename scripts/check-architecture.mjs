import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";

const expected = {
  apps: ["frontend", "tauri"],
  libs: ["core", "db", "platform"],
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
  "libs/ui",
  "libs/contracts",
  "libs/desktop-client",
  "libs/ipc-contracts",
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
const customPermissions = capability.permissions
  .filter((permission) => !permission.startsWith("core:"))
  .sort();
const expectedPermissions = [
  "allow-discovery-cancel",
  "allow-discovery-results",
  "allow-discovery-start",
  "allow-git-environment-verify",
  "allow-imports-apply",
  "allow-imports-prepare",
  "allow-library-list",
  "allow-onboarding-complete-local",
  "allow-onboarding-progress-update",
  "allow-roots-list",
  "allow-roots-pick",
  "allow-roots-register",
  "allow-roots-remove",
  "allow-roots-update",
  "allow-system-bootstrap",
];
if (customPermissions.join("\n") !== expectedPermissions.join("\n")) {
  throw new Error(
    `Unexpected native command permissions: ${customPermissions.join(", ")}`,
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
