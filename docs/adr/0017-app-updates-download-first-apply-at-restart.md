# App updates download first and apply at restart or quit

SkillBinder publishes stable releases and nightly betas ([docs/release.md](../release.md)), but a running copy never learns of them, so users stay on the build they first downloaded. The app now keeps itself up to date. It checks for an app update at launch, every hour while it runs, when the user presses Check now in Settings, and when the user changes channel. The user chooses a channel in Settings: Stable, which follows releases, or Beta, which follows the newest release or beta, whichever is newer.

An app update is downloaded, verified, and staged in the background before the user hears of it. Only then does a persistent notice say "SkillBinder 0.2.0 is ready", with Restart and Dismiss. Restart quits and relaunches into the new version. Dismiss hides the notice until the next launch, and the staged update is installed the next time the app quits, so the next launch is the new version either way. This works in place on macOS, Windows, and Linux. Where the app cannot replace itself, the notice offers Download instead, which opens the release page, and nothing is downloaded. That covers an app run from its disk image, a translocated app, and a folder the user cannot write.

The installed version never goes down. A user on 0.1.1-beta.12 who switches to Stable keeps the beta until Stable publishes a release newer than it, and Settings says so. An update that is staged but not installed yet and is not on the new channel is discarded on the switch.

"Update" already means a skill update from its source (ADR 0005). This feature is the **app update**, and its copy always names SkillBinder.

## Considered Options

- Notifying when an update is available and downloading it only on click was rejected by the user. The user waits through the download, and a notice that might not finish is worse than one that is ready.
- Installing only when the user presses Restart was rejected by the user. A dismissed update would never arrive for someone who never restarts on purpose, which is how VS Code and Zed avoid stale installs too.
- Hiding a dismissed notice until a newer version appears was rejected by the user. The update installs on quit anyway, so the notice only needs to stay quiet for the session.
- Updating in place only on macOS and linking Windows and Linux to the release page was rejected by the user. Those users would update by hand forever.
- Offering a downgrade when switching from Beta to Stable was rejected by the user. An older build can meet a database that a newer build migrated, and Diesel migrations do not run backwards.
- A switch that turns automatic checks off was rejected. The user asked for checks at launch and while the app runs, and each check sends only what the catalog request already sends (ADR 0014).

## Consequences

- Restart is unavailable while Sync, Push, Update all, an install, or a blocking change runs. Quitting would cancel work the user started.
- A build without the release stamp, such as a local build or a pull request's package, never checks. Settings says "Updates are off in this build".
- After an update, the first launch confirms it with a toast, "Updated to SkillBinder 0.2.0", whose action opens the release notes.
- How updates are found, verified, and swapped is [ADR 0018](0018-signed-update-manifests-on-pages.md).
