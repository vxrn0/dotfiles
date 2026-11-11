#!/usr/bin/env bash
# Installs Handy into ~/.local without touching the system. Downloads once into a cache
# directory and reuses it on later runs, because the release and WebKit together are ~77 MB.
set -euo pipefail

VERSION="0.10.0"
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/handy-install"
PREFIX="$HOME/.local/opt/handy"
BASE="https://geo.mirror.pkgbuild.com/extra/os/x86_64"
PACKAGES=(
    "webkit2gtk-4.1-2.54.2-1-x86_64.pkg.tar.zst"
    "openblas-0.3.34-1-x86_64.pkg.tar.zst"
    "enchant-2.8.21-1-x86_64.pkg.tar.zst"
    "hyphen-2.8.10-1-x86_64.pkg.tar.zst"
    "libmanette-0.2.13-2-x86_64.pkg.tar.zst"
    "woff2-1.0.2-7-x86_64.pkg.tar.zst"
    "wtype-0.4-2-x86_64.pkg.tar.zst"
)

mkdir -p "$CACHE"

fetch() {
    local url="$1" out="$2"
    [ -f "$out" ] && return 0
    echo "downloading $(basename "$out")"
    curl -fL --retry 3 -o "$out.part" "$url"
    mv "$out.part" "$out"
}

for pkg in "${PACKAGES[@]}"; do
    fetch "$BASE/$pkg" "$CACHE/$pkg"
done

fetch "https://archive.archlinux.org/packages/h/harfbuzz-icu/harfbuzz-icu-15.0.0-1-x86_64.pkg.tar.zst" \
    "$CACHE/harfbuzz-icu.pkg.tar.zst"

fetch "https://github.com/cjpais/Handy/releases/download/v$VERSION/Handy_${VERSION}_amd64.deb" \
    "$CACHE/Handy.deb"

expected="89c31dd9a969fdbd6cb6accf282af98391371cd6756e1a5f1b0b301089a02752a77684c3a8373456cc3f9ede6e340d3be3d277bffe0b44cbdde36609da8cec30"
actual="$(b2sum "$CACHE/Handy.deb" | cut -d' ' -f1)"
if [ "$actual" != "$expected" ]; then
    echo "Handy checksum mismatch, refusing to install" >&2
    exit 1
fi

rm -rf "$PREFIX"
mkdir -p "$PREFIX"
for pkg in "$CACHE"/*.pkg.tar.zst; do
    bsdtar -xf "$pkg" -C "$PREFIX" --strip-components=1
done

mkdir -p "$PREFIX/app"
if [ ! -f "$CACHE/deb/usr/bin/handy" ]; then
    mkdir -p "$CACHE/deb"
    bsdtar -xf "$CACHE/Handy.deb" -C "$CACHE/deb" data.tar.gz
    tar xzf "$CACHE/deb/data.tar.gz" -C "$CACHE/deb"
fi
cp -a "$CACHE/deb/usr/lib/Handy/." "$PREFIX/app/"
cp "$CACHE/deb/usr/bin/handy" "$PREFIX/app/handy"
chmod +x "$PREFIX/app/handy"

mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications" "$HOME/.local/share/icons"
cp -a "$CACHE/deb/usr/share/icons/hicolor" "$HOME/.local/share/icons/" 2>/dev/null || true
cp "$PREFIX/bin/wtype" "$HOME/.local/bin/wtype"
chmod +x "$HOME/.local/bin/wtype"

cat > "$HOME/.local/bin/handy" <<'EOF'
#!/usr/bin/env bash
PREFIX="$HOME/.local/opt/handy"
export LD_LIBRARY_PATH="$PREFIX/lib:$PREFIX/lib64:$PREFIX/app:${LD_LIBRARY_PATH:-}"
export WEBKIT_EXEC_PATH="$PREFIX/lib/webkit2gtk-4.1"
export PATH="$HOME/.local/bin:$PATH"
exec "$PREFIX/app/handy" "$@"
EOF
chmod +x "$HOME/.local/bin/handy"

cat > "$HOME/.local/share/applications/handy.desktop" <<'EOF'
[Desktop Entry]
Categories=Utility;AudioVideo;
Comment=Offline speech to text
Exec=handy
StartupWMClass=handy
Icon=handy
Name=Handy
Terminal=false
Type=Application
EOF

echo "installed to $PREFIX"
echo "run: handy"
