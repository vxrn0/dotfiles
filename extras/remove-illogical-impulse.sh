#!/usr/bin/env bash
# Removes the illogical-impulse metapackages from an existing install.
# A fresh install never needs this: extras/packages-explicit.txt has no ii packages.
set -euo pipefail

KEEP="adw-gtk-theme-git bc brightnessctl clang cliphist cmake ddcutil eza fish geoclue
hypridle hyprland hyprlock hyprpicker hyprshot hyprsunset imagemagick kitty matugen
otf-space-grotesk rsync songrec starship swappy ttf-jetbrains-mono-nerd
ttf-material-symbols-variable-git uv wget wf-recorder xdg-desktop-portal-hyprland ydotool
nodejs npm gtk-layer-shell rtkit qt6-imageformats
# bluetooth: the panel's toggle, the bar icon and the device dialog all read bluez over
# D-Bus. bluez arrives only as a dependency of bluedevil, so removing bluedevil with
# -Rns takes bluetooth with it and the toggle silently disappears.
bluez bluez-utils"

if ! pacman -Qq | grep -q '^illogical-impulse'; then
    echo "no illogical-impulse packages installed, nothing to do"
    exit 0
fi

installed="$(pacman -Qq)"
mark=()
for p in $KEEP; do
    grep -qx "$p" <<<"$installed" && mark+=("$p")
done

echo "== marking ${#mark[@]} packages that must survive as explicitly installed"
sudo pacman -D --asexplicit "${mark[@]}"

echo "== replacing the cursor theme"
if pacman -Qq | grep -qx illogical-impulse-bibata-modern-classic-bin; then
    if ! command -v paru >/dev/null && ! command -v yay >/dev/null; then
        echo "paru or yay is required to install bibata-cursor-theme" >&2
        exit 1
    fi
    helper="$(command -v paru >/dev/null && echo paru || echo yay)"
    sudo pacman -R --noconfirm illogical-impulse-bibata-modern-classic-bin
    "$helper" -S --needed --noconfirm bibata-cursor-theme
fi

echo "== removing the metapackages"
sudo pacman -R --noconfirm $(pacman -Qq | grep '^illogical-impulse')

echo "== orphans left behind (review before removing)"
pacman -Qdtq || true

cat <<'EOF'

Next: review that list. Safe to drop: fuzzel, wlogout, breeze-plus, darkly-bin,
translate-shell, ttf-readex-pro, ttf-rubik-vf, ttf-twemoji, wtype, go-yq,
gtksourceviewmm, gamemode, dart-sass, glusterfs, bluedevil.

bluedevil is a Plasma applet this setup does not use, but `-Rns` follows it to
bluez-qt and bluez, and then the Bluetooth toggle, the bar icon and the device dialog
all vanish. Mark the protocol stack explicit first or leave bluedevil installed:
`pacman -D --asexplicit bluez bluez-utils`.

Run: sudo pacman -Rns <those names>
Then: ~/extra/git/dotfiles/extras/verify.sh
EOF
