#!/bin/bash
# 把 SimpleGitClient 打包成一个真正的 macOS .app bundle。
# 用法: scripts/build_app.sh [debug|release]   (默认 release)
#   产物: dist/SimpleGitClient.app
set -euo pipefail

CONFIG="${1:-release}"
APP_NAME="SimpleGitClient"
BUNDLE_ID="com.simplegitclient.app"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "▸ swift build -c $CONFIG"
swift build -c "$CONFIG"

BIN="$(swift build -c "$CONFIG" --show-bin-path)/$APP_NAME"
APP="$ROOT/dist/$APP_NAME.app"

# 确保有图标(没有就现画一个)
if [ ! -f "$ROOT/Resources/AppIcon.icns" ]; then
    echo "▸ icon missing — generating"
    "$ROOT/scripts/make_icon.sh"
fi

echo "▸ assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$APP_NAME"
cp "$ROOT/Resources/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>              <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>       <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>        <string>$BUNDLE_ID</string>
    <key>CFBundleExecutable</key>        <string>$APP_NAME</string>
    <key>CFBundleIconFile</key>          <string>AppIcon</string>
    <key>CFBundlePackageType</key>       <string>APPL</string>
    <key>CFBundleVersion</key>           <string>1</string>
    <key>CFBundleShortVersionString</key><string>0.1</string>
    <key>LSMinimumSystemVersion</key>    <string>14.0</string>
    <key>NSPrincipalClass</key>          <string>NSApplication</string>
    <key>NSHighResolutionCapable</key>   <true/>
    <key>LSApplicationCategoryType</key> <string>public.app-category.developer-tools</string>
</dict>
</plist>
PLIST

# Ad-hoc 代码签名,避免每次启动的安全提示
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true

echo "▸ Built $APP"
echo "  打开: open \"$APP\""
