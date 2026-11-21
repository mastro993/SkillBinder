## Install

- **macOS:** open the disk image and drag SkillBinder to Applications, or install it with Homebrew: `brew install --cask mastro993/tap/skillbinder@beta` for a beta, `mastro993/tap/skillbinder` for a release.
- **Windows:** run the setup program. SmartScreen warns once: choose More info, then Run anyway.
- **Linux:** make the AppImage executable with `chmod +x skillbinder-*.AppImage`, keep it in a folder you can write, such as `~/Applications`, so it can update itself, and run it. At its first launch it offers to add itself to your applications menu. It needs FUSE (`fusermount`, from the `fuse3` package on most distributions) but not `libfuse2`; where FUSE is missing, run it with `--appimage-extract-and-run`.
