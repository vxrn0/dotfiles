#!/usr/bin/env bash
# Compares videos/record.sh with dots-record: exit status, recording tree,
# and the argument logs of every stubbed desktop command.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
reference="$repo/.config/quickshell/dotfiles/scripts/videos/record.sh"
binary="${DOTS_RECORD:-$here/../target/release/dots-record}"
[ -x "$binary" ] || binary="$here/../target/debug/dots-record"
[ -x "$binary" ] || { echo "compare: no binary at $binary" >&2; exit 2; }
[ -f "$reference" ] || { echo "compare: missing $reference" >&2; exit 2; }

work=$(mktemp -d /tmp/opencode/dots-record-compare.XXXXXX)
if [ "${KEEP:-0}" = 1 ]; then echo "compare: kept $work"; else trap 'rm -rf "$work"' EXIT; fi

cases=0
matched=0
failed=0
fail() { failed=$((failed + 1)); echo "compare: FAIL: $*" >&2; }
ok() { matched=$((matched + 1)); echo "compare: ok: $*"; }

make_fakes() {
    local root="$1"
    mkdir -p "$root/bin"
    cat > "$root/bin/pgrep" <<'EOF'
#!/usr/bin/env bash
printf 'pgrep %s\n' "$*" >> "$DOTS_LOG_DIR/pgrep.log"
[ -f "$DOTS_RECORDING_STATE" ] && exit 0
exit 1
EOF
    cat > "$root/bin/pkill" <<'EOF'
#!/usr/bin/env bash
printf 'pkill %s\n' "$*" >> "$DOTS_LOG_DIR/pkill.log"
rm -f "$DOTS_RECORDING_STATE"
exit 0
EOF
    cat > "$root/bin/notify-send" <<'EOF'
#!/usr/bin/env bash
printf 'notify-send %s\n' "$*" >> "$DOTS_LOG_DIR/notify.log"
exit 0
EOF
    cat > "$root/bin/slurp" <<'EOF'
#!/usr/bin/env bash
printf 'slurp %s\n' "$*" >> "$DOTS_LOG_DIR/slurp.log"
[ "${DOTS_SLURP_FAIL:-0}" = 1 ] && exit 1
printf '10,20 300x200\n'
EOF
    cat > "$root/bin/hyprctl" <<'EOF'
#!/usr/bin/env bash
printf 'hyprctl %s\n' "$*" >> "$DOTS_LOG_DIR/hyprctl.log"
[ "${DOTS_HYPR_FAIL:-0}" = 1 ] && exit 1
printf '%s\n' '[{"name":"HDMI-A-1","focused":false},{"name":"DP-1","focused":true}]'
EOF
    cat > "$root/bin/pactl" <<'EOF'
#!/usr/bin/env bash
printf 'pactl %s\n' "$*" >> "$DOTS_LOG_DIR/pactl.log"
if [ "$1 $2" = "get-default-sink" ]; then
    [ "${DOTS_NO_DEFAULT_SINK:-0}" = 1 ] && exit 1
    printf 'alsa_output.test\n'
    exit 0
fi
if [ "$1 $2" = "list sources" ]; then
    [ "${DOTS_NO_SOURCES:-0}" = 1 ] && exit 0
    printf '1\talsa_output.test.monitor\tmodule\tRUNNING\n2\talsa_input.test\tmodule\tSUSPENDED\n'
    exit 0
fi
exit 0
EOF
    cat > "$root/bin/wf-recorder" <<'EOF'
#!/usr/bin/env bash
printf 'wf-recorder cwd=%s args=%s\n' "$PWD" "$*" >> "$DOTS_LOG_DIR/wf-recorder.log"
printf 'ran\n' > "$PWD/wf-ran"
exit "${DOTS_WF_EXIT:-0}"
EOF
    chmod +x "$root/bin/"*
}

seed() {
    local root="$1"
    local config="$2"
    mkdir -p "$root/home/.config/dotfiles" "$root/state" "$root/cache" "$root/run" "$root/log"
    make_fakes "$root"
    if [ "$config" = custom ]; then
        printf '{"screenRecord":{"savePath":"%s"}}\n' "$root/home/recordings" > "$root/home/.config/dotfiles/config.json"
    elif [ "$config" = null ]; then
        printf '{"screenRecord":{"savePath":null}}\n' > "$root/home/.config/dotfiles/config.json"
    elif [ "$config" = invalid ]; then
        printf 'not json\n' > "$root/home/.config/dotfiles/config.json"
    fi
}

run_one() {
    local root="$1"
    shift
    set +e
    (
        env \
            HOME="$root/home" \
            XDG_CONFIG_HOME="$root/home/.config" \
            XDG_STATE_HOME="$root/state" \
            XDG_CACHE_HOME="$root/cache" \
            XDG_RUNTIME_DIR="$root/run" \
            DOTS_LOG_DIR="$root/log" \
            DOTS_RECORDING_STATE="$root/recording.state" \
            PATH="$root/bin:/usr/local/bin:/usr/bin:/bin" \
            "$@" > "$root/stdout" 2> "$root/stderr"
    )
    echo $? > "$root/status"
    set -e
}

normalize() {
    local root="$1"
    local file="$2"
    sed -e "s|$root/home|\$HOME|g" -e "s|$root|\$ROOT|g" \
        -e 's|recording_[0-9._-]*\.mp4|recording_TIMESTAMP.mp4|g' "$file"
}

run_case() {
    local name="$1"
    local config="$2"
    local running="$3"
    local envs="$4"
    shift 4
    cases=$((cases + 1))
    local ref="$work/$name/reference"
    local rust="$work/$name/rust"
    seed "$ref" "$config"
    seed "$rust" "$config"
    if [ "$running" = 1 ]; then
        : > "$ref/recording.state"
        : > "$rust/recording.state"
    fi

    set +e
    (
        export DOTS_SLURP_FAIL=0 DOTS_HYPR_FAIL=0 DOTS_NO_DEFAULT_SINK=0 DOTS_NO_SOURCES=0 DOTS_WF_EXIT=0
        eval "$envs"
        run_one "$ref" bash "$reference" "$@"
        run_one "$rust" "$binary" "$@"
    )
    set -e

    if [ "$(cat "$ref/status")" != "$(cat "$rust/status")" ]; then
        fail "$name exit $(cat "$ref/status") != $(cat "$rust/status")"
        return
    fi
    if ! diff -ruN <(cd "$ref" && find home -printf '%y %P %m\n' | sort) \
        <(cd "$rust" && find home -printf '%y %P %m\n' | sort) > "$work/tree.diff"; then
        cat "$work/tree.diff" >&2
        fail "$name tree differs"
        return
    fi
    local log
    for log in pgrep pkill notify slurp hyprctl pactl wf-recorder; do
        normalize "$ref" "$ref/log/$log.log" > "$ref/log/$log.norm" 2>/dev/null || : > "$ref/log/$log.norm"
        normalize "$rust" "$rust/log/$log.log" > "$rust/log/$log.norm" 2>/dev/null || : > "$rust/log/$log.norm"
        if ! diff -u "$ref/log/$log.norm" "$rust/log/$log.norm" >&2; then
            fail "$name $log log differs"
            return
        fi
    done
    ok "$name"
}

run_case fullscreen none 0 "" --fullscreen
run_case fullscreen-sound none 0 "" --fullscreen --sound
run_case sound-fallback none 0 "export DOTS_NO_DEFAULT_SINK=1" --fullscreen --sound
run_case sound-nothing none 0 "export DOTS_NO_DEFAULT_SINK=1 DOTS_NO_SOURCES=1" --fullscreen --sound
run_case region none 0 "" --region "10,20 300x200"
run_case slurp none 0 "" 
run_case slurp-cancel none 0 "export DOTS_SLURP_FAIL=1" 
run_case stop none 1 "" --fullscreen
run_case custom-path custom 0 "" --fullscreen
run_case missing-config none 0 "" --fullscreen
run_case null-path null 0 "" --fullscreen
run_case invalid-config invalid 0 "" --fullscreen
run_case wf-exit none 0 "export DOTS_WF_EXIT=7" --fullscreen
run_case unknown-flag none 0 "" --fullscreen --nonsense
run_case hypr-fail none 0 "export DOTS_HYPR_FAIL=1" --fullscreen

echo "compare: $cases cases, $matched matched, $failed failed"
[ "$cases" -gt 0 ] && [ "$failed" -eq 0 ]
