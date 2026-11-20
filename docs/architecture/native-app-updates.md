# Native app updates

App updates tell a running SkillBinder about a newer release on its channel ([spec](../specs/app-updates.md), [design](../specs/app-updates/design.md), ADR 0017, ADR 0018; [feature](../features/native/settings-agents.md#settings--updates)). Slice S1 (#169) checks and offers every newer version as download-only; downloading, staging, and installing in place come in later slices.

## Ownership

| Where | What |
| --- | --- |
| `crates/proto/src/app_update.rs` | `Channel { Stable, Beta }`; `AppVersion`, which parses only `X.Y.Z` and `X.Y.Z-beta.N` and orders a beta before its release; `AppUpdateState` (`Off`, `Idle` with the last Check now's `CheckFailure`, `Checking`, `DownloadOnly`); and `AppUpdateSnapshot`, carried on `AppSnapshot.app_update`. |
| `crates/engine/src/app_update.rs` + `app_update/` | The check on a `skillbinder-app-update` thread, never the worker, one at a time; `manifest.rs` verifies the signature and reads the manifest. The channel, the switch, and the last check sit in a shared in-memory record that the worker's snapshot and the client read without waiting. |
| `crates/client/src/app_update.rs` | Check now always checks; while automatic checks are on, the client also checks at launch, every hour on its runtime, after a channel change, and when the switch is turned on. It publishes the record from its 250-millisecond poll and when a check ends. |
| `crates/ui` | `native_view/settings/updates.rs` (the section) and `native_view/app_update_notice.rs` (the notice, dismissed in memory). |

## Checks

`EngineConfig::app_update` is `Option<UpdateSource>`: the site laid out like the catalog's and the minisign public key in base64. `EngineConfig::platform` sets it only in a build stamped with `SKILLBINDER_RELEASE`, which `build.yml` sets for a stamped version, so an unstamped build's state is `Off` and it never makes a request. Its site is the catalog's base URL and its key the compiled-in release key. `EngineConfig::isolated` leaves it `None`; native-test builds take `--update-dir <dir>`, a folder laid out like the site with the key in its `key.pub`, and `--app-version <version>` in place of the running one (`EngineConfig::app_version`). The synthetic world's `app-updates/` is one, signed with a key made when the world is built.

A check GETs `updates/v1/<channel>.json` through the catalog's HTTP client (`catalog::Fetcher`: `SkillBinder/<version>` user agent, 10-second timeout), reading at most 64 KB, with the last verified answer's ETag as `If-None-Match`, then its `.minisig`. `minisign-verify` checks the signature against the key, then the manifest is read: its `version`, its `notes_url`, and whether its `assets` hold this build's target (`universal-apple-darwin`, `x86_64-pc-windows-msvc`, or `x86_64-unknown-linux-gnu`); unknown fields are ignored. Only a newer version with this build's archive is offered, as `DownloadOnly`. An older or equal version, or a missing archive, reads as up to date. A 304 keeps the last verified answer, held in memory per channel. A refused signature or an unreadable manifest is logged and fails the check. A failed Check now shows its `CheckFailure`, `Offline` when the host could not be reached and `Failed` otherwise; a failed automatic check keeps the state it found. Switching channel drops what was found on the other channel, and when, and a check that ends after a switch leaves no answer and is followed by a check of the new channel.

The device settings are `update_channel` (absent: the build's own channel, Beta for a `-beta.N` build), `update_auto_check` (absent: on), `update_last_check` (when the last check that read a manifest ran and the version it offered, so a relaunch shows the offer until the next check; a channel switch removes it), and `last_run_version`, written at every launch for the "Updated to" toast of a later slice.
