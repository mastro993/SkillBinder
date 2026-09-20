import { readFileSync } from "node:fs";

const stylesheet = "apps/frontend/src/app/styles/index.css";
const runtimeVariables = "apps/frontend/src/app/styles/css-variables.d.ts";

const colorLiteral =
  /#[\da-fA-F]{3,8}\b|\b(?:rgba?|hsla?)\(|[^-\w](?:white|black|gray|grey|silver|maroon|red|orange|yellow|olive|lime|green|teal|aqua|blue|navy|fuchsia|purple)\b/;

const css = readFileSync(stylesheet, "utf8");
const findings = [];
const blocks = [];

for (const [index, line] of css.split("\n").entries()) {
  const trimmed = line.trim();
  const position = index + 1;
  const inDark = blocks.includes("@media (prefers-color-scheme: dark)");
  if (trimmed.endsWith("{")) {
    blocks.push(trimmed.slice(0, -1).trim());
    if (inDark && blocks.at(-1) !== ":root") {
      findings.push(
        `${position}: dark mode may only override tokens, found ${trimmed}`,
      );
    }
    continue;
  }
  if (trimmed === "}") {
    blocks.pop();
    continue;
  }
  if (!trimmed) continue;
  if (trimmed.startsWith("--")) continue;
  if (colorLiteral.test(trimmed)) {
    findings.push(
      `${position}: ${trimmed} hard-codes a colour, declare a token in :root instead`,
    );
    continue;
  }
  if (inDark)
    findings.push(
      `${position}: dark mode may only override tokens, found ${trimmed}`,
    );
}

if (findings.length) {
  throw new Error(
    `${stylesheet} bypasses the theme tokens:\n${findings.join("\n")}`,
  );
}

const declared = new Set([
  ...[...css.matchAll(/^\s*--([\w-]+):/gm)].map((match) => match[1]),
  ...[...readFileSync(runtimeVariables, "utf8").matchAll(/"--([\w-]+)"/g)].map(
    (match) => match[1],
  ),
]);
const referenced = new Set(
  [...css.matchAll(/var\(--([\w-]+)\)/g)].map((match) => match[1]),
);
const missing = [...referenced].filter((name) => !declared.has(name));
if (missing.length) {
  throw new Error(
    `${stylesheet} references tokens that are never declared: ${missing.join(", ")}`,
  );
}

const unused = [...declared].filter((name) => !referenced.has(name));
if (unused.length) {
  throw new Error(`${stylesheet} declares unused tokens: ${unused.join(", ")}`);
}

console.log(
  `Theme check passed. ${referenced.size} tokens, no hard-coded colours.`,
);
