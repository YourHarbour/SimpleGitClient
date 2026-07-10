#!/usr/bin/env bash
# Build the release binary and install a desktop entry + icon into ~/.local.
# Run again any time to update. Uninstall with ./install.sh --uninstall.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
APP_ID="com.simplegitclient.app"
BIN_NAME="simple-git-client"
ICON_FILE="$ICON_DIR/$APP_ID.png"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f "$BIN_DIR/$BIN_NAME" "$APP_DIR/$APP_ID.desktop" "$ICON_FILE"
  update-desktop-database "$APP_DIR" 2>/dev/null || true
  gtk-update-icon-cache "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
  echo "Uninstalled Simple Git Client."
  exit 0
fi

echo ">> Building release binary (this can take a couple of minutes)…"
( cd "$HERE" && cargo build --release )

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
install -m 0755 "$HERE/target/release/$BIN_NAME" "$BIN_DIR/$BIN_NAME"
install -m 0644 "$HERE/icons/$APP_ID.png" "$ICON_FILE"

sed \
  -e "s|@EXEC@|$BIN_DIR/$BIN_NAME|g" \
  -e "s|@ICON@|$ICON_FILE|g" \
  "$HERE/data/$APP_ID.desktop.in" > "$APP_DIR/$APP_ID.desktop"

update-desktop-database "$APP_DIR" 2>/dev/null || true
gtk-update-icon-cache "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo ""
echo ">> Installed:"
echo "   binary  : $BIN_DIR/$BIN_NAME"
echo "   desktop : $APP_DIR/$APP_ID.desktop"
echo "   icon    : $ICON_FILE"
echo ""
echo "Launch it from your app grid as \"Simple Git Client\", or run: $BIN_NAME"
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
  echo "(Note: $BIN_DIR is not on your PATH — add it, or run the full path.)"
fi
