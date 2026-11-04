#!/usr/bin/env bash
# Restore the packages that `pacman -Rns` took out with the illogical-impulse
# metapackages on 2026-11-05. Run as root: sudo ./extras/restore-packages.sh
#
# `-Rns` removes dependencies nothing else requires, and the metapackages were the
# only thing declaring kitty, hyprland, the lock/idle daemons and the screenshot
# tools. The running session keeps working from memory, but a restart without this
# leaves the machine with no compositor and no terminal.
set -euo pipefail

[[ $EUID -eq 0 ]] || { echo "run with sudo" >&2; exit 1; }

repo_packages=(
    bc brightnessctl clang cliphist cmake ddcutil eza fish geoclue gnome-keyring
    gtk-layer-shell hypridle hyprland hyprlock hyprpicker hyprshot hyprsunset
    imagemagick jq kitty matugen rsync songrec starship swappy tesseract
    ttf-jetbrains-mono-nerd uv wf-recorder wget wl-clipboard
    xdg-desktop-portal-hyprland ydotool
)

aur_packages=(
    adw-gtk-theme-git bibata-cursor-theme otf-space-grotesk
    ttf-material-symbols-variable-git
)

echo "==> repo packages"
pacman -S --needed --noconfirm "${repo_packages[@]}"

echo "==> services"
systemctl enable --now bluetooth NetworkManager 2>/dev/null || true

echo "==> done: $(pacman -Qq | wc -l) packages installed"
echo
echo "Four of them come from the AUR; run this as your own user:"
echo "  paru -S --needed ${aur_packages[*]}"
