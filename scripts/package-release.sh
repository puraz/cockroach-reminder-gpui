#!/usr/bin/env bash

set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: $0 <target> <version> [output-dir]" >&2
  exit 2
fi

target="$1"
version="${2#v}"
output_dir="${3:-dist}"
binary_name="cockroach-reminder-gpui"
binary_path="target/$target/release/$binary_name"
arch="${target%%-*}"

if [[ ! -x "$binary_path" ]]; then
  echo "Release binary not found: $binary_path" >&2
  exit 1
fi

mkdir -p "$output_dir"
output_dir="$(cd "$output_dir" && pwd)"
staging_dir="$(mktemp -d)"
trap 'rm -rf "$staging_dir"' EXIT

case "$target" in
  *-apple-darwin)
    app_dir="$staging_dir/Cockroach Reminder.app"
    contents_dir="$app_dir/Contents"
    mkdir -p "$contents_dir/MacOS" "$contents_dir/Resources"

    install -m 755 "$binary_path" "$contents_dir/MacOS/$binary_name"
    sed "s/__VERSION__/$version/g" packaging/macos/Info.plist > "$contents_dir/Info.plist"

    sips -s format icns assets/icon.png \
      --out "$contents_dir/Resources/AppIcon.icns" >/dev/null

    archive="$output_dir/cockroach-reminder-v$version-macos-$arch.zip"
    ditto -c -k --sequesterRsrc --keepParent "$app_dir" "$archive"
    ;;

  *-unknown-linux-gnu)
    package_dir="$staging_dir/cockroach-reminder"
    mkdir -p \
      "$package_dir/bin" \
      "$package_dir/share/applications" \
      "$package_dir/share/icons/hicolor/128x128/apps"

    install -m 755 "$binary_path" "$package_dir/bin/$binary_name"
    install -m 644 packaging/linux/cockroach-reminder.desktop \
      "$package_dir/share/applications/cockroach-reminder.desktop"
    install -m 644 assets/icon.png \
      "$package_dir/share/icons/hicolor/128x128/apps/cockroach-reminder.png"
    install -m 644 README.md README.zh-CN.md "$package_dir/"

    archive="$output_dir/cockroach-reminder-v$version-linux-$arch.tar.gz"
    tar -C "$staging_dir" -czf "$archive" cockroach-reminder
    ;;

  *)
    echo "Unsupported Unix target: $target" >&2
    exit 1
    ;;
esac

echo "Created $archive"
