#!/usr/bin/env bash
# Runs hyprconfigurator.py and dots-hyprconfig on identical copies of the real
# Hyprland config files and synthetic fixtures, then diffs file, mode and output.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
reference="$repo/.config/quickshell/dotfiles/scripts/hyprland/hyprconfigurator.py"
binary="${DOTS_HYPRCONFIG:-$here/../target/release/dots-hyprconfig}"
[ -x "$binary" ] || binary="$here/../target/debug/dots-hyprconfig"
[ -x "$binary" ] || { echo "compare: no binary at $binary" >&2; exit 2; }
[ -f "$reference" ] || { echo "compare: missing $reference" >&2; exit 2; }

work=$(mktemp -d /tmp/opencode/dots-hyprconfig-compare.XXXXXX)
if [ "${KEEP:-0}" = 1 ]; then echo "compare: kept $work"; else trap 'rm -rf "$work"' EXIT; fi

cases=0
matched=0
failed=0

fail() {
    failed=$((failed + 1))
    echo "compare: FAIL: $*" >&2
}

ok() {
    matched=$((matched + 1))
    echo "compare: ok: $*"
}

run_one() {
    local root="$1"
    local engine="$2"
    shift 2
    set +e
    (
        env HOME="$root/home" XDG_CONFIG_HOME="$root/config" \
            XDG_STATE_HOME="$root/state" XDG_CACHE_HOME="$root/cache" \
            "$@" > "$root/stdout" 2> "$root/stderr"
    )
    echo $? > "$root/status"
    set -e
}

# seed is a path copied to config.lua, or empty for a missing file.
run_case() {
    local name="$1"
    local seed="$2"
    local resets="$3"
    shift 3
    cases=$((cases + 1))

    local ref="$work/$name/reference"
    local rust="$work/$name/rust"
    local dir
    for dir in "$ref" "$rust"; do
        mkdir -p "$dir/home" "$dir/config" "$dir/state" "$dir/cache"
        mkdir -p "$dir/sub"
        if [ -n "$seed" ]; then
            cp "$seed" "$dir/sub/config.lua"
            chmod 600 "$dir/sub/config.lua"
        fi
    done

    run_one "$ref" reference python3 "$reference" --file "$ref/sub/config.lua" "$@"
    run_one "$rust" rust "$binary" --file "$rust/sub/config.lua" "$@"

    if [ "$(cat "$ref/status")" != "$(cat "$rust/status")" ]; then
        fail "$name exit $(cat "$ref/status") != $(cat "$rust/status")"
        return
    fi
    if [ -e "$ref/sub/config.lua" ] || [ -e "$rust/sub/config.lua" ]; then
        if ! cmp -s "$ref/sub/config.lua" "$rust/sub/config.lua"; then
            diff -u "$ref/sub/config.lua" "$rust/sub/config.lua" >&2
            fail "$name file differs"
            return
        fi
        local refmode rustmode
        refmode=$(stat -c %a "$ref/sub/config.lua")
        rustmode=$(stat -c %a "$rust/sub/config.lua")
        if [ "$refmode" != "$rustmode" ]; then
            fail "$name mode $refmode != $rustmode"
            return
        fi
    fi
    if [ "$(cat "$ref/status")" = 0 ]; then
        sed "s|$ref|ROOT|g" "$ref/stdout" > "$ref/stdout.norm"
        sed "s|$rust|ROOT|g" "$rust/stdout" > "$rust/stdout.norm"
        if [ "$resets" -gt 1 ]; then
            sort "$ref/stdout.norm" > "$ref/stdout.sorted"
            sort "$rust/stdout.norm" > "$rust/stdout.sorted"
            if ! diff -u "$ref/stdout.sorted" "$rust/stdout.sorted" >&2; then
                fail "$name stdout differs (sorted; Python reset order is a set)"
                return
            fi
        elif ! diff -u "$ref/stdout.norm" "$rust/stdout.norm" >&2; then
            fail "$name stdout differs"
            return
        fi
    fi
    ok "$name"
}

fixtures="$work/fixtures"
mkdir -p "$fixtures"
cp "$repo/.config/hypr/hyprland/general.lua" "$fixtures/general.lua"
cp "$repo/.config/hypr/hyprland/colors.lua" "$fixtures/colors.lua"
cp "$repo/.config/hypr/custom/general.lua" "$fixtures/custom-general.lua"
cp "$repo/.config/hypr/hyprland/shellOverrides/main.lua" "$fixtures/overrides.lua"

one_line="$fixtures/one-line.lua"
cat > "$one_line" <<'EOF'
-- header
hl.config({a=1, b="two"})
  hl.config({ nested = { value = 4 } })
hl.config({nested={deeper={leaf=9}}})
hl.config({ flag = true, text = "keep" })
EOF

indented="$fixtures/indented.lua"
cat > "$indented" <<'EOF'
   hl.config({a=1})
	hl.config({b=2})
hl.config( {c=3})
EOF

no_newline="$fixtures/no-newline.lua"
printf 'hl.config({a=1})' > "$no_newline"

crlf="$fixtures/crlf.lua"
printf 'hl.config({a=1})\r\nhl.config({b=2})\r\n' > "$crlf"

run_case real-general "$fixtures/general.lua" 0 --set general:gaps_in 7
run_case real-colors "$fixtures/colors.lua" 0 --set general:col:active_border '"rgba(11223344)"'
run_case real-custom-reset "$fixtures/custom-general.lua" 1 --reset general:col
run_case real-overrides "$fixtures/overrides.lua" 0 --set a:b '"value"'

run_case replace-nested "$one_line" 0 --set a:b 2
run_case replace-three-levels "$one_line" 0 --set nested:deeper:leaf 10
run_case replace-indented "$indented" 0 --set a 5
run_case replace-tab "$indented" 0 --set b 6
run_case replace-no-newline "$no_newline" 0 --set a 3

run_case numbers "$one_line" 0 --set n1 42 --set n2 -3 --set n3 +4 --set n4 1.5 \
    --set n5 .5 --set n6 1e3 --set n7 NaN --set n8 inf --set n9 1_000
run_case bools "$one_line" 0 --set b1 true --set b2 false --set b3 True
run_case strings "$one_line" 0 --set s1 'hello world' --set s2 'a,b,c' --set s3 '{"a","b"}'
run_case list-value "$one_line" 0 --set list 'one,two,three'

run_case reset-existing "$one_line" 1 --reset a
run_case reset-absent "$one_line" 1 --reset missing:key
run_case set-and-reset "$one_line" 1 --set a 8 --reset a
run_case duplicate-set "$one_line" 0 --set a 1 --set a 2
run_case empty-marker "$one_line" 1 --set a '[[EMPTY]]'
run_case inline-keys "$one_line" 0 --set a 99
run_case prefix-key "$one_line" 0 --set nest 1
run_case dotted-key "$one_line" 0 --set 'x.y:z' '"dot"'
run_case spaced-nested "$indented" 0 --set nested:value 2
run_case nested-space-before-brace "$indented" 0 --set c 4
run_case crlf "$crlf" 0 --set a 7
run_case no-keys "" 0
run_case missing-file "" 0 --set appearance:palette:accentColor '"#123456"'
run_case shell-empty-reset "$fixtures/overrides.lua" 1 --reset a:b

# Invalid input only needs the same exit code; argparse text is not the contract.
cases=$((cases + 1))
ref="$work/arg-error/reference"
rust="$work/arg-error/rust"
for dir in "$ref" "$rust"; do mkdir -p "$dir"; done
run_one "$ref" reference python3 "$reference" --set only-one
run_one "$rust" rust "$binary" --set only-one
if [ "$(cat "$ref/status")" = "$(cat "$rust/status")" ] && [ "$(cat "$rust/status")" = 2 ]; then
    ok arg-error
else
    fail "arg-error exits $(cat "$ref/status") / $(cat "$rust/status")"
fi

echo "compare: $cases cases, $matched matched, $failed failed"
[ "$cases" -gt 0 ] && [ "$failed" -eq 0 ]
