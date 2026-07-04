#!/bin/bash
# 生成 App 图标:绘制 1024 PNG → 生成 iconset → 打包成 Resources/AppIcon.icns
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p Resources

echo "▸ drawing icon"
swift scripts/GenerateIcon.swift Resources/AppIcon.png

ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
gen() { sips -z "$1" "$1" Resources/AppIcon.png --out "$ICONSET/$2" >/dev/null; }
gen 16   icon_16x16.png
gen 32   icon_16x16@2x.png
gen 32   icon_32x32.png
gen 64   icon_32x32@2x.png
gen 128  icon_128x128.png
gen 256  icon_128x128@2x.png
gen 256  icon_256x256.png
gen 512  icon_256x256@2x.png
gen 512  icon_512x512.png
gen 1024 icon_512x512@2x.png

iconutil -c icns "$ICONSET" -o Resources/AppIcon.icns
echo "▸ built Resources/AppIcon.icns"
