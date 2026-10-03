import { readFileSync, readdirSync, mkdtempSync, openSync, closeSync, rmSync } from "node:fs";
import { resolve, relative } from "node:path";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";

const root = resolve(import.meta.dirname, "..");
const allowed = {
  "skillbinder": ["skillbinder-app", "skillbinder-core", "skillbinder-platform", "skillbinder-ui"],
  "skillbinder-core": [],
  "skillbinder-db": ["skillbinder-core"],
  "skillbinder-platform": ["skillbinder-core"],
  "skillbinder-app": ["skillbinder-core", "skillbinder-db", "skillbinder-platform"],
  "skillbinder-ui": ["skillbinder-app"],
};
for (const [directory, expected] of Object.entries({apps: ["desktop"], crates: ["app", "core", "db", "platform", "ui"]})) {
  const actual = readdirSync(resolve(root, directory), {withFileTypes: true})
    .filter((entry) => entry.isDirectory()).map((entry) => entry.name).sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(`${directory}: unexpected workspace roots: ${actual}`);
}
// File-backed output also works in environments that disallow socket-backed child pipes.
const temporary = mkdtempSync(resolve(tmpdir(), "skillbinder-metadata-"));
let metadata;
try {
  const output = openSync(resolve(temporary, "metadata.json"), "w");
  const errors = openSync(resolve(temporary, "errors.log"), "w");
  let command;
  try {
    command = spawnSync("cargo", ["metadata", "--no-deps", "--format-version", "1", "--locked", "--offline"], {cwd: root, stdio: ["ignore", output, errors]});
  } finally {
    closeSync(output);
    closeSync(errors);
  }
  if (command.error) throw command.error;
  if (command.status !== 0) throw new Error(readFileSync(resolve(temporary, "errors.log"), "utf8"));
  metadata = JSON.parse(readFileSync(resolve(temporary, "metadata.json"), "utf8"));
} finally {
  rmSync(temporary, {recursive: true, force: true});
}
for (const pkg of metadata.packages) {
  const dependencies = allowed[pkg.name];
  if (!dependencies) throw new Error(`Unexpected workspace package: ${pkg.name}`);
  for (const dependency of pkg.dependencies) {
    if (dependency.name.startsWith("skillbinder-") && !dependencies.includes(dependency.name)) {
      throw new Error(`${pkg.name} must not depend on ${dependency.name}`);
    }
    if (dependency.name.startsWith("gpui") && !["skillbinder-ui", "skillbinder"].includes(pkg.name)) {
      throw new Error(`UI dependency in ${pkg.name}`);
    }
  }
  const manifest = readFileSync(pkg.manifest_path, "utf8");
  if (/^skillbinder-[\w-]+\s*=/m.test(manifest)) throw new Error(`Inherit internal workspace dependencies in ${pkg.name}`);
  for (const field of ["version", "edition", "license", "rust-version", "publish"]) {
    if (!manifest.includes(`${field}.workspace = true`)) throw new Error(`Inherit workspace ${field} in ${pkg.name}`);
  }
}
function sources(directory) {
  return readdirSync(directory, {withFileTypes: true}).flatMap((entry) => {
    const path = resolve(directory, entry.name);
    return entry.isDirectory() ? sources(path) : path.endsWith(".rs") ? [path] : [];
  });
}
for (const path of sources(resolve(root, "apps/desktop"))) {
  if (/impl\s+(?:gpui::)?Render(?:Once)?\b|\bdiv\(/.test(readFileSync(path, "utf8"))) {
    throw new Error(`Define UI components in crates/ui: ${relative(root, path)}`);
  }
}
console.log("Architecture: PASS");
