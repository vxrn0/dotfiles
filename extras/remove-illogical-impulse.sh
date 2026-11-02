#!/usr/bin/env bash
# Removes the illogical-impulse metapackages from an existing install.
# A fresh install never needs this: extras/packages-explicit.txt has no ii packages.
set -euo pipefail

KEEP="adw-gtk-theme-git bc brightnessctl clang cliphist cmake ddcutil eza fish geoclue
hypridle hyprland hyprlock hyprpicker hyprshot hyprsunset imagemagick kitty matugen
otf-space-grotesk rsync songrec starship swappy ttf-jetbrains-mono-nerd
ttf-material-symbols-variable-git uv wget wf-recorder xdg-desktop-portal-hyprland ydotool
nodejs npm gtk-layer-shell rtkit qt6-imageformats"

if ! pacman -Qq | grep -q '^illogical-impulse'; then
    echo "no illogical-impulse packages installed, nothing to do"
    exit 0
fi

echo "== marking packages that must survive as explicitly installed"
sudo pacman -D --asexplicit $KEEP

echo "== replacing the cursor theme"
if pacman -Qq | grep -qx illogical-impulse-bibata-modern-classic-bin; then
    sudo pacman -R --noconfirm illogical-impulse-bibata-modern-classic-bin
    paru -S --needed --noconfirm bibata-cursor-theme
fi

echo "== removing the metapackages"
sudo pacman -R --noconfirm $(pacman -Qq | grep '^illogical-impulse')

echo "== orphans left behind (review before removing)"
pacman -Qdtq || true

cat <<'EOF'

Next: check that list. Safe to drop: fuzzel, wlogout, breeze-plus, darkly-bin,
translate-shell, ttf-readex-pro, ttf-rubik-vf, ttf-twemoji, wtype, go-yq,
gtksourceviewmm, gamemode, dart-sass, glusterfs.

Run: sudo pacman -Rns <those names>
Then: ./extras/verify.sh
EOF
