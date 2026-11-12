#!/usr/bin/env bash
# Installs Handy (offline speech to text). System libraries come from pacman; the app itself
# is unpacked from the release. Layout matters: Tauri resolves bundled resources relative to
# the binary as ../lib/Handy, so the binary lives in bin/ and everything else in lib/Handy/.
# The hotkey is a Hyprland bind (SUPER+ALT+D), not the app's own one, which is X11-only.
set -euo pipefail

VERSION="0.10.0"
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/handy-install"
PREFIX="$HOME/.local/opt/handy"
DEB="$CACHE/Handy_${VERSION}_amd64.deb"
EXPECTED="89c31dd9a969fdbd6cb6accf282af98391371cd6756e1a5f1b0b301089a02752a77684c3a8373456cc3f9ede6e340d3be3d277bffe0b44cbdde36609da8cec30"

sudo pacman -S --needed --noconfirm \
    webkit2gtk-4.1 openblas enchant hyphen libmanette woff2 wtype

mkdir -p "$CACHE"
if [ ! -f "$DEB" ]; then
    echo "downloading Handy $VERSION"
    curl -fL --retry 3 -o "$DEB.part" \
        "https://github.com/cjpais/Handy/releases/download/v$VERSION/Handy_${VERSION}_amd64.deb"
    mv "$DEB.part" "$DEB"
fi

actual="$(b2sum "$DEB" | cut -d' ' -f1)"
if [ "$actual" != "$EXPECTED" ]; then
    echo "Handy checksum mismatch, refusing to install" >&2
    exit 1
fi

rm -rf "$PREFIX"
mkdir -p "$PREFIX/bin" "$PREFIX/lib"
bsdtar -xf "$DEB" -C "$CACHE" data.tar.gz
tar xzf "$CACHE/data.tar.gz" -C "$CACHE" usr/lib/Handy usr/bin/handy
cp -a "$CACHE/usr/lib/Handy" "$PREFIX/lib/Handy"
cp "$CACHE/usr/bin/handy" "$PREFIX/bin/handy"
chmod +x "$PREFIX/bin/handy"

mkdir -p "$HOME/.local/bin"
cat > "$HOME/.local/bin/handy" <<'EOF'
#!/usr/bin/env bash
PREFIX="$HOME/.local/opt/handy"
export LD_LIBRARY_PATH="$PREFIX/lib/Handy:${LD_LIBRARY_PATH:-}"
export PATH="$HOME/.local/bin:$PATH"
exec "$PREFIX/bin/handy" "$@"
EOF
chmod +x "$HOME/.local/bin/handy"

echo "installed to $PREFIX"
echo "start it from the Hyprland autostart, toggle with SUPER+ALT+D"
echo "models go to ~/.cache/huggingface/hub; download them from the app's settings"
