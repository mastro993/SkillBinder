# Third-party notices

SkillBinder depends on third-party Rust packages under their respective licenses. `Cargo.lock` identifies the exact graph. Imported skills and their payloads retain their source licenses.

## Native UI dependencies

- GPUI Kit 0.7.0 and its GPUI Base and GPUI Component crates are licensed under Apache-2.0. Their source is available at `https://github.com/longbridge/gpui-kit`, release revision `0c830f4d257e69fdd17200650533ab4ca9a40cc0`.
- GPUI Pre 0.3.7 and its platform crates retain their upstream license terms. The shared backend is supplied by the unmodified `gpui-mcp` repository at revision `9dda8e5cb49990261e3fdaa26abe38112d30dafb`; its GPUI patch and Apache-2.0 MCP bridge retain their notices. The bridge is included only in native-test builds.
- Hugeicons artwork is extracted from `@hugeicons/core-free-icons` version `4.3.5`. The complete MIT notice is in `assets/hugeicons/LICENSE.md`. `inventory.json` records the exact package modules, artwork hashes, and variants.
- Agent logos under `assets/agent-logos/` are copied byte-for-byte from `@lobehub/icons-static-svg` version `1.95.1` (MIT, Copyright (c) 2023 LobeHub; the package ships no license file, so `LICENSE-lobe-icons.md` is taken from `lobehub/lobe-icons` at `05a9b8c41e4735a4a2b6d2f1e43574bd8ec6c8cb`) and from `simple-icons` version `16.35.0` (CC0-1.0, with its trademark disclaimer in `DISCLAIMER-simple-icons.md`). The fx logo is fx's own brand artwork from `fx-logos.zip`, which states no license. `inventory.json` records each logo's agent, source file, and hash. The licenses cover the drawings only. Each logo is a trademark of its owner and is used only to identify that owner's agent; SkillBinder is not affiliated with or endorsed by them.

  MIT License

  Copyright (c) 2023 LobeHub

  Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

  The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

The historical font license records under `fixtures/contracts/font-licenses` document baseline research. No font binaries from the previous application are bundled.

## vercel-labs/skills registry snapshot

- Project: `vercel-labs/skills`
- Commit: `13e4063a1cf913f5606d57d42ab83a86f5001e04`
- License: MIT
- Files: `src/agents.ts`, `src/types.ts`
- Use: reviewed declarative registry data with pinned source hashes; upstream executable source is not bundled. The native gate verifies preserved bytes and generated documentation, not a fresh comparison against upstream.
- Copyright: Copyright (c) 2026 Vercel, Inc.
