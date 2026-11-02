#!/usr/bin/env bash
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE=install
FAILED=()
AUR_HELPER=""

for arg in "$@"; do
    case "$arg" in
        --check) MODE=check ;;
        --dry-run) MODE=dry-run ;;
        -h|--help)
            cat <<EOF
usage: install.sh [--check | --dry-run]

  (no flag)   preflight, then install
  --check     preflight only, changes nothing
  --dry-run   preflight, then print the plan without running it
EOF
            exit 0
            ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done

say() { printf '\n==> %s\n' "$*"; }
fail() { printf 'preflight: %s\n' "$*" >&2; exit 1; }

if [ "$(id -u)" -eq 0 ]; then
    echo "run this as your normal user, not root" >&2
    exit 1
fi
command -v pacman >/dev/null || { echo "this is not an Arch system" >&2; exit 1; }

# ---------------------------------------------------------------- preflight

say "preflight"

[ -d /etc/pacman.d ] || fail "pacman configuration is missing"

net_ok() { curl -fsS -m 10 -o /dev/null "$1"; }
net_ok https://archlinux.org/ || fail "cannot reach archlinux.org"
net_ok https://aur.archlinux.org/rpc/ || fail "cannot reach aur.archlinux.org"
net_ok https://dl.flathub.org/ || fail "cannot reach flathub.org"
net_ok https://github.com/ || fail "cannot reach github.com"

updates="$(pacman -Qu 2>/dev/null | wc -l || true)"
if [ "$updates" -gt 0 ]; then
    fail "$updates pending updates. Run 'sudo pacman -Syu' first: installing against a stale system breaks Arch"
fi

for f in \
    "$REPO/.config/hypr/hyprland/colors.lua" \
    "$REPO/.config/kitty/colors.conf" \
    "$REPO/extras/dotfiles-config.json" \
    "$REPO/extras/install.sh" \
    "$REPO/extras/build-dynamic-cursors.sh" \
    "$REPO/extras/make-material-symbols-filled.py" \
    "$REPO/extras/fonts/google-sans-flex" \
    "$REPO/extras/sddm/pixie/Main.qml" \
    "$REPO/extras/packages-explicit.txt" \
    "$REPO/extras/flatpaks.txt"
do
    [ -e "$f" ] || fail "missing in the repo: ${f#$REPO/}"
done

available_kb="$(df -Pk "$HOME" | awk 'NR==2{print $4}')"
[ "$available_kb" -gt 4000000 ] || fail "less than 4 GB free in $HOME"

mapfile -t WANTED < <(grep -v '^[[:space:]]*$' "$REPO/extras/packages-explicit.txt")
REPO_PKGS=()
AUR_PKGS=()
UNKNOWN=()
for p in "${WANTED[@]}"; do
    if pacman -Si "$p" >/dev/null 2>&1; then
        REPO_PKGS+=("$p")
    else
        AUR_PKGS+=("$p")
    fi
done

if [ ${#AUR_PKGS[@]} -gt 0 ]; then
    query=""
    for p in "${AUR_PKGS[@]}"; do query+="arg[]=$p&"; done
    aur_json="$(curl -fsS -m 30 "https://aur.archlinux.org/rpc/v5/info?${query}")" || fail "AUR lookup failed"
    for p in "${AUR_PKGS[@]}"; do
        printf '%s' "$aur_json" | grep -q "\"Name\":\"$p\"" || UNKNOWN+=("$p")
    done
fi
[ ${#UNKNOWN[@]} -eq 0 ] || fail "not in any repository: ${UNKNOWN[*]}"

resolve_log="$(mktemp)"
if ! pacman -S --print --print-format '%n %v' "${REPO_PKGS[@]}" >/dev/null 2>"$resolve_log"; then
    echo "pacman cannot resolve the package set:" >&2
    head -5 "$resolve_log" >&2
    rm -f "$resolve_log"
    fail "package resolution failed"
fi
if grep -qi "conflict" "$resolve_log"; then
    echo "pacman reports conflicts:" >&2
    grep -i "conflict" "$resolve_log" | head -5 >&2
    rm -f "$resolve_log"
    fail "conflicting packages"
fi
rm -f "$resolve_log"

mapfile -t FLATPAKS < <(cut -f1 "$REPO/extras/flatpaks.txt" | grep -v '^[[:space:]]*$')
BAD_FLATPAK=()
for app in "${FLATPAKS[@]}"; do
    curl -fsS -m 20 -o /dev/null "https://flathub.org/api/v2/summary/$app" || BAD_FLATPAK+=("$app")
done
[ ${#BAD_FLATPAK[@]} -eq 0 ] || fail "not on flathub: ${BAD_FLATPAK[*]}"

gray_asset="gray-stable-x86_64-linux.tar.gz"
gray_base="https://github.com/vstaln/gray/releases/latest/download"
curl -fsS -m 30 -o /dev/null -r 0-1024 "$gray_base/$gray_asset" || fail "gray release is not downloadable"
curl -fsS -m 30 -o /dev/null -r 0-256 "$gray_base/SHA256SUMS" || fail "gray checksum file is not downloadable"

printf 'packages: %d from repo, %d from AUR\n' "${#REPO_PKGS[@]}" "${#AUR_PKGS[@]}"
printf 'flatpaks: %d\n' "${#FLATPAKS[@]}"
echo "preflight passed"

if [ "$MODE" = check ]; then
    exit 0
fi

if [ "$MODE" = dry-run ]; then
    say "plan"
    printf 'install repo packages:\n  %s\n' "${REPO_PKGS[*]}"
    printf 'install AUR packages:\n  %s\n' "${AUR_PKGS[*]}"
    printf 'install flatpaks:\n  %s\n' "${FLATPAKS[*]}"
    cat <<EOF

copy $REPO/.config into ~/.config (existing files are backed up first)
copy $REPO/.local into ~/.local
install fonts, generate Material Symbols Filled, refresh the font cache
copy the pixie SDDM theme and select it
build and install hypr-dynamic-cursors
download gray and verify its checksum
npm install -g monaco-editor
enable sddm NetworkManager bluetooth fstrim.timer
run matugen on the saved wallpaper
EOF
    exit 0
fi

sudo -v || fail "sudo is required"

# ---------------------------------------------------------------- install

say "base tools"
sudo pacman -S --needed --noconfirm base-devel git rsync

say "AUR helper"
if command -v paru >/dev/null; then
    AUR_HELPER=paru
elif command -v yay >/dev/null; then
    AUR_HELPER=yay
else
    tmp="$(mktemp -d)"
    git clone --depth=1 https://aur.archlinux.org/paru-bin.git "$tmp/paru-bin"
    (cd "$tmp/paru-bin" && makepkg -si --noconfirm)
    rm -rf "$tmp"
    AUR_HELPER=paru
fi

say "packages"
sudo pacman -S --needed --noconfirm "${REPO_PKGS[@]}"
for p in "${AUR_PKGS[@]}"; do
    "$AUR_HELPER" -S --needed --noconfirm "$p" || FAILED+=("$p")
done

say "flatpak"
sudo flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
for app in "${FLATPAKS[@]}"; do
    sudo flatpak install -y --noninteractive flathub "$app" || FAILED+=("$app")
done

say "configs"
backup="$HOME/dotfiles-preinstall-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$backup"
for d in "$REPO"/.config/*; do
    name="$(basename "$d")"
    [ -e "$HOME/.config/$name" ] && cp -a "$HOME/.config/$name" "$backup/" 2>/dev/null || true
done
rsync -a "$REPO/.config/" "$HOME/.config/"
rsync -a "$REPO/.local/" "$HOME/.local/"
if [ ! -f "$HOME/.config/dotfiles/config.json" ] && [ -f "$REPO/extras/dotfiles-config.json" ]; then
    mkdir -p "$HOME/.config/dotfiles"
    sed "s|@HOME@|$HOME|g" "$REPO/extras/dotfiles-config.json" > "$HOME/.config/dotfiles/config.json"
fi
chmod +x "$HOME/.local/bin/"* "$HOME/.config/hypr/hyprland/scripts/"* 2>/dev/null || true
echo "previous config saved to $backup"

say "fonts and login theme"
mkdir -p "$HOME/.local/share/fonts"
cp -a "$REPO/extras/fonts/google-sans-flex" "$HOME/.local/share/fonts/"
"$REPO/extras/make-material-symbols-filled.py"
fc-cache -f >/dev/null
sudo cp -a "$REPO/extras/sddm/pixie" /usr/share/sddm/themes/
sudo mkdir -p /etc/sddm.conf.d
printf '[Theme]\nCurrent=pixie\n' | sudo tee /etc/sddm.conf.d/20-pixie.conf >/dev/null

say "cursor plugin"
"$REPO/extras/build-dynamic-cursors.sh" || FAILED+=("hypr-dynamic-cursors")

say "gray"
if [ ! -x "$HOME/.local/bin/gray" ]; then
    tmp="$(mktemp -d)"
    if curl -fsSL -o "$tmp/$gray_asset" "$gray_base/$gray_asset" &&
       curl -fsSL -o "$tmp/SHA256SUMS" "$gray_base/SHA256SUMS" &&
       ( cd "$tmp" && grep " $gray_asset\$" SHA256SUMS | sha256sum -c - >/dev/null ); then
        mkdir -p "$HOME/.local/bin"
        tar xzf "$tmp/$gray_asset" -C "$HOME/.local/bin" gray && chmod +x "$HOME/.local/bin/gray"
    else
        FAILED+=("gray")
    fi
    rm -rf "$tmp"
fi

say "editor"
npm config set prefix "$HOME/.local" >/dev/null 2>&1 || true
npm install -g monaco-editor || FAILED+=("monaco-editor")

say "shell and services"
if systemctl list-unit-files sddm.service >/dev/null 2>&1; then
    sudo systemctl enable sddm
else
    echo "sddm is not installed, not enabling the display manager" >&2
    FAILED+=("sddm")
fi
sudo systemctl enable NetworkManager bluetooth fstrim.timer

say "colors"
if command -v matugen >/dev/null && command -v jq >/dev/null; then
    wallpaper="$(jq -r '.wallpaper.path // empty' "$HOME/.config/dotfiles/config.json" 2>/dev/null)"
    if [ -n "$wallpaper" ] && [ -f "$wallpaper" ]; then
        matugen image "$wallpaper" -m dark || true
    else
        echo "no wallpaper yet, colors stay as the committed seeds"
    fi
fi

say "done"
if [ ${#FAILED[@]} -gt 0 ]; then
    printf 'failed: %s\n' "${FAILED[*]}"
    exit 1
fi

"$REPO/extras/verify.sh"
echo "reboot to start the session"
