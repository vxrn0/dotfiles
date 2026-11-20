#!/usr/bin/env bash
# Compares colors/code/material-code-set-color.sh with dots-material-code on
# copies of every supported editor settings file.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
reference="$repo/.config/quickshell/dotfiles/scripts/colors/code/material-code-set-color.sh"
binary="${DOTS_MATERIAL_CODE:-$here/../target/release/dots-material-code}"
[ -x "$binary" ] || binary="$here/../target/debug/dots-material-code"
[ -x "$binary" ] || { echo "compare: no binary at $binary" >&2; exit 2; }
[ -f "$reference" ] || { echo "compare: missing $reference" >&2; exit 2; }

work=$(mktemp -d /tmp/opencode/dots-material-code-compare.XXXXXX)
if [ "${KEEP:-0}" = 1 ]; then echo "compare: kept $work"; else trap 'rm -rf "$work"' EXIT; fi

cases=0; matched=0; failed=0
fail() { failed=$((failed + 1)); echo "compare: FAIL: $*" >&2; }
ok() { matched=$((matched + 1)); echo "compare: ok: $*"; }

write_settings() {
    local root="$1"
    local relative="$2"
    local content="$3"
    mkdir -p "$root/config/$(dirname "$relative")"
    printf '%b' "$content" > "$root/config/$relative"
    chmod 640 "$root/config/$relative"
}

prepare_simple() {
    write_settings "$1" "Code/User/settings.json" '{\n  "editor.fontSize": 14\n}\n'
}
prepare_present() {
    write_settings "$1" "Code/User/settings.json" '{\n  "material-code.primaryColor": "#111111",\n  "editor.fontSize": 14\n}\n'
}
prepare_spaces() {
    write_settings "$1" "Code/User/settings.json" '{\n  "material-code.primaryColor"  :   "#111111" ,\n}\n'
}
prepare_tabs() {
    write_settings "$1" "Code/User/settings.json" '{\n\t"material-code.primaryColor"\t:\t"#111111"\t\n}\n'
}
prepare_duplicate() {
    write_settings "$1" "Code/User/settings.json" '{\n  "material-code.primaryColor": "#111111", "material-code.primaryColor": "#222222"\n}\n'
}
prepare_two_lines() {
    write_settings "$1" "Code/User/settings.json" '{\n  "material-code.primaryColor": "#111111",\n  "material-code.primaryColor": "#222222"\n}\n'
}
prepare_no_final_newline() {
    write_settings "$1" "Code/User/settings.json" '{\n  "material-code.primaryColor": "#111111"\n}'
}
prepare_no_brace() {
    write_settings "$1" "Code/User/settings.json" '{\n  "material-code.primaryColor": "#111111"\n'
}
prepare_closing_comma() {
    write_settings "$1" "Code/User/settings.json" '{\n  "editor.fontSize": 14\n},\n'
}
prepare_empty() {
    write_settings "$1" "Code/User/settings.json" ''
}
prepare_escaped_quote() {
    write_settings "$1" "Code/User/settings.json" '{\n  "material-code.primaryColor": "#11\\"11"\n}\n'
}
prepare_all_forks() {
    local root="$1"
    for relative in \
        "Code/User/settings.json" \
        "VSCodium/User/settings.json" \
        "Code - OSS/User/settings.json" \
        "Code - Insiders/User/settings.json" \
        "Cursor/User/settings.json" \
        "Antigravity/User/settings.json" \
        "Windsurf/User/settings.json"; do
        write_settings "$root" "$relative" '{\n  "editor.fontSize": 14\n}\n'
    done
}
prepare_symlink() {
    local root="$1"
    mkdir -p "$root/config/Code/User"
    printf '{\n  "material-code.primaryColor": "#111111"\n}\n' > "$root/config/Code/User/target.json"
    ln -s target.json "$root/config/Code/User/settings.json"
}
prepare_mixed() {
    local root="$1"
    write_settings "$root" "Code/User/settings.json" '{\n  "material-code.primaryColor": "#111111"\n}\n'
    write_settings "$root" "Cursor/User/settings.json" '{\n  "editor.fontSize": 14\n}\n'
    write_settings "$root" "Windsurf/User/settings.json" '{\n  "material-code.primaryColor": "#222222"\n}\n'
}

seed() {
    local root="$1"
    local prepare="$2"
    local color="$3"
    mkdir -p "$root/home" "$root/state/quickshell/user/generated" "$root/config"
    if [ "$color" != __missing__ ]; then
        printf '%b' "$color" > "$root/state/quickshell/user/generated/color.txt"
    fi
    "$prepare" "$root"
}

run_one() {
    local root="$1"
    shift
    set +e
    (
        env HOME="$root/home" \
            XDG_CONFIG_HOME="$root/config" \
            XDG_STATE_HOME="$root/state" \
            XDG_CACHE_HOME="$root/cache" \
            PATH="/usr/local/bin:/usr/bin:/bin" \
            "$@" > "$root/stdout" 2> "$root/stderr"
    )
    echo $? > "$root/status"
    set -e
}

run_case() {
    local name="$1"
    local prepare="$2"
    local color="$3"
    cases=$((cases + 1))
    local ref="$work/$name/reference"
    local rust="$work/$name/rust"
    seed "$ref" "$prepare" "$color"
    seed "$rust" "$prepare" "$color"

    run_one "$ref" bash "$reference"
    run_one "$rust" "$binary"

    if [ "$(cat "$ref/status")" != "$(cat "$rust/status")" ]; then
        fail "$name exit $(cat "$ref/status") != $(cat "$rust/status")"
        return
    fi
    if ! diff -ruN "$ref/config" "$rust/config" > "$work/config.diff"; then
        cat "$work/config.diff" >&2
        fail "$name settings differ"
        return
    fi
    diff -ruN <(cd "$ref" && find config -printf '%y %P %m\n' | sort) \
        <(cd "$rust" && find config -printf '%y %P %m\n' | sort) > "$work/modes.diff" || {
        cat "$work/modes.diff" >&2
        fail "$name settings modes differ"
        return
    }
    if ! diff -u "$ref/stdout" "$rust/stdout" >&2; then
        fail "$name stdout differs"
        return
    fi
    ok "$name"
}

run_case append 'prepare_simple' '#abcdef\n'
run_case replace 'prepare_present' '#abcdef\n'
run_case replace-spaces 'prepare_spaces' '#abcdef\n'
run_case replace-tabs 'prepare_tabs' '#abcdef\n'
run_case replace-duplicate 'prepare_duplicate' '#abcdef\n'
run_case replace-two-lines 'prepare_two_lines' '#abcdef\n'
run_case replace-no-final-newline 'prepare_no_final_newline' '#abcdef\n'
run_case replace-no-brace 'prepare_no_brace' '#abcdef\n'
run_case append-closing-comma 'prepare_closing_comma' '#abcdef\n'
run_case empty-file 'prepare_empty' '#abcdef\n'
run_case escaped-quote 'prepare_escaped_quote' '#abcdef\n'
run_case all-forks 'prepare_all_forks' '#abcdef\n'
run_case mixed 'prepare_mixed' '#abcdef\n'
run_case symlink 'prepare_symlink' '#abcdef\n'
run_case missing-color 'prepare_simple' __missing__
run_case empty-color 'prepare_present' ''

echo "compare: $cases cases, $matched matched, $failed failed"
[ "$cases" -gt 0 ] && [ "$failed" -eq 0 ]
