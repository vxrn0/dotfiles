#!/usr/bin/env bash
# Runs the two original thumbnail scripts and dots-thumbnails on the same
# sandboxed directory, then compares the cache trees, PNG metadata and pixels.
# Nothing here touches the live desktop: HOME and every XDG directory point at
# the temporary tree, and the ImageMagick fallback uses a logging wrapper.
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../../../.." && pwd)
scripts="$repo/.config/quickshell/dotfiles/scripts/thumbnails"
python_reference="$scripts/thumbgen.py"
magick_reference="$scripts/generate-thumbnails-magick.sh"
stubs="$here/reference-stubs"

if [ -n "${DOTS_THUMBNAILS:-}" ]; then
    binary="$DOTS_THUMBNAILS"
elif [ -x "$here/../target/release/dots-thumbnails" ]; then
    binary="$here/../target/release/dots-thumbnails"
else
    binary="$here/../target/debug/dots-thumbnails"
fi
[ -x "$binary" ] || { echo "compare: no binary at $binary" >&2; exit 2; }
[ -f "$python_reference" ] || { echo "compare: missing $python_reference" >&2; exit 2; }
[ -f "$magick_reference" ] || { echo "compare: missing $magick_reference" >&2; exit 2; }
command -v magick >/dev/null || { echo "compare: magick is not installed" >&2; exit 2; }
command -v python3 >/dev/null || { echo "compare: python3 is not installed" >&2; exit 2; }
python3 -c 'from PIL import Image' 2>/dev/null || {
    echo "compare: Python Pillow is required to compare pixels" >&2
    exit 2
}

work=$(mktemp -d /tmp/opencode/dots-thumbnails-compare.XXXXXX)
if [ "${KEEP:-0}" = 1 ]; then echo "compare: kept $work"; else trap 'rm -rf "$work"' EXIT; fi
fixtures="$work/fixtures"
flat="$fixtures/flat"
nested="$flat/nested"
mkdir -p "$flat" "$nested"

magick -size 320x200 gradient:red-blue "$flat/gradient.png"
magick -size 300x180 pattern:checkerboard "$flat/checker.jpg"
magick -size 200x120 xc:none -fill '#ff000080' -draw 'rectangle 10,10 190,110' "$flat/alpha.png"
magick -size 700x400 gradient:green-yellow "$flat/large.png"
magick -size 120x80 gradient:black-white "$flat/skipped.gif"
magick -size 90x60 gradient:white-black "$nested/deep.png"
magick -size 140x90 gradient:blue-red "$flat/wéird name (1).png"
cat > "$flat/icon.svg" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="120" height="60" viewBox="0 0 120 60">
  <rect width="120" height="60" fill="#1e1e2e"/>
  <circle cx="60" cy="30" r="22" fill="#89b4fa"/>
</svg>
EOF
printf 'not an image\n' > "$flat/notes.txt"
printf 'not a video\n' > "$flat/clip.mp4"
find "$fixtures" -exec touch -d '2025-01-02 03:04:05' {} +

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

fake_bin() {
    local root="$1"
    mkdir -p "$root/bin"
    cat > "$root/bin/magick" <<EOF
#!/usr/bin/env bash
printf 'magick %s\\n' "\$*" >> "\$DOTS_LOG_DIR/magick.log"
exec $(command -v magick) "\$@"
EOF
    chmod +x "$root/bin/magick"
}

seed() {
    local root="$1"
    mkdir -p "$root/home" "$root/config" "$root/state" "$root/cache" "$root/data" "$root/run" "$root/log"
    fake_bin "$root"
}

run_engine() {
    local root="$1"
    shift
    set +e
    (
        env \
            HOME="$root/home" \
            XDG_CONFIG_HOME="$root/config" \
            XDG_STATE_HOME="$root/state" \
            XDG_CACHE_HOME="$root/cache" \
            XDG_DATA_HOME="$root/data" \
            XDG_RUNTIME_DIR="$root/run" \
            GIO_USE_VFS=local \
            DOTS_LOG_DIR="$root/log" \
            PYTHONPATH="$stubs" \
            PATH="$root/bin:/usr/local/bin:/usr/bin:/bin" \
            "$@" > "$root/stdout" 2> "$root/stderr"
    )
    echo $? > "$root/status"
    set -e
}

run_reference_gnome() {
    local root="$1"
    shift
    run_engine "$root" python3 "$python_reference" "$@"
}

run_reference_magick() {
    local root="$1"
    shift
    run_engine "$root" bash "$magick_reference" "$@"
}

compare_cache() {
    local left="$1"
    local right="$2"
    python3 - "$left" "$right" <<'PY'
import hashlib
import os
import struct
import sys
from PIL import Image, ImageChops, ImageStat

left, right = sys.argv[1], sys.argv[2]

def files(root):
    found = {}
    base = os.path.join(root, "thumbnails")
    for directory, _, names in os.walk(base):
        for name in names:
            path = os.path.join(directory, name)
            found[os.path.relpath(path, base)] = path
    return found

a, b = files(left), files(right)
if set(a) != set(b):
    print("only reference:", sorted(set(a) - set(b)))
    print("only rust:", sorted(set(b) - set(a)))
    sys.exit(1)

def texts(path):
    data = open(path, "rb").read()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise SystemExit(f"{path}: not a PNG")
    offset = 8
    found = {}
    while offset + 12 <= len(data):
        length = struct.unpack(">I", data[offset:offset + 4])[0]
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + length]
        if kind == b"tEXt":
            key, value = payload.split(b"\0", 1)
            found[key.decode()] = value.decode()
        offset += 12 + length
        if kind == b"IEND":
            break
    return found

for name in sorted(a):
    pa, pb = a[name], b[name]
    ta, tb = texts(pa), texts(pb)
    for key in ("Thumb::URI", "Thumb::MTime", "Software"):
        if ta.get(key) != tb.get(key):
            print(f"{name}: {key}: {ta.get(key)!r} != {tb.get(key)!r}")
            sys.exit(1)
    ia = Image.open(pa).convert("RGBA")
    ib = Image.open(pb).convert("RGBA")
    if ia.size != ib.size:
        print(f"{name}: size {ia.size} != {ib.size}")
        sys.exit(1)
    difference = ImageChops.difference(ia, ib)
    maximum = max(channel[1] for channel in difference.getextrema())
    mean = sum(ImageStat.Stat(difference).mean) / 4
    # Both sides are produced by the same thumbnailer; the tolerance only
    # absorbs a different encoder version. Measured: max 0, mean 0.
    if maximum > 2 or mean > 0.05:
        print(f"{name}: pixels differ, max={maximum} mean={mean:.3f}")
        sys.exit(1)
print(f"pixels and metadata match for {len(a)} files")
PY
}

compare_magick_logs() {
    local left="$1"
    local right="$2"
    local left_home="$3"
    local right_home="$4"
    local left_log="$left/log/magick.log"
    local right_log="$right/log/magick.log"
    [ -f "$left_log" ] || : > "$left_log"
    [ -f "$right_log" ] || : > "$right_log"
    sed "s|$left_home|\$HOME|g" "$left_log" | sort > "$work/left.log"
    sed "s|$right_home|\$HOME|g" "$right_log" | sort > "$work/right.log"
    diff -u "$work/left.log" "$work/right.log"
}

# One case has both engines run once or twice. The second pass is for freshness.
run_case() {
    local name="$1"
    local reference="$2"
    local runs="$3"
    local expected="$4"
    shift 4
    cases=$((cases + 1))
    local refroot="$work/$name/reference"
    local newroot="$work/$name/rust"
    seed "$refroot"
    seed "$newroot"

    local pass
    for pass in $(seq 1 "$runs"); do
        if [ "$reference" = gnome ]; then
            run_reference_gnome "$refroot" "$@"
            run_engine "$newroot" "$binary" "$@"
        else
            run_reference_magick "$refroot" "$@"
            run_engine "$newroot" "$binary" "$@"
        fi
        if [ "$(cat "$refroot/status")" != "$(cat "$newroot/status")" ]; then
            fail "$name pass $pass exit $(cat "$refroot/status") != $(cat "$newroot/status")"
            cat "$refroot/stderr" >&2
            cat "$newroot/stderr" >&2
            return
        fi
    done

    local left_cache="$refroot/cache"
    local right_cache="$newroot/cache"
    if [ "$reference" = magick ]; then
        left_cache="$refroot/home/.cache"
        right_cache="$newroot/home/.cache"
    fi
    if ! compare_cache "$left_cache" "$right_cache"; then
        fail "$name cache differs"
        return
    fi
    local count
    count=$(find "$right_cache/thumbnails" -type f -name '*.png' ! -path '*/fail/*' 2>/dev/null | wc -l)
    if [ "$count" -ne "$expected" ]; then
        fail "$name expected $expected thumbnails, found $count"
        return
    fi
    if ! diff -u "$refroot/stdout" "$newroot/stdout" > "$work/stdout.diff"; then
        cat "$work/stdout.diff" >&2
        fail "$name stdout differs"
        return
    fi
    if ! compare_magick_logs "$refroot" "$newroot" "$refroot/home" "$newroot/home"; then
        fail "$name magick command log differs"
        return
    fi
    ok "$name"
}

gnome_args=(--size normal --machine_progress -d "$flat")
run_case gnome-normal gnome 1 7 "${gnome_args[@]}"
run_case gnome-large gnome 1 7 --size large --machine_progress -d "$flat"
run_case gnome-only-images gnome 1 6 --size normal --machine_progress --only_images -d "$flat"
run_case gnome-recursive gnome 1 7 --size normal --machine_progress --only_images --recursive -d "$flat"
run_case gnome-fresh gnome 2 7 --size normal --machine_progress -d "$flat"

# A changed mtime must replace exactly that entry.
cases=$((cases + 1))
refroot="$work/gnome-mtime/reference"
newroot="$work/gnome-mtime/rust"
seed "$refroot"
seed "$newroot"
run_reference_gnome "$refroot" --size normal --machine_progress -d "$flat"
run_engine "$newroot" "$binary" --size normal --machine_progress -d "$flat"
touch -d '2025-02-03 04:05:06' "$flat/gradient.png"
run_reference_gnome "$refroot" --size normal --machine_progress -d "$flat"
run_engine "$newroot" "$binary" --size normal --machine_progress -d "$flat"
if compare_cache "$refroot/cache" "$newroot/cache"; then ok gnome-mtime; else fail gnome-mtime cache differs; fi

# A valid failed-thumbnail record for checker.jpg means both must skip it.
cases=$((cases + 1))
refroot="$work/gnome-failed/reference"
newroot="$work/gnome-failed/rust"
seed "$refroot"
seed "$newroot"
for root in "$refroot" "$newroot"; do
    python3 - "$root/cache" "$flat/checker.jpg" <<'PY'
import hashlib
import sys
from PIL import Image, PngImagePlugin
cache, source = sys.argv[1], sys.argv[2]
uri = "file://" + source
mtime = str(int(__import__("os").path.getmtime(source)))
digest = hashlib.md5(uri.encode()).hexdigest()
directory = __import__("os").path.join(cache, "thumbnails/fail/gnome-thumbnail-factory")
__import__("os").makedirs(directory, exist_ok=True)
info = PngImagePlugin.PngInfo()
info.add_text("Thumb::URI", uri)
info.add_text("Thumb::MTime", mtime)
image = Image.new("RGB", (1, 1), (0, 0, 0))
image.save(__import__("os").path.join(directory, digest + ".png"), pnginfo=info)
PY
done
run_reference_gnome "$refroot" --size normal --machine_progress -d "$flat"
run_engine "$newroot" "$binary" --size normal --machine_progress -d "$flat"
if compare_cache "$refroot/cache" "$newroot/cache"; then
    count=$(find "$newroot/cache/thumbnails/normal" -name '*.png' | wc -l)
    if [ "$count" -eq 6 ]; then ok gnome-failed; else fail "gnome-failed expected 6, found $count"; fi
else
    fail gnome-failed cache differs
fi

# Missing directory: same exit code, no cache.
cases=$((cases + 1))
refroot="$work/gnome-missing/reference"
newroot="$work/gnome-missing/rust"
seed "$refroot"
seed "$newroot"
run_reference_gnome "$refroot" --size normal --machine_progress -d "$work/absent"
run_engine "$newroot" "$binary" --size normal --machine_progress -d "$work/absent"
if [ "$(cat "$refroot/status")" = 1 ] && [ "$(cat "$newroot/status")" = 1 ]; then
    ok gnome-missing
else
    fail "gnome-missing exits $(cat "$refroot/status") / $(cat "$newroot/status")"
fi

run_case magick-file magick 1 1 --file "$flat/gradient.png"
run_case magick-directory magick 1 6 -d "$flat"
run_case magick-fresh magick 2 6 --size normal -d "$flat"
run_case magick-skip-gif magick 1 0 --file "$flat/skipped.gif"
run_case magick-missing-file magick 1 0 --file "$work/not-there.png"
run_case magick-missing-dir magick 1 0 -d "$work/not-there"

# --size after the mode is ignored by the shell loop; the binary keeps that.
run_case magick-size-after-mode magick 1 1 --file "$flat/gradient.png" --size large

echo "compare: $cases cases, $matched matched, $failed failed"
[ "$cases" -gt 0 ] && [ "$failed" -eq 0 ]
