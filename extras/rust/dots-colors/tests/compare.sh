#!/usr/bin/env bash
# Byte-for-byte comparison of dots-colors (--color path) with the Python reference.
# Every run gets a throwaway HOME/XDG tree; nothing touches the live desktop.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
colors_dir="$repo/.config/quickshell/dotfiles/scripts/colors"
reference="$colors_dir/generate_colors_material.py"

if [ -n "${DOTFILES_VIRTUAL_ENV:-}" ]; then
    python="$DOTFILES_VIRTUAL_ENV/bin/python3"
elif [ -x "$HOME/.local/state/quickshell/.venv/bin/python3" ]; then
    python="$HOME/.local/state/quickshell/.venv/bin/python3"
else
    python=python3
fi

if [ -n "${DOTS_COLORS:-}" ]; then
    binary="$DOTS_COLORS"
elif [ -x "$here/../target/release/dots-colors" ]; then
    binary="$here/../target/release/dots-colors"
else
    binary="$here/../target/debug/dots-colors"
fi

if [ ! -x "$binary" ]; then
    echo "compare: no binary at $binary; run: cargo build --release --offline" >&2
    exit 2
fi
if [ ! -f "$reference" ]; then
    echo "compare: reference not found: $reference" >&2
    exit 2
fi

scheme_file=""
for candidate in "$colors_dir/terminal/catppuccin.json" "$colors_dir/terminal/scheme-base.json"; do
    if [ -f "$candidate" ]; then
        scheme_file="$candidate"
        break
    fi
done

sandbox=$(mktemp -d)
trap 'rm -rf "$sandbox"' EXIT
mkdir -p "$sandbox/home" "$sandbox/config" "$sandbox/state" "$sandbox/cache" "$sandbox/data" "$sandbox/run"
export HOME="$sandbox/home"
export XDG_CONFIG_HOME="$sandbox/config"
export XDG_STATE_HOME="$sandbox/state"
export XDG_CACHE_HOME="$sandbox/cache"
export XDG_DATA_HOME="$sandbox/data"
export XDG_RUNTIME_DIR="$sandbox/run"

colors=(
    '#000000' '#ffffff' '#777777' '#808080'
    '#ff0000' '#00ff00' '#0000ff' '#ffff00' '#00ffff' '#ff00ff'
    '#afc6ff' '#44464f'
)
schemes=(
    scheme-fruit-salad scheme-expressive scheme-monochrome scheme-rainbow
    scheme-tonal-spot scheme-neutral scheme-fidelity scheme-content scheme-vibrant
)

total=0
matched=0
failed=0

run_case() {
    local label="$1"
    shift
    total=$((total + 1))
    "$python" "$reference" "$@" >"$sandbox/ref.out" 2>"$sandbox/ref.err"
    local ref_rc=$?
    "$binary" "$@" >"$sandbox/bin.out" 2>"$sandbox/bin.err"
    local bin_rc=$?

    if [ "$ref_rc" -eq "$bin_rc" ] && cmp -s "$sandbox/ref.out" "$sandbox/bin.out"; then
        matched=$((matched + 1))
        return 0
    fi

    failed=$((failed + 1))
    echo "FAIL: $label (rc python=$ref_rc rust=$bin_rc)"
    printf '  args:'
    printf ' %q' "$@"
    printf '\n'
    diff -u "$sandbox/ref.out" "$sandbox/bin.out" | sed -n '1,24p'
    if [ "$ref_rc" -ne "$bin_rc" ]; then
        echo "  python stderr: $(tail -n 1 "$sandbox/ref.err")"
        echo "  rust stderr:   $(tail -n 1 "$sandbox/bin.err")"
    fi
}

for color in "${colors[@]}"; do
    for mode in dark light; do
        for scheme in "${schemes[@]}"; do
            run_case "$color $mode $scheme" --color "$color" --mode "$mode" --scheme "$scheme"
            if [ -n "$scheme_file" ]; then
                run_case "$color $mode $scheme terms" --color "$color" --mode "$mode" \
                    --scheme "$scheme" --termscheme "$scheme_file"
            fi
        done
    done
done

# The production call adds --blend_bg_fg; cover it on the schemes that use
# every branch of the terminal harmonisation.
if [ -n "$scheme_file" ]; then
    for color in "${colors[@]}"; do
        for mode in dark light; do
            for scheme in scheme-tonal-spot scheme-fidelity scheme-monochrome; do
                run_case "$color $mode $scheme blend" --color "$color" --mode "$mode" \
                    --scheme "$scheme" --termscheme "$scheme_file" --blend_bg_fg
            done
        done
    done
fi

# --debug prints float formatting, ANSI swatches and the harmonise section.
for color in "${colors[@]}"; do
    for mode in dark light; do
        for scheme in scheme-tonal-spot scheme-expressive; do
            run_case "$color $mode $scheme debug" --color "$color" --mode "$mode" \
                --scheme "$scheme" --debug
            if [ -n "$scheme_file" ]; then
                run_case "$color $mode $scheme debug terms" --color "$color" --mode "$mode" \
                    --scheme "$scheme" --termscheme "$scheme_file" --debug
            fi
        done
    done
done

# Non-default tuning, transparency and --smart are functional on --color.
if [ -n "$scheme_file" ]; then
    for color in '#336699' '#ff0000' '#777777' '#000000'; do
        run_case "$color tuning dark" --color "$color" --mode dark --scheme scheme-tonal-spot \
            --termscheme "$scheme_file" --harmony 0.3 --harmonize_threshold 45 \
            --term_fg_boost 0.6 --transparency transparent --smart
        run_case "$color tuning light" --color "$color" --mode light --scheme scheme-tonal-spot \
            --termscheme "$scheme_file" --harmony 0.3 --harmonize_threshold 45 \
            --term_fg_boost 0.6 --transparency transparent --smart
    done
fi

# --cache is a no-op on the --color path: stdout unchanged, no file written.
for color in '#000000' '#336699' '#ffffff'; do
    ref_cache="$sandbox/ref-cache"
    bin_cache="$sandbox/bin-cache"
    rm -f "$ref_cache" "$bin_cache"
    total=$((total + 1))
    "$python" "$reference" --color "$color" --cache "$ref_cache" >"$sandbox/ref.out" 2>"$sandbox/ref.err"
    ref_rc=$?
    "$binary" --color "$color" --cache "$bin_cache" >"$sandbox/bin.out" 2>"$sandbox/bin.err"
    bin_rc=$?
    if [ "$ref_rc" -eq "$bin_rc" ] && cmp -s "$sandbox/ref.out" "$sandbox/bin.out" \
        && [ ! -e "$ref_cache" ] && [ ! -e "$bin_cache" ]; then
        matched=$((matched + 1))
    else
        failed=$((failed + 1))
        echo "FAIL: cache $color (rc python=$ref_rc rust=$bin_rc, file_existed=$([ -e "$ref_cache" ] || [ -e "$bin_cache" ] && echo yes || echo no))"
        diff -u "$sandbox/ref.out" "$sandbox/bin.out" | sed -n '1,24p'
    fi
done

# The image path is phase 2b: the binary must refuse it loudly, with empty stdout.
printf 'P6\n1 1\n255\n\x00\x00\x00' >"$sandbox/one.ppm"
total=$((total + 1))
"$binary" --path "$sandbox/one.ppm" >"$sandbox/bin.out" 2>"$sandbox/bin.err"
bin_rc=$?
if [ "$bin_rc" -ne 0 ] && [ ! -s "$sandbox/bin.out" ] \
    && grep -q "not implemented yet" "$sandbox/bin.err"; then
    matched=$((matched + 1))
else
    failed=$((failed + 1))
    echo "FAIL: --path must exit non-zero with a not-implemented message (rc=$bin_rc)"
fi

echo "compare: $total cases, $matched matched, $failed failed"
if [ -z "$scheme_file" ]; then
    echo "compare: no terminal JSON found; termscheme axis was skipped"
elif [ ! -f "$colors_dir/terminal/catppuccin.json" ]; then
    echo "compare: catppuccin.json absent, used $(basename "$scheme_file") for the termscheme axis"
fi
[ "$failed" -eq 0 ]
