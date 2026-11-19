# Catalog

Explore's Trending and Popular creators read the catalog: the curated creators, their skills, and how many installs each skill gained over the last 7 days. This folder holds its curation only. The catalog job (`cargo run -p xtask -- catalog`, in `xtask/src/catalog.rs`) builds it, and `.github/workflows/catalog.yml` publishes it to this repository's GitHub Pages site. Decisions: [ADR 0014](../docs/adr/0014-explore-catalog-over-https.md); contract: [catalog-job.md](../docs/specs/explore-and-install/catalog-job.md).

## Curation

`curated.toml` holds one `[[creator]]` table per creator:

```toml
[[creator]]
id = "mattpocock"                 # lowercase letters, digits, and single hyphens; also the avatar's file name
name = "Matt Pocock"              # shown in Explore
github = "mattpocock"             # GitHub login; its avatar is mirrored
repositories = ["https://github.com/mattpocock/skills.git"]

[[creator.exclude]]               # optional, once per repository
repository = "https://github.com/mattpocock/skills.git"
paths = ["skills/in-progress/loop-me"]   # skill folders left out
```

A repository is `https://github.com/<owner>/<repository>.git` and is curated once. To propose a creator, or to change one, open a pull request that edits `curated.toml`. The app has no way to suggest one. A merged change publishes within minutes.

## What the job does

For each curated repository it makes a shallow, blobless clone that checks out only `SKILL.md` files and finds skills with the `skills` CLI's rules. A root `SKILL.md` is the only skill. Otherwise it looks at the root's direct folders and searches `skills/`, `skills/.curated`, `skills/.experimental`, `skills/.system`, and the agents' workspace skill directories, such as `.claude/skills` and `.agents/skills`, to depth 3. If none of that finds a skill, it searches every folder to depth 5. It leaves out a skill that install would refuse or the CLI hides: one whose frontmatter is missing or malformed, whose `name` breaks the library's name rule (at most 64 lowercase letters, digits, and single internal hyphens), whose `description` is empty, or that sets `metadata.internal: true`. It also leaves out a second skill with the same name in one repository. Each left-out skill is a warning in the run's log.

It asks skills.sh's search for each repository's all-time installs: one query of the repository name and owner, which returns at most 200 skills. An entry counts for a skill when its `source` is the repository's `owner/repo` and its `skillId` is the skill's name; a skill with no entry counts 0. It downloads the `history.json` the site published, adds tonight's totals, keeps the last 30 nights, and sets each skill's `installs_week` to its gain over the last 7 nights, or since its first night while the history is shorter. It mirrors each creator's `https://github.com/<login>.png?size=64` into `v1/avatars/`, as `.png`, or `.jpg` when GitHub serves a JPEG.

## Schedule and failures

The workflow runs nightly at 03:00 UTC, on every push to `main` that changes this folder or the job, and by hand. It deploys with the workflow's own `GITHUB_TOKEN` (`pages: write`, `id-token: write`) and never commits.

- A repository that cannot be cloned keeps its skills from the last published catalog.
- When skills.sh fails or answers in another shape, the run publishes fresh listings with the last published counts, keeps the history as it was, and then fails, so GitHub notifies the owner.
- Only a 404 for the published history starts it empty, the first run's case or a lost site. Any other failure to download it, or the catalog, stops the run before deploying, so the previous site stays up.
- `cargo run -p xtask -- catalog check site` checks the built site against catalog format v1 before it deploys, and any violation fails the run.

## The v1 contract

`https://mastro993.github.io/SkillBinder/v1/catalog.json` follows [Catalog format v1](../docs/specs/explore-and-install/design.md#catalog-format-v1): `schema: 1`, `generated_at`, `creators[]` (`id`, `name`, optional `avatar` relative to `v1/`), and `skills[]` (`slug`, `name`, `description`, `creator`, `repository`, `path`, `installs_week`), ordered by `installs_week`, highest first. It carries metadata only, never `SKILL.md` text or skill files; each skill's content stays under its repository's license and reaches the app through Git. The site also serves `v1/avatars/<creator>.png` (or `.jpg`) and `history.json`, which only the job reads. GitHub Pages serves them with `Cache-Control: max-age=600` and an `ETag`. Fields can be added to v1; a breaking change publishes `/v2/` and leaves `/v1/` in place.

`fixtures/catalog/site/` is a v1 site the job's tests build from fictional repositories; the app's catalog tests read the same sample.

## Setup

GitHub Pages must be on, with **GitHub Actions** as its source (Settings → Pages). The `github-pages` environment it creates must allow deployments from `main`.
