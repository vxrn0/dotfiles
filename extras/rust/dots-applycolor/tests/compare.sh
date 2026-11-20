#!/usr/bin/env bash
# Compares colors/applycolor.sh with dots-applycolor in a bubblewrap sandbox.
# /dev/pts is replaced by a fake directory, so no terminal receives escapes.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
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

seed() {
    local root="$1"
    local config="$2"
    local sequences="$3"
    mkdir -p \
        "$root/home/.config" \
        "$root/config/dotfiles" \
        "$root/state/quickshell/user/generated/terminal" \
        "$root/log" \
        "$root/pts"
    mkdir -p "$root/config/quickshell/dotfiles/scripts/colors"
    cp "$reference_source/applycolor.sh" \
        "$root/config/quickshell/dotfiles/scripts/colors/applycolor.sh"
    if [ "$sequences" = 1 ]; then
        printf '\033]4;0;#32353a\033\\\\\033]11;#111318\033\\\\' \
            > "$root/state/quickshell/user/generated/terminal/sequences.txt"
    fi
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
    set +e
    (
        env HOME="$root/home" \
            XDG_CONFIG_HOME="$root/config" \
            XDG_STATE_HOME="$root/state" \
            DOTS_PTS_DIR="$root/pts" \
            PATH=/usr/local/bin:/usr/bin:/bin \
            bwrap --ro-bind / / --bind "$root" "$root" \
                --dev /dev --bind "$root/pts" /dev/pts \
                --unshare-net --die-with-parent --chdir "$root" \
                "$@" > "$root/stdout" 2> "$root/stderr"
    )
    echo $? > "$root/status"
    set -e
}

run_case() {
    local name="$1"
    local config="$2"
    local sequences="$3"
    cases=$((cases + 1))
    local ref="$work/$name/reference"
    local rust="$work/$name/rust"
    seed "$ref" "$config" "$sequences"
    seed "$rust" "$config" "$sequences"

    run_one "$ref" bash "$ref/config/quickshell/dotfiles/scripts/colors/applycolor.sh"
    run_one "$rust" "$binary"

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
    sed "s|$ref|ROOT|g" "$ref/stdout" | sort > "$ref/stdout.sorted"
    sed "s|$rust|ROOT|g" "$rust/stdout" | sort > "$rust/stdout.sorted"
    if ! diff -u "$ref/stdout.sorted" "$rust/stdout.sorted" >&2; then
        fail "$name stdout differs"
        return
    fi
    ok "$name"
}

run_case enabled "true" 1
run_case disabled "false" 1
run_case missing-config "" 1
run_case string-true "string" 1
run_case invalid-config "invalid" 1
run_case no-sequences "true" 0

echo "compare: $cases cases, $matched matched, $failed failed"
[ "$cases" -gt 0 ] && [ "$failed" -eq 0 ]
