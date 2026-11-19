# Catalog job

The contract slice S0 builds, from [design.md](design.md) T1 and T2 of the [Explore and install spec](../explore-and-install.md) (ADR [0014](../../adr/0014-explore-catalog-over-https.md)).

## Code

- The job is Rust, an `xtask` subcommand: `cargo run -p xtask -- catalog --out <dir>`, in `xtask/src/catalog.rs` with submodules under `xtask/src/catalog/` (curation, listing, counts, history, avatars, publish), each under the size and `cccc` limits. It is not a new workspace member, and it uses `xtask`'s own Clippy policy; `xtask verify` keeps rejecting any other `clippy.toml`.
- It runs `git` as a process, as `xtask` already does, and fetches over HTTPS with `ureq`, the same version the engine uses (ADR 0014), which adds `ureq` to the `xtask` row of `xtask/src/graph.rs`.
- `catalog/` holds data only: `curated.toml` and `README.md`. `.github/workflows/catalog.yml` builds the site into `--out`, uploads it with `actions/upload-pages-artifact`, and deploys it with `actions/deploy-pages`.
- Its tests are ordinary `xtask` tests over local `file://` repositories and saved skills.sh responses; they never touch the network. They write a fixture site that the engine's catalog tests also read (`CatalogSource::Directory`, T3), so producer and consumer share one v1 sample.

## Contract

- `catalog/curated.toml` holds one `[[creator]]` table per creator: `id`, `name`, `github = "<login>"`, and `repositories = ["https://github.com/<owner>/<repo>.git", …]`. An optional `[[creator.exclude]]` table with `repository` and `paths = [...]` leaves skills out.
- Counts: a skills.sh entry belongs to a skill when its `source` is the repository's `owner/repo` and its `skillId` is the skill's slug. An unmatched skill counts 0.
- History: the published `history.json` maps each date to `{<repository>#<path>: all-time installs}` for the last 30 nights. Each run downloads it from the live site, adds today, drops nights older than 30 days, and publishes it again. The app never reads it. Only a 404 (the first run, or a lost site) starts history empty, and trending counts from that night. Any other failure to download it, such as a 5xx, a timeout, or invalid JSON, fails the run before deploying, so the previous site and its history stay up.
- Partial failure: an unreachable curated repository keeps its entries from the last published file. When skills.sh fails, the job publishes fresh listings with the last counts, then fails the run. A failed run notifies the repository's owner through GitHub's workflow notifications.
- Avatars come from `https://github.com/<login>.png?size=64`, which needs no token. The workflow's own `GITHUB_TOKEN`, with `contents: read`, `pages: write`, and `id-token: write`, deploys Pages; it never pushes, and no other secret exists.
- GitHub Pages fixes `Cache-Control: max-age=600`, so freshness is not configurable; the app's conditional GET makes most refreshes cost one empty response.
