# Third-party notices

SkillBinder depends on third-party Rust packages under their respective licenses. `Cargo.lock` identifies the exact graph. Imported skills and their payloads retain their source licenses.

## Native UI dependencies

- GPUI Kit 0.10.0 and its GPUI Base and GPUI Component crates are licensed under Apache-2.0. Their source is available at `https://github.com/longbridge/gpui-kit`, release revision `f697c3133d55c64fa8a5b67926147fb3dadd276d`.
- GPUI Pre 0.3.12 and its platform crates retain their upstream license terms. The shared backend is supplied by the project's fork of `gpui-mcp`, `https://github.com/mastro993/gpui-mcp`, at revision `d162da702699d4298498438a3864face3569aea7`; its GPUI patch and Apache-2.0 MCP bridge retain their notices. The bridge is included only in native-test builds.
- Hugeicons artwork is extracted from `@hugeicons/core-free-icons` version `4.3.5`. The complete MIT notice is in `assets/hugeicons/LICENSE.md`. `inventory.json` records the exact package modules, artwork hashes, and variants; `preset-glyphs.json` does the same for the glyphs only preset icons use.
- Agent logos under `assets/agent-logos/` are copied byte-for-byte from `@lobehub/icons-static-svg` version `1.95.1` (MIT, Copyright (c) 2023 LobeHub; the package ships no license file, so `LICENSE-lobe-icons.md` is taken from `lobehub/lobe-icons` at `05a9b8c41e4735a4a2b6d2f1e43574bd8ec6c8cb`) and from `simple-icons` version `16.35.0` (CC0-1.0, with its trademark disclaimer in `DISCLAIMER-simple-icons.md`). The fx logo is fx's own brand artwork from `fx-logos.zip`, which states no license. `inventory.json` records each logo's agent, source file, and hash. The licenses cover the drawings only. Each logo is a trademark of its owner and is used only to identify that owner's agent; SkillBinder is not affiliated with or endorsed by them.

  MIT License

  Copyright (c) 2023 LobeHub

  Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

  The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

The historical font license records under `fixtures/contracts/font-licenses` document baseline research. No font binaries from the previous application are bundled.

## Menu bar and system tray

The menu bar or system tray icon and its menu are drawn by `tray-icon` 0.26.1 and `muda` 0.21.2 (MIT OR Apache-2.0) on macOS and Windows, and by `ksni` 0.3.7 (Unlicense) on Linux, over the `zbus` D-Bus connection GPUI already uses (ADR 0021). Their sources are available at `https://github.com/tauri-apps/tray-icon`, `https://github.com/tauri-apps/muda`, and `https://github.com/iovxw/ksni`. Their other new dependencies, at the versions `Cargo.lock` pins: `crossbeam-channel`, `keyboard-types`, `pastey`, and `task-local`: MIT OR Apache-2.0; `dpi`: Apache-2.0 AND MIT. The app decodes the icon's artwork with `png` 0.18.1 and carries its clicks over `async-channel` 2.5.0 (both MIT OR Apache-2.0), which GPUI already depends on. The artwork under `assets/tray/` is SkillBinder's own rune, the template glyph in `docs/design/brand/source/icons.js`.

## Command line

The `skillbinder` command and the app talk over a local socket or named pipe through `interprocess` 2.4.4 (0BSD OR Apache-2.0), with `recvmsg` and `doctest-file` (0BSD) and, on Windows, `widestring` (MIT OR Apache-2.0) (ADR 0022). The command reads its arguments with `clap` 4.6.7, `clap_builder`, and `clap_lex` (MIT OR Apache-2.0), and catches Ctrl-C with `ctrlc` 3.5.3 (MIT OR Apache-2.0), which uses `nix` 0.32.0 (MIT) on Unix, `dispatch2` (Zlib OR Apache-2.0 OR MIT) on macOS, and `windows-sys` (MIT OR Apache-2.0) on Windows.

- `anstream`, `anstyle`, `anstyle-parse`, `anstyle-query`, `anstyle-wincon`, `colorchoice`, `is_terminal_polyfill`, `once_cell_polyfill`, and `utf8parse`: MIT OR Apache-2.0; `strsim`: MIT.

## HTTP client

The engine fetches Explore's catalog with `ureq` 3.5.2 (MIT OR Apache-2.0) over `rustls` 0.23.46 (Apache-2.0 OR ISC OR MIT), without OpenSSL (ADR 0014); `xtask`'s catalog job, which is never shipped, uses the same crate. Its source is available at `https://github.com/algesten/ureq`. Its other dependencies, at the versions `Cargo.lock` pins:

- `ureq-proto`, `http`, `httparse`, `base64`, `percent-encoding`, `rustls-pki-types`, `utf8-zero`, `once_cell`, `log`, `itoa`, `cfg-if`, `getrandom`, and `libc`: MIT OR Apache-2.0; `zeroize`: Apache-2.0 OR MIT; `bytes`: MIT.
- `ring` 0.17.14: Apache-2.0 AND ISC, its BoringSSL-derived code under the OpenSSL-style ISC terms its `LICENSE` file records; `rustls-webpki` and `untrusted`: ISC; `subtle`: BSD-3-Clause.
- `webpki-roots` 1.0.12: CDLA-Permissive-2.0. It embeds Mozilla's root certificate list, which the engine trusts for HTTPS.

## Update signing

The application checks the app update manifests' signatures with `minisign-verify` 0.3.0 (MIT), which has no dependencies (ADR 0018). `xtask`, which is never shipped, signs them with `minisign` 0.10.0 (MIT), which the engine's tests also use to sign fixture manifests. Their sources are available at `https://github.com/jedisct1/rust-minisign` and `https://github.com/jedisct1/rust-minisign-verify`. `minisign`'s other dependencies, at the versions `Cargo.lock` pins:

- `ct-codecs`: MIT; `scrypt`, `salsa20`, `pbkdf2`, `hmac`, `cipher`, `sha2`, and `getrandom`: MIT OR Apache-2.0.
- `rpassword` and `rtoolbox`: Apache-2.0.

On Windows, the application asks whether the session is ending, so a quit at logoff or shutdown starts no app update, with `winsafe` 0.0.29 (MIT), which has no dependencies. Its source is available at `https://github.com/rodrigocfd/winsafe`. The Windows setup program is built with Inno Setup 6.7.4, under the Inno Setup License, which allows distributing the setup programs it compiles; its source is available at `https://github.com/jrsoftware/issrc`.

The Linux AppImage starts through the AppImage type 2 runtime, release `20251108`, copied unmodified into every AppImage (MIT, Copyright (c) 2004-23 probonopd). The runtime statically links musl libc (MIT), libfuse (LGPL-2.1), squashfuse (BSD-2-Clause), libzstd (BSD-3-Clause), and zlib (zlib License), under the terms its `LICENSE` lists. Its source and build scripts, from which the runtime can be rebuilt and relinked, are available at `https://github.com/AppImage/type2-runtime/tree/20251108`. The AppImage is assembled with appimagetool 1.9.1 (MIT), which is not shipped; its source is available at `https://github.com/AppImage/appimagetool`.

## vercel-labs/skills registry snapshot

- Project: `vercel-labs/skills`
- Commit: `13e4063a1cf913f5606d57d42ab83a86f5001e04`
- License: MIT
- Files: `src/agents.ts`, `src/types.ts`
- Use: reviewed declarative registry data with pinned source hashes; upstream executable source is not bundled. The native gate verifies preserved bytes and generated documentation, not a fresh comparison against upstream.
- Copyright: Copyright (c) 2026 Vercel, Inc.
