#!/usr/bin/env bash
# Byte-for-byte comparison of dots-colors with the Python reference.
# Every run gets a throwaway HOME/XDG tree; nothing touches the live desktop.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
colors_dir="$repo/.config/quickshell/dotfiles/scripts/colors"
reference="$colors_dir/generate_colors_material.py"
scheme_reference="$colors_dir/scheme_for_image.py"

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

# scheme_for_image.py imports cv2 and numpy, which the colour venv does not
# have.  Use $python when it can import cv2, otherwise a python3 that can.
scheme_python="${DOTS_COLORS_SCHEME_PYTHON:-$python}"
scheme_env=()
if [ -n "${DOTS_COLORS_SCHEME_PYTHONPATH:-}" ]; then
    scheme_env=(env "PYTHONPATH=$DOTS_COLORS_SCHEME_PYTHONPATH")
fi
scheme_available=0
if [ -f "$scheme_reference" ] \
    && "${scheme_env[@]}" "$scheme_python" -c 'import cv2' >/dev/null 2>&1; then
    scheme_available=1
elif [ -f "$scheme_reference" ] && "${scheme_env[@]}" python3 -c 'import cv2' >/dev/null 2>&1; then
    scheme_python=python3
    scheme_available=1
fi
if [ "$scheme_available" -eq 0 ] && [ -f "$scheme_reference" ]; then
    echo "compare: scheme_for_image skipped: cv2 not importable (set DOTS_COLORS_SCHEME_PYTHON or DOTS_COLORS_SCHEME_PYTHONPATH)" >&2
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

run_scheme_case() {
    local label="$1"
    shift
    total=$((total + 1))
    "${scheme_env[@]}" "$scheme_python" "$scheme_reference" "$@" \
        >"$sandbox/ref.out" 2>"$sandbox/ref.err"
    local ref_rc=$?
    "$binary" scheme-for-image "$@" >"$sandbox/bin.out" 2>"$sandbox/bin.err"
    local bin_rc=$?

    if [ "$ref_rc" -eq "$bin_rc" ] && cmp -s "$sandbox/ref.out" "$sandbox/bin.out"; then
        matched=$((matched + 1))
        return 0
    fi

    failed=$((failed + 1))
    echo "FAIL: scheme $label (rc python=$ref_rc rust=$bin_rc)"
    printf '  args:'
    printf ' %q' "$@"
    printf '\n'
    diff -u "$sandbox/ref.out" "$sandbox/bin.out" | sed -n '1,24p'
}

run_cache_case() {
    local label="$1"
    shift
    total=$((total + 1))
    local ref_cache="$sandbox/ref-cache-$total"
    local bin_cache="$sandbox/bin-cache-$total"
    rm -f "$ref_cache" "$bin_cache"
    "$python" "$reference" "$@" --cache "$ref_cache" \
        >"$sandbox/ref.out" 2>"$sandbox/ref.err"
    local ref_rc=$?
    "$binary" "$@" --cache "$bin_cache" >"$sandbox/bin.out" 2>"$sandbox/bin.err"
    local bin_rc=$?

    if [ "$ref_rc" -eq "$bin_rc" ] && cmp -s "$sandbox/ref.out" "$sandbox/bin.out" \
        && cmp -s "$ref_cache" "$bin_cache"; then
        matched=$((matched + 1))
        return 0
    fi

    failed=$((failed + 1))
    echo "FAIL: cache $label (rc python=$ref_rc rust=$bin_rc)"
    printf '  args:'
    printf ' %q' "$@"
    printf '\n'
    diff -u "$sandbox/ref.out" "$sandbox/bin.out" | sed -n '1,24p'
    if ! cmp -s "$ref_cache" "$bin_cache"; then
        echo "  cache python: $(cat "$ref_cache" 2>/dev/null)"
        echo "  cache rust:   $(cat "$bin_cache" 2>/dev/null)"
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

# ---------------------------------------------------------------------------
# Image path.

mkdir -p "$sandbox/images"
"$python" - "$sandbox/images" <<'PY'
import sys
from PIL import Image

directory = sys.argv[1]
Image.new("L", (257, 129), 128).save(directory + "/grey.png")
Image.new("RGB", (1, 1), (10, 20, 30)).save(directory + "/one.png")
frames = [
    Image.new("RGB", (24, 16), (220, 40, 60)),
    Image.new("RGB", (24, 16), (20, 160, 90)),
]
frames[0].save(
    directory + "/anim.gif",
    save_all=True,
    append_images=frames[1:],
    duration=100,
    loop=0,
)
frames[0].save(directory + "/one.gif")
PY
printf 'not an image\n' >"$sandbox/images/not-image.txt"

image_dir="${DOTS_COLORS_IMAGE_DIR:-/home/aiden/extra/from}"
images=()
if [ -d "$image_dir" ]; then
    while IFS= read -r -d '' path; do
        images+=("$path")
    done < <(find "$image_dir" -maxdepth 3 -type f \
        \( -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.png' \) \
        -print0 | sort -z | head -z -n 12)
fi

if [ "${#images[@]}" -eq 0 ]; then
    echo "compare: no images under $image_dir; the image axis was skipped" >&2
else
    for image in "${images[@]}"; do
        label=$(basename "$image")
        run_case "$label default" --path "$image"
        run_case "$label default light" --path "$image" --mode light
        run_case "$label size 64" --path "$image" --size 64
        run_case "$label size 200 light" --path "$image" --size 200 --mode light
        run_case "$label smart debug" --path "$image" --smart --debug
        run_cache_case "$label default" --path "$image"
        if [ -n "$scheme_file" ]; then
            run_case "$label terms" --path "$image" --termscheme "$scheme_file" \
                --blend_bg_fg
            run_case "$label terms light" --path "$image" --mode light \
                --termscheme "$scheme_file" --blend_bg_fg
            run_case "$label debug terms" --path "$image" --debug \
                --termscheme "$scheme_file"
            if [ "$scheme_available" -eq 1 ]; then
                selected=$("${scheme_env[@]}" "$scheme_python" "$scheme_reference" \
                    "$image" 2>/dev/null | tr -d '\n')
                run_case "$label selected $selected" --path "$image" \
                    --scheme "$selected" --termscheme "$scheme_file" --blend_bg_fg
            fi
        fi
    done
fi

# Greyscale, 1x1, GIFs, a file that is not an image, and a missing file.
for fixture in "$sandbox/images/grey.png" "$sandbox/images/one.png" \
    "$sandbox/images/anim.gif" "$sandbox/images/one.gif"; do
    label=$(basename "$fixture")
    run_case "$label default" --path "$fixture"
    run_case "$label size 64" --path "$fixture" --size 64
    run_case "$label size 200 light" --path "$fixture" --size 200 --mode light
    run_case "$label smart debug" --path "$fixture" --smart --debug
    if [ -n "$scheme_file" ]; then
        run_case "$label terms" --path "$fixture" --termscheme "$scheme_file" \
            --blend_bg_fg
    fi
done
run_case "not an image" --path "$sandbox/images/not-image.txt"
run_case "missing image" --path "$sandbox/images/missing.png"

# ---------------------------------------------------------------------------
# scheme_for_image.py, behind `dots-colors scheme-for-image`.

if [ "$scheme_available" -eq 1 ]; then
    for image in "${images[@]}" "$sandbox/images/grey.png" "$sandbox/images/one.png" \
        "$sandbox/images/anim.gif" "$sandbox/images/one.gif"; do
        label=$(basename "$image")
        run_scheme_case "$label" "$image"
        run_scheme_case "$label colorfulness" --colorfulness "$image"
    done
    run_scheme_case "not an image" "$sandbox/images/not-image.txt"
    run_scheme_case "missing image" "$sandbox/images/missing.png"
    run_scheme_case "no arguments"
fi

echo "compare: $total cases, $matched matched, $failed failed"
if [ -z "$scheme_file" ]; then
    echo "compare: no terminal JSON found; termscheme axis was skipped"
elif [ ! -f "$colors_dir/terminal/catppuccin.json" ]; then
    echo "compare: catppuccin.json absent, used $(basename "$scheme_file") for the termscheme axis"
fi
if [ "$scheme_available" -eq 0 ]; then
    echo "compare: scheme_for_image axis was skipped; cv2 is not installed"
fi
[ "$failed" -eq 0 ]
