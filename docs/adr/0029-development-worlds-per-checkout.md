# Development worlds per checkout

A contributor wants to run development builds while the installed SkillBinder keeps running, with as many development builds as they like, while Release and Beta stay single-instance ([spec](../specs/development-worlds.md)).

Release and Beta are already one app on two update channels: one bundle identifier, one application data folder, one `application.lock`. Only one of them runs at a time, and that stays as it is. A development build with no flags (`cargo run -p skillbinder`) keeps using the same normal locations, so it is that one instance too. Run while the installed app is up, it opens the installed app's window, as any second launch does.

A development build that runs beside the installed app runs in a **development world**: a copy of the synthetic world (`fixtures/synthetic/`) at `<checkout>/.skillbinder-dev/<name>/`, built on its first launch and kept between launches. It is launched as a `native-test` build on the world's `data`, `home`, `catalog`, and `app-updates` by `cargo run -p xtask -- dev [<name>]`. A world is a separate application data folder with a fake home, so it has its own instance lock, library lock, and command socket. It never reads or writes the user's library, their agents' skill folders, their agents' MCP settings, or their installed `skillbinder` command. One world runs per name, any number of names per checkout, and any number of checkouts at once.

## Considered Options

- Skipping the instance check for development builds, as Zed's Dev channel does, on the shared data, was rejected. The engine's `library.lock` refuses a second process on one library anyway, and a branch's new migrations would change a database the released app can't read.
- Separate application data on the real home was rejected. Two libraries would write managed copies into the same global skill directories (ADR 0006), each treating the other's copies as the user's own, and the development build's Settings could re-point the installed command and the agents' MCP entries at `target/`.
- An unstamped build quietly switching to separate data was rejected by the user: a plain `cargo run` must keep using the normal locations, so a development build can still run on real data.
- A fresh world on every launch was rejected: nothing set up in it would survive a restart.
- One development library shared by every checkout was rejected: branches with different schemas would break each other's database, and only one could run at a time.
- A `cargo dev` alias was rejected: `xtask verify` forbids subcommand aliases in `.cargo/config.toml`.

## Consequences

- No engine or app identity changes: the isolation `native-test` builds already have (`EngineConfig::isolated`, the `--data-dir` and `--home` refusals) is the whole boundary.
- `.skillbinder-dev/` is ignored by Git, survives `cargo clean`, and goes away with its worktree.
- A world shows the synthetic world's catalog and app updates, not the live ones, and its Git reads only the world's home, so it can't use the user's credentials.
- Alternating `cargo run -p skillbinder` and `xtask dev` in one checkout rebuilds the app each time, because the `native-test` feature changes the same `target/debug/skillbinder`.
