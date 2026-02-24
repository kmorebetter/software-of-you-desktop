#!/usr/bin/env bash
# sync-plugin.sh
#
# Copies the latest Software of You plugin into the desktop app's plugin/
# directory for bundling into the .app at build time.
#
# Usage:
#   ./scripts/sync-plugin.sh [path-to-plugin-repo]
#
# If no path is given, defaults to ../better-software-of-you

set -euo pipefail

PLUGIN_SRC="${1:-$(dirname "$(dirname "$0")")/better-software-of-you}"
PLUGIN_DEST="$(dirname "$(dirname "$0")")/plugin"

if [ ! -d "$PLUGIN_SRC" ]; then
  echo "Error: plugin source not found at $PLUGIN_SRC"
  echo "Usage: ./scripts/sync-plugin.sh /path/to/better-software-of-you"
  exit 1
fi

echo "Syncing plugin from: $PLUGIN_SRC"
echo "           into:     $PLUGIN_DEST"

# Clear destination (except .gitkeep)
find "$PLUGIN_DEST" -not -name '.gitkeep' -not -path "$PLUGIN_DEST" -delete 2>/dev/null || true

# Copy plugin files (exclude git, node_modules, output, local data)
rsync -a \
  --exclude='.git' \
  --exclude='.DS_Store' \
  --exclude='node_modules' \
  --exclude='output/' \
  --exclude='data/soy.db' \
  --exclude='*.zip' \
  "$PLUGIN_SRC/" "$PLUGIN_DEST/"

# Write/update VERSION file (uses git tag or commit hash from source repo)
VERSION=$(cd "$PLUGIN_SRC" && git describe --tags --always 2>/dev/null || echo "0.0.0-dev")
echo "$VERSION" > "$PLUGIN_DEST/VERSION"

echo "Done. Plugin version: $VERSION"
echo "Ready to build. Run: cargo tauri build --target universal-apple-darwin"
