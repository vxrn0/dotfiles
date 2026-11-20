#!/usr/bin/env bash
# Compares keyring/unlock.sh + is_unlocked.sh with the two dots-keyring bins.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
scripts="$repo/.config/quickshell/dotfiles/scripts/keyring"
unlock_ref="$scripts/unlock.sh"
check_ref="$scripts/is_unlocked.sh"
unlock_bin="${DOTS_KEYRING_UNLOCK:-$here/../target/release/dots-keyring-unlock}"
check_bin="${DOTS_KEYRING_CHECK:-$here/../target/release/dots-keyring-is-unlocked}"
[ -x "$unlock_bin" ] || unlock_bin="$here/../target/debug/dots-keyring-unlock"
[ -x "$check_bin" ] || check_bin="$here/../target/debug/dots-keyring-is-unlocked"
[ -x "$unlock_bin" ] && [ -x "$check_bin" ] || { echo "compare: missing binaries" >&2; exit 2; }
[ -f "$unlock_ref" ] && [ -f "$check_ref" ] || { echo "compare: missing scripts" >&2; exit 2; }

work=$(mktemp -d /tmp/opencode/dots-keyring-compare.XXXXXX)
if [ "${KEEP:-0}" = 1 ]; then echo "compare: kept $work"; else trap 'rm -rf "$work"' EXIT; fi

cases=0; matched=0; failed=0
fail() { failed=$((failed + 1)); echo "compare: FAIL: $*" >&2; }
ok() { matched=$((matched + 1)); echo "compare: ok: $*"; }

make_fakes() {
    local root="$1"
    mkdir -p "$root/bin"
    cat > "$root/bin/busctl" <<'EOF'
#!/usr/bin/env bash
printf 'busctl %s\n' "$*" >> "$DOTS_LOG_DIR/busctl.log"
[ "${DOTS_BUSCTL_FAIL:-0}" = 1 ] && exit 1
[ "${DOTS_LOCKED:-1}" = 0 ] && printf 'b false\n' || printf 'b true\n'
EOF
    cat > "$root/bin/whoami" <<'EOF'
#!/usr/bin/env bash
printf 'whoami %s\n' "$*" >> "$DOTS_LOG_DIR/whoami.log"
printf 'tester\n'
EOF
    cat > "$root/bin/killall" <<'EOF'
#!/usr/bin/env bash
printf 'killall %s\n' "$*" >> "$DOTS_LOG_DIR/killall.log"
exit "${DOTS_KILLALL_EXIT:-0}"
EOF
    cat > "$root/bin/gnome-keyring-daemon" <<'EOF'
#!/usr/bin/env bash
printf 'gnome-keyring-daemon %s\n' "$*" >> "$DOTS_LOG_DIR/daemon.log"
cat > "$DOTS_LOG_DIR/daemon-stdin"
printf 'GNOME_KEYRING_CONTROL=/tmp/control\nSSH_AUTH_SOCK=/tmp/agent\n'
exit "${DOTS_DAEMON_EXIT:-0}"
EOF
    chmod +x "$root/bin/"*
}

seed() {
    local root="$1"
    local stdin="$2"
    mkdir -p "$root/home" "$root/state" "$root/cache" "$root/log" "$root/bin"
    make_fakes "$root"
    printf '%b' "$stdin" > "$root/stdin"
}

run_one() {
    local root="$1"
    shift
    set +e
    (
        env HOME="$root/home" \
            XDG_STATE_HOME="$root/state" \
            XDG_CACHE_HOME="$root/cache" \
            DOTS_LOG_DIR="$root/log" \
            PATH="$root/bin:/usr/local/bin:/usr/bin:/bin" \
            "$@" < "$root/stdin" > "$root/stdout" 2> "$root/stderr"
    )
    echo $? > "$root/status"
    set -e
}

run_case() {
    local name="$1"
    local check="$2"
    local stdin="$3"
    local envs="$4"
    local password="$5"
    cases=$((cases + 1))
    local ref="$work/$name/reference"
    local rust="$work/$name/rust"
    seed "$ref" "$stdin"
    seed "$rust" "$stdin"

    set +e
    (
        export DOTS_LOCKED=1 DOTS_BUSCTL_FAIL=0 DOTS_KILLALL_EXIT=0 DOTS_DAEMON_EXIT=0
        unset UNLOCK_PASSWORD
        eval "$envs"
        if [ -n "$password" ]; then export UNLOCK_PASSWORD="$password"; fi
        if [ "$check" = 1 ]; then
            run_one "$ref" bash "$check_ref"
            run_one "$rust" "$check_bin"
        else
            run_one "$ref" bash "$unlock_ref"
            run_one "$rust" "$unlock_bin"
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
    if ! diff -u "$ref/stderr" "$rust/stderr" >&2; then
        fail "$name stderr differs"
        return
    fi
    local log
    for log in busctl whoami killall daemon daemon-stdin; do
        [ -f "$ref/log/$log" ] || : > "$ref/log/$log"
        [ -f "$rust/log/$log" ] || : > "$rust/log/$log"
        if ! diff -u "$ref/log/$log" "$rust/log/$log" >&2; then
            fail "$name $log log differs"
            return
        fi
    done
    ok "$name"
}

run_case check-unlocked 1 "" "export DOTS_LOCKED=0" ""
run_case check-locked 1 "" "export DOTS_LOCKED=1" ""
run_case check-bus-failure 1 "" "export DOTS_BUSCTL_FAIL=1" ""
run_case unlock-already 0 "" "export DOTS_LOCKED=0" ""
run_case unlock-env 0 "" "" "secret"
run_case unlock-prompt 0 "secret\n" "" ""
run_case unlock-empty-env-prompt 0 "secret\n" "export UNLOCK_PASSWORD=" ""
run_case unlock-spaces-env 0 "" "" "   "
run_case unlock-trim-prompt 0 "  secret  \n" "" ""
run_case unlock-empty-line 0 "\n" "" ""
run_case unlock-eof 0 "" "" ""
run_case unlock-killall-fails 0 "" "export DOTS_KILLALL_EXIT=1" "secret"
run_case unlock-daemon-fails 0 "" "export DOTS_DAEMON_EXIT=1" "secret"

echo "compare: $cases cases, $matched matched, $failed failed"
[ "$cases" -gt 0 ] && [ "$failed" -eq 0 ]
