#!/bin/bash
# 组装 macOS .app（Info.plist + 图标 + ad-hoc 签名）
set -euo pipefail

APP_NAME="Git看板"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/src-tauri/target/release/gitboard"
BUNDLE="$ROOT/dist/$APP_NAME.app"

if [ ! -f "$BIN" ]; then
  echo "缺少二进制：$BIN（先 cd src-tauri && cargo build --release）" >&2
  exit 1
fi

rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
cp "$BIN" "$BUNDLE/Contents/MacOS/GitBoard"
cp "$ROOT/src-tauri/icons/AppIcon.icns" "$BUNDLE/Contents/Resources/AppIcon.icns"

cat > "$BUNDLE/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>Git 看板</string>
  <key>CFBundleDisplayName</key>
  <string>Git 看板</string>
  <key>CFBundleExecutable</key>
  <string>GitBoard</string>
  <key>CFBundleIdentifier</key>
  <string>local.gitboard.app</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>3.1.1</string>
  <key>CFBundleVersion</key>
  <string>6</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
PLIST

printf 'APPL????' > "$BUNDLE/Contents/PkgInfo"
codesign --force --deep --sign - "$BUNDLE" >/dev/null 2>&1 || true
echo "已生成：$BUNDLE"
du -sh "$BUNDLE"
