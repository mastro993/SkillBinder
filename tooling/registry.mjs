import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(new URL("..", import.meta.url).pathname);
const registryPath = resolve(root, "libs/core/src/discovery/registry.json");
const docsPath = resolve(root, "docs/agent-support.md");

const read = (path) => readFileSync(path, "utf8");
const registry = () => JSON.parse(read(registryPath));
const fixturePath = (commit, file) =>
  resolve(
    root,
    "tests/fixtures/upstream-skills-cli",
    commit,
    file.split("/").at(-1),
  );

function fail(message) {
  throw new Error(message);
}

function extractIds(typesText) {
  const union = typesText.match(/export type AgentType =([\s\S]*?);/);
  if (!union) fail("Could not find AgentType union");
  return [...union[1].matchAll(/\|\s*'([^']+)'/g)].map((match) => match[1]);
}

function extractUpstreamAgents(source) {
  const object = source.match(
    /export const agents: Record<AgentType, AgentConfig> = \{([\s\S]*?)^\};/m,
  );
  if (!object) fail("Could not find upstream agents object");
  const entries = [];
  const pattern =
    /^ {2}(?:'([^']+)'|([A-Za-z0-9-]+)): \{([\s\S]*?)(?=^ {2}(?:'[^']+'|[A-Za-z0-9-]+): \{|^};|(?=(?![\s\S])))/gm;
  for (const match of object[1].matchAll(pattern)) {
    const id = match[1] ?? match[2];
    const body = match[3];
    const field = (name) => {
      const value = body.match(new RegExp(`^    ${name}: (.+?),$`, "m"));
      if (!value) fail(`Missing ${name} for upstream agent ${id}`);
      return value[1];
    };
    entries.push({
      id,
      displayName: parseString(field("displayName"), id, "displayName"),
      projectSkillsDir: parseString(field("skillsDir"), id, "skillsDir"),
      globalRoots: translateGlobalPath(field("globalSkillsDir"), id),
    });
  }
  return entries;
}

function parseString(value, id, field) {
  const match = value.match(/^'([^']*)'$/);
  if (!match) fail(`Unsupported ${field} expression for ${id}: ${value}`);
  return match[1];
}

function translateGlobalPath(expression, id) {
  if (expression === "undefined") return [];
  if (expression === "getOpenClawGlobalSkillsDir()") {
    return ["~/.openclaw/skills", "~/.clawdbot/skills", "~/.moltbot/skills"];
  }
  const call = expression.match(/^join\((.*)\)$/);
  if (!call)
    fail(`Unsupported globalSkillsDir expression for ${id}: ${expression}`);
  const parts = call[1].split(",").map((part) => part.trim());
  const homes = {
    home: "~",
    configHome: "${XDG_CONFIG_HOME:-~/.config}",
    codexHome: "${CODEX_HOME:-~/.codex}",
    claudeHome: "${CLAUDE_CONFIG_DIR:-~/.claude}",
    vibeHome: "${VIBE_HOME:-~/.vibe}",
    hermesHome: "${HERMES_HOME:-~/.hermes}",
    autohandHome: "${AUTOHAND_HOME:-~/.autohand}",
    grokHome: "${GROK_HOME:-~/.grok}",
    sarvamHome: "${SARVAM_HOME:-~/.sarvam}",
  };
  const home = homes[parts.shift()];
  if (
    !home ||
    parts.length === 0 ||
    parts.some((part) => !/^'[^']*'$/.test(part))
  ) {
    fail(`Unsupported globalSkillsDir expression for ${id}: ${expression}`);
  }
  return [home + parts.map((part) => `/${part.slice(1, -1)}`).join("")];
}

function digest(path) {
  return createHash("sha256").update(read(path)).digest("hex");
}

function compare(actual, expected, id, field) {
  const actualText = JSON.stringify(actual);
  const expectedText = JSON.stringify(expected);
  if (actualText !== expectedText) {
    fail(
      `${id} ${field} mismatch: registry=${actualText}; upstream=${expectedText}`,
    );
  }
}

function generatedDocs(data) {
  const { upstream, agents } = data;
  const rows = agents.map((agent) => {
    const roots =
      agent.globalRoots.length === 0 ? "none" : agent.globalRoots.join("<br>");
    return `| ${agent.id} | ${agent.displayName} | \`${agent.projectSkillsDir}\` | ${roots} | ${agent.status} |`;
  });
  return `# Agent support

Pinned compatibility data for \`${upstream.repository}\` at commit \`${upstream.commit}\`.

## Review record

- Review date: ${upstream.retrievedAt}
- Upstream snapshot: \`tests/fixtures/upstream-skills-cli/${upstream.commit}/\`
- Reviewed file digests:
  - \`src/agents.ts\`: \`${upstream.files["src/agents.ts"]}\`
  - \`src/types.ts\`: \`${upstream.files["src/types.ts"]}\`
- Status values distinguish documented facts from path-tested and runtime-tested facts. Every entry below is currently \`documented\` only; no path or agent-runtime test is claimed.

## Registry

| ID | Display name | Project skills directory | Global roots | Status |
| --- | --- | --- | --- | --- |
${rows.join("\n")}

Several agents read the same physical directory, including \`.agents/skills\`. Discovery deduplicates shared physical paths while retaining all reader IDs.

OpenClaw upstream chooses one existing directory from \`.openclaw/skills\`, \`.clawdbot/skills\`, and \`.moltbot/skills\`. SkillBinder scans every existing candidate, so skills in a legacy directory remain visible.

Regenerate this document with:

\`node tooling/registry.mjs docs\`
`;
}

function check() {
  const data = registry();
  const { upstream, agents } = data;
  for (const [file, expected] of Object.entries(upstream.files)) {
    const actual = digest(fixturePath(upstream.commit, file));
    if (actual !== expected)
      fail(`${file} digest mismatch: registry=${expected}; snapshot=${actual}`);
  }
  const commitRoot = resolve(
    root,
    "tests/fixtures/upstream-skills-cli",
    upstream.commit,
  );
  const ids = extractIds(read(resolve(commitRoot, "types.ts")));
  const upstreamAgents = extractUpstreamAgents(
    read(resolve(commitRoot, "agents.ts")),
  );
  const registryIds = agents.map((agent) => agent.id);
  if (new Set(ids).size !== ids.length)
    fail("Upstream AgentType union contains duplicate IDs");
  if (new Set(registryIds).size !== registryIds.length)
    fail("Registry contains duplicate IDs");
  compare(
    [...upstreamAgents].map((agent) => agent.id).sort(),
    [...ids].sort(),
    "upstream",
    "agent object id set",
  );
  compare([...registryIds].sort(), [...ids].sort(), "registry", "id set");
  for (const upstreamAgent of upstreamAgents) {
    const agent = agents.find((candidate) => candidate.id === upstreamAgent.id);
    if (!agent) fail(`${upstreamAgent.id} missing from registry`);
    compare(
      agent.displayName,
      upstreamAgent.displayName,
      agent.id,
      "displayName",
    );
    compare(
      agent.projectSkillsDir,
      upstreamAgent.projectSkillsDir,
      agent.id,
      "projectSkillsDir",
    );
    compare(
      agent.globalRoots,
      upstreamAgent.globalRoots,
      agent.id,
      "globalRoots",
    );
  }
  const expectedDocs = generatedDocs(data);
  const actualDocs = read(docsPath);
  if (actualDocs !== expectedDocs)
    fail(`docs/agent-support.md differs; run node tooling/registry.mjs docs`);
  console.log(`registry check passed: ${ids.length} upstream agents verified`);
}

function docs() {
  writeFileSync(docsPath, generatedDocs(registry()));
  console.log(
    `wrote docs/agent-support.md: ${registry().agents.length} agents`,
  );
}

const command = process.argv[2];
try {
  if (command === "check") check();
  else if (command === "docs") docs();
  else fail("Usage: node tooling/registry.mjs <check|docs>");
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
}
