# dots-regions

Phase 3 of the Rust backend: a working replacement for
`.config/quickshell/dotfiles/scripts/images/find_regions.py`, the "suggest a
region" feature in the screenshot selector.

The Python script needs `cv2.ximgproc.segmentation.createSelectiveSearchSegmentation()`,
and OpenCV is not installed in the venv, so it has not run for a long time. There
is no reference to diff against; this crate builds the feature instead of porting
it. The output shape and the flag behaviour are the script's.

## Approach

The image is converted to luma at native resolution up to 1920 pixels on the
long side (integer downscale above that), turned into a Sobel edge map with a
threshold of mean + 1.2 standard deviations clamped to 40..96, dilated by one
pixel, and split into 8-connected components; each component's bounding box
becomes a candidate, with the dilation undone and the coordinates scaled back.
Candidates then pass the Python size rules and IoU-0.7 non-maximum suppression.

Why this and not the alternatives:

- Selective search itself needs OpenCV. Reimplementing its graph segmentation
  and hierarchical merging would be a much larger project than the feature is
  worth, and would still not match a reference that cannot run.
- Hough lines only fit straight segments. A code block or a photo's texture is
  not made of lines, and every text stroke becomes a spurious line.
- A downscaled grid energy map was rejected because a coarse cell blurs
  one-pixel window and panel borders, which are exactly what the feature should
  find, and it needs a second pass to turn energy blobs into rectangles.
- Edge components keep the exact border coordinates and need no second pass.
  One dilation closes the gaps that JPEG and anti-aliasing leave in an outline.

## Build and test

```sh
cargo build --release --offline
cargo test --offline
cargo test --release --offline --test regions_cli -- --ignored --nocapture
```

The last command is the timing gate: it generates a 1920x1080 PNG in
`target/tmp` and requires the binary to finish under 300 ms.

Synthetic fixtures are generated inside the test, not committed: one rectangle,
two rectangles, a textured interior, a gradient with a hard-edged panel, and a
full-black and full-white image. Each expected rectangle must reach IoU >= 0.7;
both blank images must produce `[]`.

The real-image test uses `$DOTS_REGIONS_IMAGES`, default
`/home/aiden/extra/from`, and three files there. It asserts valid JSON, boxes
inside the image, widths below `--max-width`, heights below `--max-height`, no
exact duplicates and fewer than 40 results, and writes a debug overlay to
`target/tmp/regions/meta-debug.png`. If the directory is absent the test prints
that it is skipping and returns; on this machine it runs.

## Interface

```text
dots-regions [--image PATH] [--min-width N] [--min-height N]
             [--max-width N] [--max-height N] [--hyprctl] [--single]
             [--debug-output PATH]
```

Defaults match the Python: image `/tmp/quickshell/media/screenshot/image`, minimum
width 200, minimum height 100, no maximum. `-i` and `-do` are accepted as short
forms. The minimum and maximum comparisons are strict (`width > min`,
`width < max`), and a candidate covering the whole image at (0, 0) is dropped,
exactly as in the script.

stdout is one JSON array:

```json
[{"x": 100, "y": 90, "width": 300, "height": 200}]
```

With `--hyprctl` each entry becomes `{"at": [x, y], "size": [width, height]}`,
the shape the caller parses. `--single` keeps only the largest region.
`--debug-output` draws two-pixel red boxes on the original and writes it to that
path, choosing the format from the extension. As in the Python, a failed debug
write does not change the exit status; it prints a warning to stderr. stdout is
unaffected.

Exit status: 0 on success, 1 when the image is missing or cannot be decoded
(`Error: Could not load image <path>` on stderr), 2 for an unknown flag or a
missing/non-integer value, as argparse does.

## Deliberate differences and limits

- `--quality`, `--k`, `--min-size`, `--sigma` and `--resize-factor` are not
  accepted. They select and tune selective search, which this is not; the QML
  caller passes none of them, and silently ignoring them would be worse than
  exit 2.
- Decoding is PNG and JPEG only. `cv2.imread` also reads BMP, WebP and TIFF;
  those now exit 1. Screenshots from the shell are PNG.
- Only luma is used. An edge made of two colours with the same luma is
  invisible to the detector.
- There is one scale, no hierarchical grouping. An object assembled from many
  small, widely spaced parts can appear as its parts or as one enclosing box;
  selective search would propose both.
- A texture-free fill against a background with almost the same luma has no
  edge and is not proposed. Nothing in the image marks it as content.
- Not ported: nothing that affects the caller. There is no async, no GPU and no
  GUI.

## Verification status

- Synthetic fixtures: 5 expected rectangles matched at IoU >= 0.7; both blank
  images empty.
- Real fixtures: 3 images, 5 / 1 / 1 regions at `--max-width 960
  --max-height 540`; all bounds, maximums, duplicates and the < 40 limit checked.
- Flag contract: min/max filtering on a real detection, `--hyprctl` shape,
  `--single`, `--debug-output` overlay, missing and unreadable image, unknown
  flag.
- Runtime, release build on this machine: 33 ms for a 1920x1080 PNG, 54 ms for
  the 1920x1080 JPEG fixture. The requirement is under 300 ms.

Unverified: behaviour on an actual live screenshot from this desktop, and on
compositor scales other than 1. Every fixture is a still image read from disk.
