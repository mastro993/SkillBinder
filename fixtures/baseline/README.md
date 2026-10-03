# Native baseline evidence

Revision `83d55ef789a7cf256021f6620742e0e9021a8312` is the behavior and visual reference. The original source was archived outside this repository before removal.

`manifest.json` inventories 88 canonical macOS PNGs and 352 associated JSON records. The images cover light and dark states at 1180 by 760, 860 by 620, and 1600 by 900 outer window sizes, including sidebar widths of 192, 232, and 400 pixels.

The capture used the original fixture components in production-equivalent mode. No web fonts loaded. The requested CSS stack resolved to the native Apple system font. The debug application and fixture badge remained present, so these captures do not establish packaged-release behavior.

The native capture API reported a screen-recording permission denial and used its CGWindow fallback. PNGs retain the raw black desktop canvas and the application window. They were not cropped, masked, or edited. Display scale and window bounds are recorded beside each image.

`behavior/` preserves declarative development-mode interaction evidence. Its screenshots remain outside the repository because they duplicate canonical states and contain development typography. References to those external screenshots are historical evidence locations.

The evidence does not prove native filesystem effects, a real directory-picker journey, IME behavior, operating-system preference changes, or Windows and Linux behavior. `xtask verify` checks hashes and JSON integrity, not visual correctness.
