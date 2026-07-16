# Cockroach Reminder

[简体中文](README.zh-CN.md) | English

[![CI](https://github.com/puraz/cockroach-reminder-gpui/actions/workflows/ci.yml/badge.svg)](https://github.com/puraz/cockroach-reminder-gpui/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/puraz/cockroach-reminder-gpui)](https://github.com/puraz/cockroach-reminder-gpui/releases/latest)

Cockroach Reminder is a tray-based break timer for macOS, Windows, and Linux. When a break starts, animated cockroaches walk across every connected display in transparent, click-through windows. They are hard to ignore, but they do not block your work.

The app is built with [GPUI](https://www.gpui.rs/) and [gpui-component](https://github.com/longbridge/gpui-component). Its interface is currently in Simplified Chinese.

## Features

- Configurable work interval and break duration
- Multi-display animated overlays that let mouse input pass through
- Controls for cockroach count, size, movement, and animation speed
- System notifications at the start of a break
- Tray controls to pause, resume, start a break immediately, or open settings
- Settings saved between launches

The default schedule is a 25-minute work interval followed by a 15-second break with 10 cockroaches.

## Download

Packages are available from the [latest GitHub release](https://github.com/puraz/cockroach-reminder-gpui/releases/latest).

| Platform | Package |
| --- | --- |
| macOS Apple Silicon | `cockroach-reminder-v*-macos-aarch64.zip` |
| macOS Intel | `cockroach-reminder-v*-macos-x86_64.zip` |
| Windows x86_64 | `cockroach-reminder-v*-windows-x86_64.zip` |
| Linux x86_64 | `cockroach-reminder-v*-linux-x86_64.tar.gz` |

Release packages are currently unsigned and not notarized. macOS and Windows may show a security prompt on first launch. Linux builds target X11; an X11 session or XWayland is required, along with GTK 3, AppIndicator, libxdo, Fontconfig, XKB, and the Vulkan loader.

### Opening the app on macOS

After extracting the archive, move `Cockroach Reminder.app` to `Applications`. Try Control-clicking the app and choosing **Open** first. If macOS still reports that the app is damaged, remove the quarantine attribute and launch it again:

```sh
xattr -dr com.apple.quarantine "/Applications/Cockroach Reminder.app"
open "/Applications/Cockroach Reminder.app"
```

If `xattr` reports a permission error, run that command again with `sudo`. Only bypass Gatekeeper for a package downloaded from this repository's GitHub Releases; the archive can be checked against the published `SHA256SUMS` file before opening it.

## Usage

Start the app and use its tray or menu-bar icon. The settings window does not open automatically.

From the tray menu you can check the remaining time, pause or resume the timer, trigger a break immediately, and open the settings window. During a break, the overlay remains click-through, so keyboard and mouse input continue to reach the windows underneath it.

Settings are stored in your operating system's configuration directory under `com.cockroach.reminder/config.json`.

## Build from source

The repository uses the Rust nightly toolchain declared in `rust-toolchain.toml`. `cargo` installs it automatically through rustup.

### macOS

Install Xcode and its command-line tools. GPUI also needs the Metal compiler. Check it with:

```sh
xcrun metal --version
```

If it is missing and your Xcode supports downloadable components, run
`xcodebuild -downloadComponent MetalToolchain`. Older Xcode versions should be updated to a release that includes the Metal compiler.

### Windows

Install Visual Studio Build Tools with the Desktop development with C++ workload and a recent Windows SDK.

### Ubuntu / Debian

Install the native libraries used by GPUI and the tray integration:

```sh
sudo apt-get update
sudo apt-get install build-essential clang cmake libayatana-appindicator3-dev \
  libfontconfig-dev libgtk-3-dev libssl-dev libvulkan1 libx11-xcb-dev \
  libxdo-dev libxkbcommon-x11-dev
```

Then build and run the application:

```sh
cargo run --release --locked
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

Pull requests and pushes to `main` run these checks on Linux, Windows, and macOS. To publish a release, update the version in `Cargo.toml`, commit the change, and push a matching tag such as `v1.1.0`. The release workflow builds all platform packages and adds a `SHA256SUMS` file.

## License

MIT
