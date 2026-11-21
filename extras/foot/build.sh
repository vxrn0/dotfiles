#!/usr/bin/env bash
# Builds foot with the local patch that keeps the close animation alive.
#
#   ./build.sh            build the installed foot version
#   ./build.sh 1.29.0     build a specific version
#   ./build.sh master     build the current development branch
#
# The result goes to ~/.local/bin/foot, which comes before /usr/bin in PATH,
# so Hyprland and every shell pick it up. Re-run after `pacman -Syu` updates
# foot; a full build takes about 5 seconds.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
patch_file="$here/0001-keep-the-window-mapped.patch"
prefix="${FOOT_PREFIX:-$HOME/.local}"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/dotfiles-foot"

version="${1:-}"
if [ -z "$version" ]; then
    version=$(pacman -Q foot | awk '{print $2}' | sed 's/-[0-9]*$//')
fi

for tool in meson ninja pkg-config curl patch; do
    if ! command -v "$tool" >/dev/null; then
        echo "build.sh: missing $tool; install it with:" >&2
        echo "  sudo pacman -S meson ninja" >&2
        exit 1
    fi
done

archive="$cache/foot-$version.tar.gz"
source_dir="$cache/foot-$version"
build_dir="$cache/build-$version"

mkdir -p "$cache"
if [ ! -f "$archive" ]; then
    echo "build.sh: downloading foot $version"
    curl -fsSL -o "$archive.part" "https://codeberg.org/dnkl/foot/archive/$version.tar.gz"
    mv "$archive.part" "$archive"
fi

rm -rf "$source_dir" "$build_dir"
mkdir -p "$source_dir"
tar xzf "$archive" -C "$source_dir" --strip-components=1

echo "build.sh: applying $(basename "$patch_file")"
if ! patch -d "$source_dir" -p1 --forward < "$patch_file"; then
    echo "build.sh: the patch no longer applies to foot $version." >&2
    echo "build.sh: check upstream first; it may have fixed this already." >&2
    exit 1
fi

meson setup "$build_dir" "$source_dir" \
    --buildtype=release \
    -Ddocs=disabled \
    -Dtests=false \
    -Dterminfo=disabled \
    -Dthemes=false >/dev/null
ninja -C "$build_dir" >/dev/null

mkdir -p "$prefix/bin"
install -m755 "$build_dir/foot" "$prefix/bin/foot"

echo "build.sh: installed $("$prefix/bin/foot" --version | head -1) to $prefix/bin/foot"
if [ "$(command -v foot)" != "$prefix/bin/foot" ]; then
    echo "build.sh: warning: 'foot' resolves to $(command -v foot), not the patched binary" >&2
fi
