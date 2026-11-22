# The command is installed from Settings, the way each platform expects

The `skillbinder` command (ADR 0022) ships inside the app's package, where no shell looks for it. Settings › General gets a **Command line tool** row that shows whether the command works in a new terminal and installs, repairs, or uninstalls it ([spec](../specs/command-line.md)). Nothing puts the command on the PATH until the user presses Install, except Homebrew.

**macOS.** Install links `/usr/local/bin/skillbinder` to `Contents/MacOS/skillbinder-cli` in the app's bundle. `/usr/local/bin` is on every shell's PATH through `/etc/paths`, but it does not exist on a new Mac and `/usr/local` belongs to root, so the link is written as the user when the folder is writable, and otherwise through macOS's administrator prompt (`osascript` with administrator privileges), which creates the folder with mode 755. Cancelling the prompt changes nothing. Installing is offered only while the app runs from `/Applications` or `~/Applications` (ADR 0019), since a link into a disk image or Downloads would break. The Homebrew cask links the command itself with its `binary` stanza, and the row then reads "Installed by Homebrew" with no button, since `brew` owns that link.

**Windows.** The setup program installs `bin\skillbinder.exe`, a console program, beside the app. Install adds that `bin` folder to the user's PATH in `HKCU\Environment`, as an expandable string, and tells running programs the environment changed. Only terminals opened afterwards see it. Uninstall removes the entry, and so does the uninstaller.

**Linux.** An AppImage's files exist only while it runs, so Install copies the command to `~/.local/bin/skillbinder`. Each launch replaces that copy when it differs from the one in the running AppImage, so an app update updates it. When `~/.local/bin` is not on the PATH the app sees, the row says so and shows the line to add to the shell profile.

Install never overwrites a `skillbinder` it did not put there. A link or copy that leads to another SkillBinder, such as one moved or one from before an app update moved it, can be repaired. Anything else at that path is left alone, and the row says another `skillbinder` is in the way.

## Considered Options

- `~/.local/bin` on macOS with no password was rejected by the user. It is not on the PATH of a stock macOS shell, so most users would also have to edit their profile.
- Trying `/usr/local/bin` and falling back to `~/.local/bin` was rejected. It doubles the states to show and test for a case the prompt already covers.
- Adding the command to the PATH from the Windows setup program was rejected. Settings is where the user installs and checks the command on every platform, and it would install a command the user never asked for.
- Leaving Homebrew users to install from Settings was rejected by the user. The cask can link it with no click.
- Editing shell profiles (`.zshrc`, `.bashrc`) was rejected. It writes into files the user owns, and which file applies depends on the shell.

## Consequences

- Moving the app breaks the macOS link; the row then offers Repair.
- The engine gains the platform code for it: the link and the administrator prompt on macOS, the registry and the environment broadcast on Windows, and the copy on Linux. Windows adds a registry crate to the engine's dependencies.
- The cask files in `packaging/homebrew/` gain the `binary` stanza.
