# Baseline font provenance

The application stylesheet declares `Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` in `apps/frontend/src/styles.css`. It contains no `@font-face` rule and the application bundles no font file. `design-tokens.json` preserves the declared stack. A computed-family observation of Inter alone does not identify which binary drew the glyphs.

The archived root component mounts `TanStackDevtools` from `@tanstack/react-devtools` version `0.10.12` unconditionally (`apps/frontend/src/routes/__root.tsx`). Its installed `@tanstack/devtools-ui` dependency is version `0.7.1`. That package's `dist/esm/styles/semantic-theme.js` inserts a document-level style element with ID `tanstack-devtools-fonts`. Its rules load `Inter` weight `100 900` from WOFF2 and `Bricolage Grotesque` weight `700` from TTF. The observed document font-face list matches those rules. This is the concrete source of the development-session font faces; it is not an explicit application font bundle.

| Font face | Installed package asset | URL expression in package ESM | SHA-256 | Size | License file |
| --- | --- | --- | --- | --- | --- |
| Inter, normal 100–900 | `@tanstack/devtools-ui/dist/assets/Inter-latin-BwkfbSeq.woff2` | `new URL("../../../assets/Inter-latin-BwkfbSeq.woff2", import.meta.url).href` in `dist/esm/assets/fonts/Inter-latin.js` | `2c295d99e26dcf357d4d01bcf270fd6924b600c9a13dd8c363ef114f4c6976fa` | 72,920 bytes | `font-licenses/OFL-Inter.txt` |
| Bricolage Grotesque, normal 700 | `@tanstack/devtools-ui/dist/assets/BricolageGrotesque-Bold-BIJrwikb.ttf` | `new URL("../../../assets/BricolageGrotesque-Bold-BIJrwikb.ttf", import.meta.url).href` in `dist/esm/assets/fonts/BricolageGrotesque-Bold.js` | `913ee3631949ee1b4fb2601269412fca5775eb994d4f87be2c366c94ac123dc5` | 82,204 bytes | `font-licenses/OFL-Bricolage-Grotesque.txt` |

Both included license texts identify the SIL Open Font License 1.1. Inter text names `The Inter Project Authors` with 2016 copyright; Bricolage text names `The Bricolage Grotesque Project Authors` with 2022 copyright. Copied license SHA-256 values are `262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a` and `46ba5f18ee20ea529f21d96c0ef8637a8314c1a5cfb2aa84018bc8157cbeff41`, respectively. No font binaries are copied into this export.

The package does not state a separate upstream font release version in its font license files. Package version `0.7.1` and binary SHA-256 identify the exact archived binaries. A font-independent or release build must be inspected before choosing the rewrite's bundled font, because the baseline screenshot may depend on this devtools style injection. Bricolage is defined by devtools but the application font stack does not request it.

## Production-equivalent capture result

The canonical capture in `../baseline/manifest.json` loaded no web fonts in all 88 states. The macOS production baseline therefore uses the native system font fallback. Development font records above are provenance evidence, not the replacement typography target.
