#!/usr/bin/env bash
# Compares colors/applycolor.sh with dots-applycolor in a bubblewrap sandbox.
# /dev/pts is replaced by a fake directory, so no terminal receives escapes.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
reference_source="$here/reference"
binary="${DOTS_APPLYCOLOR:-$here/../target/release/dots-applycolor}"
[ -x "$binary" ] || binary="$here/../target/debug/dots-applycolor"
[ -x "$binary" ] || { echo "compare: no binary at $binary" >&2; exit 2; }
[ -f "$reference_source/applycolor.sh" ] || { echo "compare: missing vendored reference" >&2; exit 2; }
command -v bwrap >/dev/null || { echo "compare: bwrap is required" >&2; exit 2; }

work=$(mktemp -d /tmp/opencode/dots-applycolor-compare.XXXXXX)
if [ "${KEEP:-0}" = 1 ]; then echo "compare: kept $work"; else trap 'rm -rf "$work"' EXIT; fi

cases=0; matched=0; failed=0
fail() { failed=$((failed + 1)); echo "compare: FAIL: $*" >&2; }
ok() { matched=$((matched + 1)); echo "compare: ok: $*"; }

make_fakes() {
    local root="$1"
    mkdir -p "$root/bin"
    cat > "$root/bin/pgrep" <<'EOF'
#!/usr/bin/env bash
printf 'pgrep %s\n' "$*" >> "$DOTS_LOG_DIR/pgrep.log"
[ "${DOTS_KITTY:-0}" = 1 ] && exit 0
exit 1
EOF
    cat > "$root/bin/kitten" <<'EOF'
#!/usr/bin/env bash
printf 'kitten %s\n' "$*" >> "$DOTS_LOG_DIR/kitten.log"
exit "${DOTS_KITTEN_EXIT:-0}"
EOF
    cat > "$root/bin/pidof" <<'EOF'
#!/usr/bin/env bash
printf 'pidof %s\n' "$*" >> "$DOTS_LOG_DIR/pidof.log"
[ -n "${DOTS_PIDOF:-}" ] && printf '%s\n' "$DOTS_PIDOF"
exit 0
EOF
    chmod +x "$root/bin/"*
}

seed() {
    local root="$1"
    local config="$2"
    local scss="$3"
    local sequences_template="$4"
    mkdir -p \
        "$root/home/.config" \
        "$root/config/dotfiles" \
        "$root/config/quickshell/dotfiles/scripts" \
        "$root/state/quickshell/user/generated" \
        "$root/cache" "$root/run" "$root/log" "$root/bin" \
        "$root/pts"
    make_fakes "$root"
    mkdir -p "$root/config/quickshell/dotfiles/scripts/colors/terminal"
    cp "$reference_source/applycolor.sh" \
        "$root/config/quickshell/dotfiles/scripts/colors/applycolor.sh"
    cp "$reference_source/terminal/sequences.txt" \
        "$root/config/quickshell/dotfiles/scripts/colors/terminal/sequences.txt"
    [ "$sequences_template" = 0 ] && rm -f \
        "$root/config/quickshell/dotfiles/scripts/colors/terminal/sequences.txt"
    printf '%b' "$scss" > "$root/state/quickshell/user/generated/material_colors.scss"
    case "$config" in
        true) printf '{"appearance":{"wallpaperTheming":{"enableTerminal":true}}}\n' \
            > "$root/config/dotfiles/config.json" ;;
        string) printf '{"appearance":{"wallpaperTheming":{"enableTerminal":"true"}}}\n' \
            > "$root/config/dotfiles/config.json" ;;
        false) printf '{"appearance":{"wallpaperTheming":{"enableTerminal":false}}}\n' \
            > "$root/config/dotfiles/config.json" ;;
        invalid) printf 'not json\n' > "$root/config/dotfiles/config.json" ;;
    esac
    printf 'terminal 0\n' > "$root/pts/0"
    printf 'terminal 1\n' > "$root/pts/1"
    printf 'not a terminal\n' > "$root/pts/notapts"
}

run_one() {
    local root="$1"
    shift
    local sleep_pid=""
    if [ "${DOTS_SLEEP:-0}" = 1 ]; then
        sleep 30 &
        sleep_pid=$!
    fi
    set +e
    (
        env HOME="$root/home" \
            XDG_CONFIG_HOME="$root/config" \
            XDG_STATE_HOME="$root/state" \
            XDG_CACHE_HOME="$root/cache" \
            XDG_RUNTIME_DIR="$root/run" \
            DOTS_LOG_DIR="$root/log" \
            DOTS_PTS_DIR="$root/pts" \
            DOTS_PIDOF="$sleep_pid" \
            PATH="$root/bin:/usr/local/bin:/usr/bin:/bin" \
            bwrap --ro-bind / / --bind "$root" "$root" \
                --dev /dev --bind "$root/pts" /dev/pts \
                --unshare-net --die-with-parent --chdir "$root" \
                "$@" > "$root/stdout" 2> "$root/stderr"
    )
    echo $? > "$root/status"
    set -e
    sleep 0.4
    if [ -n "$sleep_pid" ]; then
        if kill -0 "$sleep_pid" 2>/dev/null; then
            echo "alive" > "$root/sleep-state"
            kill "$sleep_pid" 2>/dev/null || true
        else
            echo "signalled" > "$root/sleep-state"
        fi
    fi
}

run_case() {
    local name="$1"
    local config="$2"
    local scss="$3"
    local sequences_template="$4"
    local envs="$5"
    cases=$((cases + 1))
    local ref="$work/$name/reference"
    local rust="$work/$name/rust"
    seed "$ref" "$config" "$scss" "$sequences_template"
    seed "$rust" "$config" "$scss" "$sequences_template"

    set +e
    (
        export DOTS_SLEEP=0 DOTS_PIDOF=""
        eval "$envs"
        run_one "$ref" bash "$ref/config/quickshell/dotfiles/scripts/colors/applycolor.sh"
        run_one "$rust" "$binary"
    )
    set -e

    if [ "$(cat "$ref/status")" != "$(cat "$rust/status")" ]; then
        fail "$name exit $(cat "$ref/status") != $(cat "$rust/status")"
        return
    fi
    if ! diff -ruN "$ref/state" "$rust/state" > "$work/state.diff"; then
        cat "$work/state.diff" >&2
        fail "$name state tree differs"
        return
    fi
    if ! diff -ruN "$ref/pts" "$rust/pts" > "$work/pts.diff"; then
        cat "$work/pts.diff" >&2
        fail "$name fake /dev/pts differs"
        return
    fi
    if [ -f "$ref/sleep-state" ] || [ -f "$rust/sleep-state" ]; then
        if ! diff -u "$ref/sleep-state" "$rust/sleep-state" >&2; then
            fail "$name signal result differs"
            return
        fi
    fi
    sed "s|$ref|ROOT|g" "$ref/stdout" | sort > "$ref/stdout.sorted"
    sed "s|$rust|ROOT|g" "$rust/stdout" | sort > "$rust/stdout.sorted"
    if ! diff -u "$ref/stdout.sorted" "$rust/stdout.sorted" >&2; then
        fail "$name stdout differs"
        return
    fi
    local log
    for log in pgrep pidof; do
        [ -f "$ref/log/$log.log" ] || : > "$ref/log/$log.log"
        [ -f "$rust/log/$log.log" ] || : > "$rust/log/$log.log"
        sed "s|$ref|ROOT|g" "$ref/log/$log.log" | sort > "$ref/log/$log.norm"
        sed "s|$rust|ROOT|g" "$rust/log/$log.log" | sort > "$rust/log/$log.norm"
        if ! diff -u "$ref/log/$log.norm" "$rust/log/$log.norm" >&2; then
            fail "$name $log log differs"
            return
        fi
    done
    ok "$name"
}

scss='$background: #111318;\n$term0: #1e1e2e;\n$term7: #cdd6f4;\n$primary: #89b4fa;\n$alpha: #123456;\n'
run_case enabled "true" "$scss" 1 ""
run_case disabled "false" "$scss" 1 ""
run_case missing-config "" "$scss" 1 ""
run_case string-true "string" "$scss" 1 ""
run_case invalid-config "invalid" "$scss" 1 ""
run_case no-templates "true" "$scss" 0 ""
run_case no-scss "true" "" 1 ""

echo "compare: $cases cases, $matched matched, $failed failed"
[ "$cases" -gt 0 ] && [ "$failed" -eq 0 ]
