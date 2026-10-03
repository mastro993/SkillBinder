# Dependencies and native prerequisites

| Dependency | Version / role |
| --- | --- |
| Rust | 1.96.0, pinned in rust-toolchain.toml |
| GPUI | 0.2.2, native renderer and window/input platform |
| gpui-component | 0.5.1, controls, dialogs, keyboard focus and text input |
| SQLite / Diesel | Bundled SQLite, Diesel 2.3; exact graph in Cargo.lock |
| System Git | 2.39 or newer, trusted local authentication |
| Hugeicons | Bundled MIT SVGs; no runtime download |
| Node.js | 24, development verification scripts only |

`Cargo.lock` locks all native dependencies. The application has no JavaScript runtime or package
installation step. Optional embedded-browser features of dependencies are not enabled.

## Linux

A C/C++ compiler, pkg-config, CMake, Clang/libclang, X11/Wayland development headers, fontconfig,
freetype, xkbcommon, OpenSSL, and a Vulkan-capable graphics driver are required. On Ubuntu:

```sh
sudo apt-get install build-essential pkg-config cmake clang libclang-dev libssl-dev \
  libfontconfig1-dev libfreetype6-dev libx11-dev libxcb1-dev libxkbcommon-dev \
  libxkbcommon-x11-dev libwayland-dev libvulkan-dev libx11-xcb-dev libxrandr-dev \
  libxi-dev libxcursor-dev libxinerama-dev libasound2-dev
```

Folder dialogs use the desktop portal; install `xdg-desktop-portal` plus your desktop's backend.
`xdg-open` opens the logs folder. Headless launch testing needs Xvfb and a compatible software Vulkan
driver; a successful compile does not establish that every virtual GPU supports GPUI rendering.

## macOS

Install Xcode Command Line Tools. GPUI uses Cocoa and Metal; test on an actual supported Mac.

## Windows

Install Visual Studio C++ Build Tools, the Windows SDK, and the Rust MSVC toolchain. GPUI uses the
native Windows graphics stack. No installer signing credentials are needed for source builds.

## Verification scope

CI builds/checks the native workspace on Linux, macOS, and Windows. Filesystem probes that rely
on Unix permissions or symlinks explicitly skip on other hosts. Native window tests and actual
runner outcomes are evidence for those environments, not a public compatibility certification.
