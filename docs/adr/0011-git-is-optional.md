# Git is optional; the library becomes a repository when Git appears

Git 2.39 or newer was required before the library could exist, because creating it ran `git init`. Yet once the library exists, only Sync and Push, checking sources for updates, Update, and Reinstall need Git: scanning, importing, activation, and tree ids are computed without it. Requiring Git stops a user who only wants their agents' skills organized on one computer. So onboarding shows a Git step only when no usable Git is found. It offers to install Git and can be skipped.

Without Git the library is created as a plain directory, and everything except those features works. They stay visible and say that they need Git, with the same Install Git action Settings offers. When Git is found later, at launch, on focus, or by the Git step's own check, the engine makes the existing library a repository, as Create library did for a library directory that already existed. Its skills and records stay uncommitted until the first Sync or Push, as imports always are.

Installing is offered only where the system's own installer does the work: on macOS, Apple's Command Line Tools installer (`xcode-select --install`), and on Windows, `winget install --id Git.Git -e --source winget`, which asks for elevation itself. Linux shows the package manager's command to copy, because installing needs root. Every platform also links to Git's download page. The app never bundles Git.

Finding Git never runs a binary that opens an installer: on macOS, `/usr/bin/git` is used only when `xcode-select -p` names a developer directory that holds `usr/bin/git`, because without the Command Line Tools that stub opens Apple's install prompt. The search also looks in the usual install locations, because an app opened from Finder or after a winget install does not see the updated `PATH`.

## Considered Options

- Keeping Git required, with the step shown only when it is missing and Continue blocked until it is found, was rejected. It keeps a hard stop for features many users never use.
- Bundling a private Git, as GitHub Desktop and Sourcetree do, was rejected. The app would ship and patch Git's security fixes itself.
- Linking to a download page only was rejected. On macOS and Windows the system installer is one click away, and a link adds steps for the user.
- Running the Linux package manager with elevation from the app was rejected. Package managers and their privilege prompts differ between distributions.

## Consequences

- The captured contract's rule that bootstrap requires Git before library initialization is superseded.
- A drifted skill without a source cannot be restored while Git is missing, because the library has no history yet.
- The engine runs `xcode-select` and `winget`, both under its process policy; the UI only opens the download link.
