/* Plugins prototypes: README.md and SKILL.md texts (sample data). Load before plugins-kit.js.
 * window.PL_TEXT = { readme: {pluginId: md}, skill: {'pluginId/slug': md} }
 * A plugin or skill without its own text gets a short generated one (plugins-kit.js).
 */
window.PL_TEXT = {
  readme: {
    fstack: `# fstack

Skills for designing, building, and shipping features with coding agents: shape an idea into a spec, prototype its UI, plan the issues, and land the pull requests.

## Install

\`\`\`
claude plugin marketplace add mastro993/fstack
claude plugin install fstack@fedemas
\`\`\`

## Workflow

1. **feature-design** settles what to build, one question at a time, and writes the spec.
2. **ux-brainstorm** turns open UI questions into clickable prototypes with variants.
3. **create-issues** splits the agreed spec into vertical slices on your tracker.
4. **test-driven-development** builds each slice test-first.
5. **pr-ship** and **pr-babysit** take the pull request through CI and review to merge.

## Also included

- \`domain-driven-design\` for naming and module boundaries
- \`design-system\` for tokens and shared components
- \`comment-sweep\` and \`resolve-merge-conflicts\` for cleanup before review

## Requirements

- Git 2.39 or newer
- The \`gh\` CLI for the pull request skills

## License

MIT`,
    caveman: `# caveman

Why use many token when few token do trick. Replies drop filler and keep every technical fact.

## Skills

- \`caveman\`: compressed replies, with lite, full, and ultra levels
- \`caveman-commit\`: short Conventional Commits messages
- \`caveman-review\`: one-line review comments
- \`compress\`: rewrite memory files to save input tokens

## License

MIT`,
  },
  skill: {
    'fstack/feature-design': `---
name: feature-design
description: Use when the user has a rough idea for a feature, product change, or new capability and wants to brainstorm, shape, design, or spec it before building.
license: MIT
---
# Feature design

Turn a rough idea into a spec the owner agreed to, one decision at a time.

## Steps

1. Read the code and docs the idea touches before asking anything.
2. Ask one question at a time, with your recommended answer.
3. Record each answer in \`decisions.md\` as you go.
4. Once nothing is open, write the spec under \`docs/specs/\`.

## Rules

- **One question per turn.** Never batch questions.
- Prefer the smallest model that makes the behavior unsurprising.
- Hand UI questions to \`ux-brainstorm\` and agreed work to \`create-issues\`.

## Output

- \`docs/specs/<feature>.md\`: problem, decisions, behavior, out of scope.
- An ADR for each decision that is hard to reverse.`,
    'caveman/caveman': `---
name: caveman
description: Ultra-compressed communication mode. Cuts token usage while keeping full technical accuracy.
---
# Caveman

Talk like caveman. Keep every fact, drop every filler word.

## Levels

- **lite**: drop pleasantries and hedges.
- **full** (default): fragments, no articles.
- **ultra**: symbols and abbreviations where clear.

## Never drop

- Code, paths, commands, and error text, verbatim.
- Numbers and units.`,
  },
};
