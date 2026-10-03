# Retained assets and notices

- `assets/icon.png` is byte-identical to archived `apps/tauri/icons/icon.png` at revision `83d55ef789a7cf256021f6620742e0e9021a8312`. SHA-256: `bbcadbf15a9efb9aa3b06cb32d3d2d79ad8eb3cd3b632d05d71322c9c33cf64b`. No separate attribution or license file sits beside this icon in the baseline; preserve the repository license and review distribution rights if packaging it.
- `LICENSE` is the unchanged repository license. SHA-256: `0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0`.
- `THIRD_PARTY_NOTICES.md` is unchanged and records the reviewed registry snapshot, its MIT license, and copyright. SHA-256: `d9e8dbcbc1a7060c5dd2f2b09883007a4d266533356d1965688f7671899ed29c`.
- `registry-79.json` is the checked-in declarative registry, not upstream executable code. SHA-256: `40a66c7e17208f84620c3abafc58f50d69ce95d914b7c2e0b67a26b24efc449d`.
- The current interface imports Hugeicons from a package. No Hugeicons SVG or font artwork was checked into the baseline beyond package dependencies. The rewrite needs its own reviewed license record for any extracted icon assets.

## Hugeicons export

- `assets/hugeicons/` contains 22 SVG glyphs imported by the archived frontend, plus two stroke-width-2 variants used by the sheet close button and sidebar trigger. Each SVG uses the package's exact shape data, `0 0 24 24` viewBox, and the renderer's default size, fill, color, and opacity-bearing element order. `inventory.json` lists each glyph's importing files, source ESM module, module SHA-256, SVG SHA-256, element count, and variant overrides.
- Installed source: `@hugeicons/core-free-icons` version `4.3.5`, `apps/frontend/node_modules/@hugeicons/core-free-icons/dist/esm/`. The package metadata says `MIT`. Its complete `LICENSE.md` is copied byte-for-byte into `assets/hugeicons/LICENSE.md` with SHA-256 `1658d8213209df7b9b86dfc05d724ede48d00dbc27abc15976ec7adec9601cde`. It states `Copyright (c) 2025 Hugeicons` and requires inclusion of the copyright and permission notice in copies or substantial portions. Carry that license into the rewritten app's third-party notices when bundling these SVGs.
- The baseline renders SVG data through `@hugeicons/react` version `1.1.10`. No font asset has been copied or licensed here. `design-tokens.json` records the declared font stack and notes that computed Inter may come from development tooling.
