# Bundle identifier dev.fedemas.skillbinder

The macOS bundle identifier changes from `com.skillbinder.desktop` to `dev.fedemas.skillbinder`, the reverse of a domain the maintainer owns, which `skillbinder.com` is not. Only betas have shipped so far (`v0.1.1-beta.3` to `beta.8`), so this is the cheapest time to change it ([spec](../specs/development-worlds.md)).

Only the macOS identity changes. The Windows setup's Inno `AppId` stays, since the uninstaller and every update find the installation by it (docs/release.md). The Linux app id `skillbinder`, the `.desktop` entry's name, and its `StartupWMClass` stay too. The data folder is `~/Library/Application Support/SkillBinder`, which comes from the `directories` crate's base folder and not from the bundle identifier, so the library, settings, and instance lock carry over untouched.

## Considered Options

- Keeping `com.skillbinder.desktop` was rejected by the user: the project doesn't own that domain.
- Changing the Windows `AppId` too was rejected: existing installs would no longer update or uninstall.

## Consequences

- An installed beta updates across the change. The staged bundle's check (`app_update/stage.rs`) requires a Developer ID signature from the release team and never compares identifiers.
- macOS treats the new identifier as a new app once. Permissions it granted under the old one, such as access to a workspace in Documents or Desktop, are asked for again, and the old `~/Library/Preferences/com.skillbinder.desktop.plist` is left behind.
- The Homebrew casks quit `dev.fedemas.skillbinder` on uninstall, and their `zap` removes both preference files.
