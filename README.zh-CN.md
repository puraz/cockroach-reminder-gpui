# 蟑螂提醒

简体中文 | [English](README.md)

[![CI](https://github.com/puraz/cockroach-reminder-gpui/actions/workflows/ci.yml/badge.svg)](https://github.com/puraz/cockroach-reminder-gpui/actions/workflows/ci.yml)
[![最新版本](https://img.shields.io/github/v/release/puraz/cockroach-reminder-gpui)](https://github.com/puraz/cockroach-reminder-gpui/releases/latest)

蟑螂提醒是一款运行在系统托盘里的休息计时器，支持 macOS、Windows 和 Linux。休息时间一到，蟑螂动画会出现在所有显示器上。覆盖窗口不会拦截鼠标，因此不会影响正在进行的操作。

项目使用 [GPUI](https://www.gpui.rs/) 和 [gpui-component](https://github.com/longbridge/gpui-component) 开发。

## 功能

- 自定义工作间隔和休息时长
- 支持多显示器的透明动画覆盖层
- 调整蟑螂数量、尺寸、移动范围和动画速度
- 休息开始时发送系统通知
- 从托盘暂停、继续、立即休息或打开设置
- 自动保存设置

默认每工作 25 分钟休息 15 秒，屏幕上会出现 10 只蟑螂。

## 下载

安装包可以从 [GitHub Releases](https://github.com/puraz/cockroach-reminder-gpui/releases/latest) 下载。

| 平台 | 文件 |
| --- | --- |
| macOS Apple 芯片 | `cockroach-reminder-v*-macos-aarch64.zip` |
| macOS Intel | `cockroach-reminder-v*-macos-x86_64.zip` |
| Windows x86_64 | `cockroach-reminder-v*-windows-x86_64.zip` |
| Linux x86_64 | `cockroach-reminder-v*-linux-x86_64.tar.gz` |

目前发布包没有代码签名，macOS 和 Windows 首次启动时可能显示安全提示。Linux 版本使用 X11，需要 X11 会话或 XWayland，并依赖 GTK 3、AppIndicator、libxdo、Fontconfig、XKB 和 Vulkan loader。

## 使用

启动后请从系统托盘或菜单栏图标操作，设置窗口不会自动弹出。

托盘菜单会显示剩余时间，也可以暂停或继续计时、立即开始一次休息，或打开设置窗口。休息期间的动画不会截获键盘和鼠标事件，下面的窗口仍可正常操作。

配置保存在系统配置目录的 `com.cockroach.reminder/config.json` 中。

## 从源码构建

项目使用 `rust-toolchain.toml` 中声明的 Rust nightly。安装 rustup 后，`cargo` 会自动准备所需工具链。

### macOS

需要安装 Xcode 及其命令行工具。GPUI 还需要 Metal 编译器，可用下面的命令检查：

```sh
xcrun metal -version
```

如果缺少编译器，且当前 Xcode 支持下载组件，可执行
`xcodebuild -downloadComponent MetalToolchain`。较旧的 Xcode 版本应升级到自带 Metal 编译器的版本。

### Windows

安装 Visual Studio Build Tools，勾选“使用 C++ 的桌面开发”，并安装较新的 Windows SDK。

### Ubuntu / Debian

先安装 GPUI 和系统托盘所需的本地依赖：

```sh
sudo apt-get update
sudo apt-get install build-essential clang cmake libayatana-appindicator3-dev \
  libfontconfig-dev libgtk-3-dev libssl-dev libvulkan1 libx11-xcb-dev \
  libxdo-dev libxkbcommon-x11-dev
```

然后构建并运行：

```sh
cargo run --release --locked
```

## 开发

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

提交 pull request 或推送到 `main` 后，CI 会在 Linux、Windows 和 macOS 上执行检查。发布新版本时，先修改 `Cargo.toml` 中的版本号并提交，再推送对应标签，例如 `v1.1.0`。发布流程会生成各平台压缩包以及 `SHA256SUMS` 校验文件。

## 许可

MIT
