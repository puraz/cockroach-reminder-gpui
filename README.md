# Cockroach Reminder GPUI

A GPUI + gpui-component desktop break reminder with animated, click-through
cockroach overlays and a tray-based settings window.

## Requirements

- Rust nightly (pinned by `rust-toolchain.toml`)
- macOS: Xcode Metal Toolchain

Install the macOS shader compiler when Xcode reports that it is missing:

```sh
xcodebuild -downloadComponent MetalToolchain
```

## Run

```sh
cargo run --release
```

## Verify

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```
