#!/usr/bin/env bash
set -uo pipefail

ok=0
bad=0

check() {
    local label="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        printf 'ok    %s\n' "$label"
        ok=$((ok + 1))
    else
        printf 'FAIL  %s\n' "$label"
        bad=$((bad + 1))
    fi
}

check_file() {
    if [ -e "$1" ]; then
        printf 'ok    %s\n' "$2"
        ok=$((ok + 1))
    else
        printf 'FAIL  %s (%s)\n' "$2" "$1"
        bad=$((bad + 1))
    fi
}

check_shell() {
    if sh -c "$1" >/dev/null 2>&1; then
        printf 'ok    %s\n' "$2"
        ok=$((ok + 1))
    else
        printf 'FAIL  %s\n' "$2"
        bad=$((bad + 1))
    fi
}

for c in Hyprland hyprctl hypridle hyprlock qs foot fish starship cliphist grim slurp \
         wl-copy wl-paste matugen jq tesseract wf-recorder brightnessctl btop gray; do
    check "command $c" command -v "$c"
done

for f in \
    "$HOME/.config/hypr/hyprland.lua" \
    "$HOME/.config/hypr/hyprland/colors.lua" \
    "$HOME/.config/hypr/hyprland/execs.lua" \
    "$HOME/.config/hypr/hypridle.conf" \
    "$HOME/.config/matugen/config.toml" \
    "$HOME/.config/foot/foot.ini" \
    "$HOME/.config/foot/colors.ini" \
    "$HOME/.config/fish/config.fish" \
    "$HOME/.config/dotfiles/config.json" \
    "$HOME/.local/bin/monaco" \
    "$HOME/.local/share/monaco-editor/server.mjs" \
    "$HOME/.local/lib/hyprland/dynamic-cursors.so" \
    "$HOME/.config/quickshell/dotfiles/shell.qml" \
    "/usr/share/sddm/themes/pixie/Main.qml"
do
    check_file "$f" "$(basename "$f")"
done

check_shell "grep -qx 'shell=/usr/bin/fish' \"\$HOME/.config/foot/foot.ini\"" "foot runs fish"
check_shell "fc-match -f '%{file}' 'Material Symbols Rounded' | grep -q ttf" "material symbols rounded resolves"
check_shell "fc-list | grep -qi 'google sans flex'" "google sans flex installed"
check "sddm enabled" systemctl is-enabled sddm
check "networkmanager enabled" systemctl is-enabled NetworkManager
check "hyprland lua parses" luac -p "$HOME/.config/hypr/hyprland.lua"

printf '\n%d ok, %d failed\n' "$ok" "$bad"
[ "$bad" -eq 0 ]
