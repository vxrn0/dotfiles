#!/usr/bin/env bash
# Compares clipboard/prune.sh with dots-prune on identical fake cliphist DBs.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
reference="$repo/.config/quickshell/dotfiles/scripts/clipboard/prune.sh"
binary="${DOTS_PRUNE:-$here/../target/release/dots-prune}"
[ -x "$binary" ] || binary="$here/../target/debug/dots-prune"
[ -x "$binary" ] || { echo "compare: no binary at $binary" >&2; exit 2; }
[ -f "$reference" ] || { echo "compare: missing $reference" >&2; exit 2; }

work=$(mktemp -d /tmp/opencode/dots-prune-compare.XXXXXX)
if [ "${KEEP:-0}" = 1 ]; then echo "compare: kept $work"; else trap 'rm -rf "$work"' EXIT; fi

cases=0; matched=0; failed=0
fail() { failed=$((failed + 1)); echo "compare: FAIL: $*" >&2; }
ok() { matched=$((matched + 1)); echo "compare: ok: $*"; }

make_fakes() {
    local root="$1"
    mkdir -p "$root/bin"
    cat > "$root/bin/cliphist" <<'EOF'
#!/usr/bin/env bash
printf 'cliphist %s\n' "$*" >> "$DOTS_LOG_DIR/cliphist.log"
[ "${DOTS_CLIPHIST_FAIL:-0}" = 1 ] && exit 1
db="$2"
case "$3" in
    list) cat "$db" 2>/dev/null ;;
    delete)
        tmp=$(mktemp)
        cat > "$tmp"
        printf 'delete-stdin\n' >> "$DOTS_LOG_DIR/cliphist.log"
        sort "$tmp" >> "$DOTS_LOG_DIR/cliphist.log"
        if [ -s "$tmp" ]; then
            grep -vFxf "$tmp" "$db" > "$db.new" 2>/dev/null || true
            mv "$db.new" "$db"
        fi
        rm -f "$tmp"
        ;;
esac
EOF
    chmod +x "$root/bin/cliphist"
}

seed() {
    local root="$1"
    local database="$2"
    local state="$3"
    mkdir -p "$root/home" "$root/state" "$root/log" "$root/bin"
    make_fakes "$root"
    printf '%b' "$database" > "$root/db"
    mkdir -p "$root/state"
    if [ "$state" = __empty__ ]; then
        : > "$root/state/cliphist-prune.id"
    elif [ -n "$state" ]; then
        printf '%b' "$state" > "$root/state/cliphist-prune.id"
    fi
}

run_one() {
    local root="$1"
    shift
    set +e
    (
        env HOME="$root/home" \
            XDG_STATE_HOME="$root/state" \
            XDG_CACHE_HOME="$root/cache" \
            XDG_CONFIG_HOME="$root/home/.config" \
            CLIPHIST_DB="$root/db" \
            DOTS_LOG_DIR="$root/log" \
            PATH="$root/bin:/usr/local/bin:/usr/bin:/bin" \
            "$@" > "$root/stdout" 2> "$root/stderr"
    )
    echo $? > "$root/status"
    set -e
}

run_case() {
    local name="$1"
    local database="$2"
    local state="$3"
    local passes="$4"
    local envs="$5"
    cases=$((cases + 1))
    local ref="$work/$name/reference"
    local rust="$work/$name/rust"
    seed "$ref" "$database" "$state"
    seed "$rust" "$database" "$state"

    set +e
    (
        export DOTS_CLIPHIST_FAIL=0
        eval "$envs"
        local pass
        for pass in $(seq 1 "$passes"); do
            run_one "$ref" bash "$reference"
            run_one "$rust" "$binary"
        done
    )
    set -e

    if [ "$(cat "$ref/status")" != "$(cat "$rust/status")" ]; then
        fail "$name exit $(cat "$ref/status") != $(cat "$rust/status")"
        return
    fi
    if ! cmp -s "$ref/db" "$rust/db"; then
        diff -u "$ref/db" "$rust/db" >&2
        fail "$name database differs"
        return
    fi
    local refstate="$ref/state/cliphist-prune.id"
    local ruststate="$rust/state/cliphist-prune.id"
    if [ -e "$refstate" ] || [ -e "$ruststate" ]; then
        if ! diff -u "$refstate" "$ruststate" >&2; then
            fail "$name state differs"
            return
        fi
    fi
    sed -e "s|$ref/db|DB|g" -e "s|$ref|\$ROOT|g" "$ref/log/cliphist.log" 2>/dev/null | sort > "$ref/log/norm" || : > "$ref/log/norm"
    sed -e "s|$rust/db|DB|g" -e "s|$rust|\$ROOT|g" "$rust/log/cliphist.log" 2>/dev/null | sort > "$rust/log/norm" || : > "$rust/log/norm"
    if ! diff -u "$ref/log/norm" "$rust/log/norm" >&2; then
        fail "$name cliphist log differs"
        return
    fi
    ok "$name"
}

db30='30\tthree\n20\ttwo\n10\tone\n'
run_case first-run "$db30" "" 1 ""
run_case delete-all "$db30" "30\n" 1 ""
run_case delete-older "$db30" "15\n" 1 ""
run_case invalid-state "$db30" "abc\n" 1 ""
run_case zero-mark "$db30" "0\n" 1 ""
run_case empty-db "" "" 1 ""
run_case empty-state "$db30" __empty__ 1 ""
run_case no-newline "$db30" "15" 1 ""
run_case huge-mark "$db30" "999999999999999999999\n" 1 ""
run_case nonnumeric-id 'abc\tline\n12\tline2\n' "5\n" 1 ""
run_case list-failure "$db30" "" 1 "export DOTS_CLIPHIST_FAIL=1"
run_case two-runs "$db30" "" 2 ""

echo "compare: $cases cases, $matched matched, $failed failed"
[ "$cases" -gt 0 ] && [ "$failed" -eq 0 ]
