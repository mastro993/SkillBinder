---
name: skillbinder-dev
description: Run SkillBinder from this checkout on a development world, a fake machine, and drive it. Use when asked to run, launch, screenshot, or drive the app, or to check a change in the real app.
---

# Run SkillBinder on a development world

A development world is a copy of the synthetic world at `.skillbinder-dev/<name>/` in the checkout: fictional agents, projects, a skill source, a Sync remote, a catalog, and app update manifests. The app runs on it beside the user's installed SkillBinder and every other world, and never touches the user's library, agents, or home.

## Launch

```sh
GPUI_MCP_BACKGROUND=1 cargo run -p xtask --locked -- dev [<name>] [--fresh] -- --mcp
```

Run it in the background: it stays attached until the app quits. Its first lines print the world's path, its projects folder, its Sync remote's `file://` URL, and the command line for it.

- Launch every run this way. A plain `cargo run -p skillbinder` opens the user's real library and agents; use it only when the user asks for real data.
- Give each parallel task its own `<name>`; `default` is shared by every session in the checkout. Running a world that is already up only brings its window forward.
- `--fresh` rebuilds the world and starts at onboarding. It refuses while that world runs: quit the app first (`keyboard` `cmd-q` through the bridge).
- A world launched before onboarding finished opens at onboarding; a later launch opens where it was left.

## Drive

Use the `gpui-skillbinder` MCP server from `.mcp.json`. When more than one SkillBinder runs, call `list_apps` and `select_app` the one whose process runs your world. `GPUI_MCP_BACKGROUND=1` keeps the window from taking focus; the bridge draws it covered, so never raise it.

Reach the same world from the command line:

```sh
cargo run -p skillbinder-cli -- --data-dir .skillbinder-dev/<name>/data <command>
```

## The world

`fixtures/synthetic/README.md` lists what the world holds and which states it exercises (updates, Blocked, Invalid, same-slug conflicts, Explore's catalog, a repository to add). Its knobs, as app arguments after `--` unless noted:

- **No Git:** `--git-dir <empty folder>`; link a `git` executable into it later to bring Git back.
- **Beta:** `--app-version 0.1.1-beta.12`.
- **App updates:** `--app-bundle .skillbinder-dev/<name>/data/fixture-applications/SkillBinder.app` (create the folder); the world's `app-updates/` offers the update.
- **Offline:** rename the world's `catalog/` or `app-updates/` folder.
- **Installs and Sync on local repositories:** the app's Git reads only `<world>/home/.gitconfig`, so add `[url "file://<repo>.git"] insteadOf = <URL the app uses>` there. A rewrite to a local port that never answers holds an install or Sync in progress.
- **Workspaces:** once onboarding is done, `execute_app_command add_workspace {"path": "<world>/projects"}`, since the bridge can't drive the folder panel. During onboarding it answers `started` but the window shows "Finish setup before adding workspaces."
- **Activation progress:** `execute_app_command hold_activation` holds the progress strip, `{"toast": true}` its end toast.

`docs/native-gate/checks.md` covers the isolation rules, the bridge, and `mcp-smoke`.

## Screenshots

The world's paths contain the user's name, and a workspace's header, Settings, Sync, and error messages show them. Crop them out, or show a remote through an `insteadOf` URL, before sharing a screenshot, and say what you cropped.

On macOS, when MCP screenshots time out while the window renders, capture the window by its ID:

```sh
pgrep -f 'debug/skillbinder --data-dir .*/.skillbinder-dev/<name>/data'  # the app's <pid>
swift .agents/skills/skillbinder-dev/window-id.swift <pid>               # its <window id>
screencapture -x -o -l<window id> shot.png
```
