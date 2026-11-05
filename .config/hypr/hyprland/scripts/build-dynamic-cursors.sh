#!/usr/bin/env bash
set -euo pipefail

if ! pkg-config --exists hyprland; then
    echo "hyprland headers are missing, install hyprland and try again" >&2
    exit 1
fi

COMMIT=2a8d0b735328d874a78ca93b2f201649f712a74a
DEST="$HOME/.local/lib/hyprland"
TMP="$(mktemp -d)"

git clone --filter=blob:none https://github.com/VirtCode/hypr-dynamic-cursors "$TMP"
cd "$TMP"
git checkout -q "$COMMIT"

sed -i \
    -e 's/prop(NS("mode"), *"tilt"/prop(NS("mode"),                   "tilt"/' \
    -e 's/conf(NS("shake:ipc"), *false/conf(NS("shake:ipc"),              true/' \
    -e 's/conf(NS("shake:threshold"), *6\.0f/conf(NS("shake:threshold"),        3.0f/' \
    -e 's/prop(NS("tilt:limit"), *5000/prop(NS("tilt:limit"),              1500/' \
    -e 's/prop(NS("tilt:full"), *60/prop(NS("tilt:full"),                45/' \
    -e 's/conf(NS("shake:base"), *4\.0f/conf(NS("shake:base"),             2.2f/' \
    -e 's/conf(NS("shake:speed"), *4\.0f/conf(NS("shake:speed"),            1.5f/' \
    -e 's/conf(NS("shake:limit"), *0\.0f/conf(NS("shake:limit"),            3.0f/' \
    -e 's/conf(NS("shake:timeout"), *2000/conf(NS("shake:timeout"),          900/' \
    -e 's/conf(NS("hyprcursor:nearest"), *1/conf(NS("hyprcursor:nearest"),     0/' \
    src/config/ConfigManager.cpp

make all -j"$(nproc)" >/dev/null
strip --strip-debug out/dynamic-cursors.so

mkdir -p "$DEST"
install -m644 out/dynamic-cursors.so "$DEST/dynamic-cursors.so"
rm -rf "$TMP"

echo "installed $DEST/dynamic-cursors.so"
