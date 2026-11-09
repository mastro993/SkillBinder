#!/bin/bash
# Native fixture mirroring the prototypes' data (18 skills, 4 agents, 3 workspaces); README.md
# in this folder gives the whole procedure.
#
#   make-fixture.sh <root>           stage "init": home, workspaces, Git sources, skills locks
#   make-fixture.sh <root> finish    after the UI setup (app quit): the 2 Discovery-only skills
#   make-fixture.sh <root> problems  optional: the EDITED and FOREIGN copies (file edits only)
#
# <root> must be absolute and is <fixture dir>/root: the app stores absolute paths (workspaces,
# file:// sources) in its database and records, so the seed only works at that root.
set -euo pipefail

root=${1:?usage: make-fixture.sh <absolute root> [init|finish|problems]}
stage=${2:-init}
case $root in /*) ;; *) echo "root must be absolute" >&2; exit 2 ;; esac
home=$root/home
code=$home/code
sources=$root/sources

# slug|description|folder (folder is filed in the UI, listed here for the README)
SKILLS='a11y-audit|Audit pages for WCAG accessibility issues.|Web
api-docs|Generate OpenAPI reference docs from route handlers.|Backend
changelog|Keep CHANGELOG.md in Keep a Changelog format.|Writing
code-review|Review a diff for correctness bugs and style issues.|
copy-editor|Edit product copy for clarity and tone.|Writing
data-viz|Pick the right chart and build it accessibly.|Web
docker-compose|Author and debug docker-compose files.|Backend
e2e-playwright|Author and debug Playwright end-to-end tests.|Web
frontend-design|Distinctive, intentional visual design for web UIs.|Web
git-commit|Write concise Conventional Commits messages.|
k8s-debug|Diagnose failing Kubernetes pods and services.|Backend
pdf-tools|Extract text, merge files, and fill PDF forms.|
prompt-tuning|Tighten prompts and test them against examples.|
release-notes|Draft release notes from merged pull requests.|Writing
sql-helper|Write and explain SQL for Postgres and SQLite.|Backend
storybook|Write Storybook stories for React components.|Web
tailwind-helper|Compose Tailwind classes and theme tokens.|Web
test-writer|Write focused unit tests for the code you changed.|'

desc() { printf '%s\n' "$SKILLS" | awk -F'|' -v s="$1" '$1 == s { print $2 }'; }

# write_skill <dir> <slug> [description]: <dir>/<slug>/SKILL.md, the content every copy shares.
write_skill() {
  local d=${3:-$(desc "$2")}
  [ -n "$d" ] || { echo "unknown skill $2" >&2; exit 1; }
  mkdir -p "$1/$2"
  printf -- '---\nname: %s\ndescription: %s\n---\n\n# %s\n\n%s\n' "$2" "$d" "$2" "$d" > "$1/$2/SKILL.md"
}

# Agents' global skill directories (registry-79.json): Claude Code, Cursor, the universal
# agents (~/.agents/skills) and Windsurf. Onboarding imports everything found here into Default,
# active for the agents whose directory held it; the UI then unticks the 5 inactive skills.
init_home() {
  local claude=$home/.claude/skills cursor=$home/.cursor/skills
  local agents=$home/.agents/skills windsurf=$home/.codeium/windsurf/skills s
  mkdir -p "$claude" "$cursor" "$agents" "$windsurf"
  for s in code-review git-commit pdf-tools test-writer frontend-design release-notes \
    a11y-audit copy-editor data-viz k8s-debug prompt-tuning; do write_skill "$claude" $s; done
  for s in code-review test-writer frontend-design; do write_skill "$cursor" $s; done
  for s in code-review git-commit changelog; do write_skill "$agents" $s; done
  write_skill "$windsurf" code-review
}

# Workspaces (~/code/<name>, so the app shows "~/code/acme-web" like data.js). A workspace uses
# the skill directories that exist when it is added: acme-web .claude + .agents, api-server
# .claude, docs-site none. Copies of library skills are adopted; the rest are imported from
# Discovery, which makes each active for exactly the directories holding it.
init_workspaces() {
  local w s
  for w in acme-web api-server docs-site; do
    mkdir -p "$code/$w"; printf '# %s\n' "$w" > "$code/$w/README.md"
  done
  for s in code-review storybook e2e-playwright tailwind-helper; do
    write_skill "$code/acme-web/.claude/skills" $s
  done
  for s in code-review e2e-playwright frontend-design; do
    write_skill "$code/acme-web/.agents/skills" $s
  done
  for s in sql-helper docker-compose api-docs; do
    write_skill "$code/api-server/.claude/skills" $s
  done
  ln -sfn home/code "$root/projects"
}

# One local bare Git source for the 3 data.js hasUpdate skills. Commit 1 holds exactly the
# imported content; commit 2 (HEAD) changes it, so "Check for updates" reports exactly those 3.
init_sources() {
  local work=$root/.source-work s
  local git=(git -c user.name=fixture -c user.email=fixture@example.invalid -c init.defaultBranch=main)
  export GIT_AUTHOR_DATE=2026-11-01T12:00:00Z GIT_COMMITTER_DATE=2026-11-01T12:00:00Z
  rm -rf "$work"; mkdir -p "$sources" "$work"
  "${git[@]}" init -q --bare "$sources/skills.git"
  "${git[@]}" init -q "$work"
  for s in k8s-debug pdf-tools storybook; do write_skill "$work/skills" $s; done
  "${git[@]}" -C "$work" add -A && "${git[@]}" -C "$work" commit -qm "Publish skills"
  export GIT_AUTHOR_DATE=2026-11-05T12:00:00Z GIT_COMMITTER_DATE=2026-11-05T12:00:00Z
  for s in k8s-debug pdf-tools storybook; do
    printf '\n## Changes\n\nNewer version from the source.\n' >> "$work/skills/$s/SKILL.md"
  done
  "${git[@]}" -C "$work" commit -qam "Update skills"
  "${git[@]}" -C "$work" push -q "$sources/skills.git" main
  rm -rf "$work"
  local url="file://$sources/skills.git"
  entry() { printf '"%s":{"source":"%s","sourceType":"git","sourceUrl":"%s","skillPath":"skills/%s/SKILL.md"}' "$1" "$url" "$url" "$1"; }
  # Global lock (copies in agents' global directories) and acme-web's project lock.
  printf '{"version":3,"skills":{%s,%s}}\n' "$(entry k8s-debug)" "$(entry pdf-tools)" \
    > "$home/.agents/.skill-lock.json"
  printf '{"version":1,"skills":{%s}}\n' "$(entry storybook)" > "$code/acme-web/skills-lock.json"
}

case $stage in
  init)
    [ ! -e "$root/home" ] || { echo "$root already holds a fixture" >&2; exit 1; }
    mkdir -p "$root/data"
    init_home; init_workspaces; init_sources
    echo "fixture ready: $root" ;;
  finish)
    # data.js discoveryCount 2: two skills found in acme-web that are not in the library.
    write_skill "$code/acme-web/.claude/skills" figma-export "Export Figma frames as web-ready assets."
    write_skill "$code/acme-web/.claude/skills" react-hooks "Write and test custom React hooks." ;;
  problems)
    # data.js EDITED: release-notes@claude, storybook@acme-web/claude; FOREIGN: tailwind-helper@cursor.
    printf '\nEdited here.\n' >> "$home/.claude/skills/release-notes/SKILL.md"
    printf '\nEdited here.\n' >> "$code/acme-web/.claude/skills/storybook/SKILL.md"
    write_skill "$home/.cursor/skills" tailwind-helper "My own Tailwind notes." ;;
  *) echo "unknown stage $stage" >&2; exit 2 ;;
esac
