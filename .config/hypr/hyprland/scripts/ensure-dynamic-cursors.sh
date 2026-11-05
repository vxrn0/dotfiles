#!/usr/bin/env bash
# Loads the cursor plugin, rebuilding it if it is missing or was built against another
# Hyprland version. Hyprland checks the ABI on load, so an update always invalidates it.
set -uo pipefail

DEST="$HOME/.local/lib/hyprland"
PLUGIN="$DEST/dynamic-cursors.so"
BUILDER="$HOME/.config/hypr/hyprland/scripts/build-dynamic-cursors.sh"

loaded() {
    hyprctl plugin list 2>/dev/null | grep -q dynamic-cursors
}

try_load() {
    hyprctl plugin load "$PLUGIN" >/dev/null 2>&1
    loaded
}

try_load && exit 0

if [ ! -x "$BUILDER" ]; then
    notify-send "Cursor plugin" "Cannot rebuild: $BUILDER is missing" 2>/dev/null || true
    exit 1
fi

"$BUILDER" >/tmp/dynamic-cursors-build.log 2>&1

if try_load; then
    exit 0
fi

notify-send "Cursor plugin" "Rebuild failed, see /tmp/dynamic-cursors-build.log" 2>/dev/null || true
exit 1
