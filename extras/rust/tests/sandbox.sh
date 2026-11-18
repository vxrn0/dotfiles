#!/usr/bin/env bash
# Runs the real switchwall.sh and dots-theme in a redirected HOME/XDG tree and
# compares the resulting state plus the matugen/applycolor argument captures.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$(cd "$HERE/.." && pwd)"
REPO="$(cd "$RUST_DIR/../.." && pwd)"
COLORS_REPO="$REPO/.config/quickshell/dotfiles/scripts/colors"
BIN="${DOTS_THEME_BIN:-$RUST_DIR/target/release/dots-theme}"
REAL_HOME="${HOME:-}"
VENV="${DOTFILES_VIRTUAL_ENV:-$REAL_HOME/.local/state/quickshell/.venv}"
MATUGEN_REAL="$(command -v matugen || true)"
WORK="${WORK:-$(mktemp -d /tmp/opencode/dots-theme-sandbox.XXXXXX)}"
KEEP="${KEEP:-0}"
ROOT="$WORK/root"
HOME_DIR="$ROOT/home"
OUT="$WORK/out"
FIXTURES="$WORK/fixtures"

fail() {
    echo "sandbox: FAIL: $*" >&2
    exit 1
}

cleanup() {
    if [ "$KEEP" = 1 ]; then
        echo "sandbox: kept $WORK"
    else
        rm -rf "$WORK"
    fi
}
trap cleanup EXIT

case "$BIN" in
    /*) ;;
    *) BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")" ;;
esac
case "$VENV" in
    /*) ;;
    *) VENV="$(cd "$VENV" && pwd)" ;;
esac

[ -x "$BIN" ] || fail "missing $BIN; run: cargo build --release --offline"
[ -x "$VENV/bin/python3" ] || fail "missing venv python at $VENV/bin/python3"
[ -n "$MATUGEN_REAL" ] || fail "matugen is not installed"
for program in jq ffmpeg identify flock tar diff; do
    command -v "$program" >/dev/null || fail "missing required tool: $program"
done
[ -f "$COLORS_REPO/switchwall.sh" ] || fail "missing $COLORS_REPO/switchwall.sh"

mkdir -p "$FIXTURES" "$OUT"
ffmpeg -loglevel error -y -f lavfi -i "color=c=0x336699:s=3000x2000" -frames:v 1 "$FIXTURES/still.png"
ffmpeg -loglevel error -y -f lavfi -i "testsrc=size=320x240:rate=10:duration=1" -pix_fmt yuv420p "$FIXTURES/clip.mp4"

make_fake_bin() {
    local bin="$ROOT/bin"
    mkdir -p "$bin"
    cat > "$bin/hyprctl" <<'EOF'
#!/usr/bin/env bash
printf 'hyprctl %s\n' "$*" >> "$DOTS_LOG_DIR/hyprctl.log"
if [ "$1" = monitors ]; then
    printf '%s\n' '[{"id":0,"name":"HDMI-A-1","width":1920,"height":1080,"x":0,"y":0,"scale":1.0,"focused":false},{"id":1,"name":"DP-1","width":2560,"height":1440,"x":1920,"y":0,"scale":1.0,"focused":true}]'
elif [ "$1" = cursorpos ]; then
    printf '%s\n' '{"x":100,"y":200}'
else
    exit 1
fi
EOF
    cat > "$bin/matugen" <<EOF
#!/usr/bin/env bash
printf 'matugen %s\\n' "\$*" >> "\$DOTS_LOG_DIR/matugen.log"
exec "$MATUGEN_REAL" "\$@"
EOF
    cat > "$bin/gsettings" <<'EOF'
#!/usr/bin/env bash
printf 'gsettings %s\n' "$*" >> "$DOTS_LOG_DIR/gsettings.log"
if [ "$1" = get ]; then
    printf "'prefer-dark'\n"
fi
exit 0
EOF
    cat > "$bin/mpvpaper" <<'EOF'
#!/usr/bin/env bash
printf 'mpvpaper %s\n' "$*" >> "$DOTS_LOG_DIR/mpvpaper.log"
exit 0
EOF
    cat > "$bin/pkill" <<'EOF'
#!/usr/bin/env bash
printf 'pkill %s\n' "$*" >> "$DOTS_LOG_DIR/pkill.log"
exit 0
EOF
    cat > "$bin/pgrep" <<'EOF'
#!/usr/bin/env bash
printf 'pgrep %s\n' "$*" >> "$DOTS_LOG_DIR/pgrep.log"
exit 1
EOF
    cat > "$bin/notify-send" <<'EOF'
#!/usr/bin/env bash
printf 'notify-send %s\n' "$*" >> "$DOTS_LOG_DIR/notify-send.log"
exit 0
EOF
    cat > "$bin/xdg-user-dir" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$HOME/Pictures"
EOF
    for program in kitty upscayl kdialog zenity; do
        cat > "$bin/$program" <<EOF
#!/usr/bin/env bash
printf '$program %s\\n' "\$*" >> "\$DOTS_LOG_DIR/$program.log"
exit 1
EOF
    done
    chmod +x "$bin"/*
}

seed_tree() {
    rm -rf "$ROOT"
    mkdir -p "$HOME_DIR/.config" "$HOME_DIR/.local/state" "$HOME_DIR/.cache" "$ROOT/runtime"
    mkdir -p "$HOME_DIR/.config/dotfiles"
    jq '.appearance.wallpaperTheming.enableTerminal = false' \
        "$REPO/extras/dotfiles-config.json" \
        | sed "s|@HOME@|$HOME_DIR|g" \
        > "$HOME_DIR/.config/dotfiles/config.json"

    cp -a "$REPO/.config/matugen" "$HOME_DIR/.config/matugen"
    mkdir -p \
        "$HOME_DIR/.config/hypr/custom/scripts" \
        "$HOME_DIR/.config/hypr/hyprland" \
        "$HOME_DIR/.config/hypr/hyprlock" \
        "$HOME_DIR/.config/gtk-3.0" \
        "$HOME_DIR/.config/gtk-4.0" \
        "$HOME_DIR/.config/kitty" \
        "$HOME_DIR/.config/quickshell/dotfiles/scripts"
    cp -a "$COLORS_REPO" "$HOME_DIR/.config/quickshell/dotfiles/scripts/colors"
    cmp -s "$COLORS_REPO/switchwall.sh" \
        "$HOME_DIR/.config/quickshell/dotfiles/scripts/colors/switchwall.sh" \
        || fail "switchwall.sh copy differs from the repository"
    cp "$REPO/.config/hypr/custom/scripts/__restore_video_wallpaper.sh" \
        "$HOME_DIR/.config/hypr/custom/scripts/__restore_video_wallpaper.sh"

    # The wrapper records the invocation and then runs the real helper.
    local helper="$HOME_DIR/.config/quickshell/dotfiles/scripts/colors/applycolor.sh"
    cat > "$helper" <<EOF
#!/usr/bin/env bash
printf 'applycolor %s\\n' "\$*" >> "\$DOTS_LOG_DIR/applycolor.log"
exec /usr/bin/env bash "$COLORS_REPO/applycolor.sh" "\$@"
EOF
    chmod +x "$helper"
    make_fake_bin
}

snapshot() {
    local engine="$1"
    local destination="$OUT/$CASE/$engine"
    rm -rf "$destination"
    mkdir -p "$destination"
    (cd "$ROOT" && tar --exclude=./bin -cf - .) | tar -C "$destination" -xf -

    local restore="$destination/home/.config/hypr/custom/scripts/__restore_video_wallpaper.sh"
    if [ -f "$restore" ]; then
        sed -i 's/^# Time: .*/# Time: NORMALIZED/' "$restore"
    fi

    local log
    for log in matugen applycolor gsettings mpvpaper; do
        if [ -f "$LOG_DIR/$log.log" ]; then
            cp "$LOG_DIR/$log.log" "$destination/.log-$log"
        else
            : > "$destination/.log-$log"
        fi
    done
}

run_one() {
    local engine="$1"
    shift
    LOG_DIR="$WORK/logs/$CASE/$engine"
    rm -rf "$LOG_DIR"
    mkdir -p "$LOG_DIR"

    local status
    set +e
    (
        cd "$ROOT"
        env \
            HOME="$HOME_DIR" \
            XDG_CONFIG_HOME="$HOME_DIR/.config" \
            XDG_STATE_HOME="$HOME_DIR/.local/state" \
            XDG_CACHE_HOME="$HOME_DIR/.cache" \
            XDG_RUNTIME_DIR="$ROOT/runtime" \
            DOTFILES_VIRTUAL_ENV="$VENV" \
            DOTS_LOG_DIR="$LOG_DIR" \
            PATH="$ROOT/bin:/usr/local/bin:/usr/bin:/bin" \
            "$@" > "$LOG_DIR/stdout.log" 2> "$LOG_DIR/stderr.log"
    )
    status=$?
    set -e
    if [ "$status" -ne 0 ]; then
        echo "sandbox: $CASE/$engine exited $status" >&2
        cat "$LOG_DIR/stdout.log" >&2 || true
        cat "$LOG_DIR/stderr.log" >&2 || true
        fail "$CASE/$engine"
    fi
    sleep 0.4
    snapshot "$engine"
}

run_case() {
    CASE="$1"
    shift
    echo "sandbox: case $CASE"

    seed_tree
    run_one bash /usr/bin/bash \
        "$HOME_DIR/.config/quickshell/dotfiles/scripts/colors/switchwall.sh" "$@"

    seed_tree
    run_one rust "$BIN" "$@"

    if ! diff -ruN "$OUT/$CASE/bash" "$OUT/$CASE/rust" > "$OUT/$CASE/tree.diff"; then
        cat "$OUT/$CASE/tree.diff" >&2
        fail "$CASE tree differs"
    fi

    (cd "$OUT/$CASE/bash" && find . -printf '%m %P\n' | sort) > "$OUT/$CASE/bash.modes"
    (cd "$OUT/$CASE/rust" && find . -printf '%m %P\n' | sort) > "$OUT/$CASE/rust.modes"
    if ! diff -u "$OUT/$CASE/bash.modes" "$OUT/$CASE/rust.modes" > "$OUT/$CASE/modes.diff"; then
        cat "$OUT/$CASE/modes.diff" >&2
        fail "$CASE file modes differ"
    fi

    echo "sandbox: $CASE ok (tree, modes, matugen/applycolor/gsettings/mpvpaper logs match)"
}

run_case image "$FIXTURES/still.png"
run_case color --color '#336699'
run_case video "$FIXTURES/clip.mp4"

echo "sandbox: all cases match"
