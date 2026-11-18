#!/bin/bash
set -euo pipefail
APP_NAME="Git看板"
VERSION="3.0.1"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/dist/$APP_NAME.app"
STAGE="$ROOT/dist/dmg-stage"
DMG="$ROOT/dist/$APP_NAME-$VERSION.dmg"

[ -d "$APP" ] || { echo "缺少 $APP（先跑 ./scripts/make-app.sh）" >&2; exit 1; }

rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
ditto "$APP" "$STAGE/$APP_NAME.app"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Git 看板" -srcfolder "$STAGE" -fs HFS+ -format UDZO -ov "$DMG" >/dev/null
rm -rf "$STAGE"
shasum -a 256 "$DMG" | awk '{print $1"  '$APP_NAME'-'$VERSION'.dmg"}' > "$DMG.sha256"
echo "已生成：$DMG"
du -sh "$DMG"
cat "$DMG.sha256"
