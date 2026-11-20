#!/usr/bin/env bash
# Compares musicRecognition/recognize-music.sh with dots-recognize. Every
# command that would touch audio is a logging fake.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
reference="$repo/.config/quickshell/dotfiles/scripts/musicRecognition/recognize-music.sh"
binary="${DOTS_RECOGNIZE:-$here/../target/release/dots-recognize}"
[ -x "$binary" ] || binary="$here/../target/debug/dots-recognize"
[ -x "$binary" ] || { echo "compare: no binary at $binary" >&2; exit 2; }
[ -f "$reference" ] || { echo "compare: missing $reference" >&2; exit 2; }

work=$(mktemp -d /tmp/opencode/dots-recognize-compare.XXXXXX)
if [ "${KEEP:-0}" = 1 ]; then echo "compare: kept $work"; else trap 'rm -rf "$work"' EXIT; fi

cases=0; matched=0; failed=0
fail() { failed=$((failed + 1)); echo "compare: FAIL: $*" >&2; }
ok() { matched=$((matched + 1)); echo "compare: ok: $*"; }

seed() {
    local root="$1"
    local with_songrec="$2"
    mkdir -p "$root/home" "$root/state" "$root/cache" "$root/log" "$root/bin" "$root/nosong"
    cat > "$root/bin/pactl" <<'EOF'
#!/usr/bin/env bash
printf 'pactl %s\n' "$*" >> "$DOTS_LOG_DIR/pactl.log"
case "$1 $2" in
    "get-default-sink")
        [ "${DOTS_NO_SINK:-0}" = 1 ] && exit 1
        printf 'alsa_output.test\n' ;;
    "info")
        printf 'Server Name: test\n'
        [ "${DOTS_NO_SOURCE:-0}" = 1 ] || printf 'Default Source: alsa_input.test\n' ;;
    "list short")
        [ "${DOTS_NO_DEVICE:-0}" = 1 ] || printf '1\talsa_output.test.monitor\tmodule\tRUNNING\n2\talsa_input.test\tmodule\tSUSPENDED\n' ;;
esac
exit 0
EOF
    cp "$root/bin/pactl" "$root/nosong/pactl"
    if [ "$with_songrec" = 1 ]; then
        cat > "$root/bin/songrec" <<'EOF'
#!/usr/bin/env bash
printf 'songrec %s\n' "$*" >> "$DOTS_LOG_DIR/songrec.log"
if [ -n "${DOTS_SONG_LINE:-}" ]; then printf '%s\n' "$DOTS_SONG_LINE"; fi
[ "${DOTS_SONG_MATCH:-0}" = 1 ] && exit 0
[ "${DOTS_SONG_QUICK:-0}" = 1 ] && exit 0
exec sleep 100
EOF
        chmod +x "$root/bin/songrec"
    fi
    chmod +x "$root/bin/pactl" "$root/nosong/pactl"
    for tool in awk grep mktemp mkfifo cat sleep rm sed; do
        ln -sf "$(command -v "$tool")" "$root/nosong/$tool"
    done
}

run_one() {
    local root="$1"
    local path_dir="$2"
    shift 2
    set +e
    (
        env HOME="$root/home" \
            XDG_STATE_HOME="$root/state" \
            XDG_CACHE_HOME="$root/cache" \
            DOTS_LOG_DIR="$root/log" \
            PATH="$path_dir:/usr/local/bin:/usr/bin:/bin" \
            "$@" > "$root/stdout" 2> "$root/stderr"
    )
    echo $? > "$root/status"
    set -e
}

run_case() {
    local name="$1"
    local with_songrec="$2"
    local envs="$3"
    shift 3
    cases=$((cases + 1))
    local ref="$work/$name/reference"
    local rust="$work/$name/rust"
    seed "$ref" "$with_songrec"
    seed "$rust" "$with_songrec"

    set +e
    (
        export DOTS_NO_SINK=0 DOTS_NO_SOURCE=0 DOTS_NO_DEVICE=0
        export DOTS_SONG_LINE="" DOTS_SONG_MATCH=0 DOTS_SONG_QUICK=0
        eval "$envs"
        if [ "$with_songrec" = 1 ]; then
            run_one "$ref" "$ref/bin" bash "$reference" "$@"
            run_one "$rust" "$rust/bin" "$binary" "$@"
        else
            run_one "$ref" "$ref/nosong" bash "$reference" "$@"
            run_one "$rust" "$rust/nosong" "$binary" "$@"
        fi
    )
    set -e

    if [ "$(cat "$ref/status")" != "$(cat "$rust/status")" ]; then
        fail "$name exit $(cat "$ref/status") != $(cat "$rust/status")"
        return
    fi
    if ! diff -u "$ref/stdout" "$rust/stdout" >&2; then
        fail "$name stdout differs"
        return
    fi
    local log
    for log in songrec pactl; do
        [ -f "$ref/log/$log.log" ] || : > "$ref/log/$log.log"
        [ -f "$rust/log/$log.log" ] || : > "$rust/log/$log.log"
        if ! diff -u "$ref/log/$log.log" "$rust/log/$log.log" >&2; then
            fail "$name $log log differs"
            return
        fi
    done
    ok "$name"
}

run_case monitor-match 1 'export DOTS_SONG_LINE="{\"matches\": [{\"title\":\"x\"}]}" DOTS_SONG_MATCH=1' -i 1 -t 0.2 -s monitor
run_case input-match 1 'export DOTS_SONG_LINE="{\"matches\": [{\"title\":\"y\"}]}" DOTS_SONG_MATCH=1' -i 2 -t 1 -s input
run_case empty-matches 1 'export DOTS_SONG_LINE="{\"matches\": []}" DOTS_SONG_MATCH=1' -t 1 -s monitor
run_case no-match-quick 1 'export DOTS_SONG_QUICK=1 DOTS_SONG_LINE="{\"track\": {}}" ' -t 1 -s monitor
run_case timer-kill 1 'export DOTS_SONG_LINE="{\"track\": {}}" ' -i 1 -t 0.2 -s monitor
run_case attached-flags 1 'export DOTS_SONG_MATCH=1 DOTS_SONG_LINE="{\"matches\": []}"' -i1 -t0.2 -smonitor
run_case default-args 1 'export DOTS_SONG_QUICK=1 DOTS_SONG_LINE="{\"track\": {}}" '
run_case invalid-source 1 '' -s screen
run_case no-songrec 0 '' -s monitor
run_case unknown-flag 1 '' -x -s monitor
run_case non-option 1 'export DOTS_SONG_MATCH=1 DOTS_SONG_LINE="{\"matches\": []}"' -s monitor trailing
run_case no-default-source 1 'export DOTS_NO_SOURCE=1' -s input
run_case missing-device 1 'export DOTS_NO_DEVICE=1' -s input
run_case sink-fallback 1 'export DOTS_NO_SINK=1 DOTS_SONG_MATCH=1 DOTS_SONG_LINE="{\"matches\": []}"' -s monitor

echo "compare: $cases cases, $matched matched, $failed failed"
[ "$cases" -gt 0 ] && [ "$failed" -eq 0 ]
