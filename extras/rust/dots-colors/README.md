# dots-colors

Phase 2 of the Rust backend: a drop-in replacement for both colour scripts in
`.config/quickshell/dotfiles/scripts/colors`:

- `generate_colors_material.py`, both `--color` and `--path`;
- `scheme_for_image.py`, behind the `scheme-for-image` subcommand.

It ports the reference implementation itself (`materialyoucolor` HCT/CAM16,
tonal palettes, the nine scheme variants, the dynamic colour roles, the
temperature cache and terminal harmonisation), not the Material You prose.
Arithmetic order and Python's banker's `round()` are kept so stdout matches
byte for byte.

`scheme-for-image` is a subcommand rather than a second binary because it shares
the image decoder and the INTER_AREA resize with the colour path, and because
`scheme-for-image` is not a valid `generate_colors_material.py` flag: there is no
ambiguity with the reference CLI.

## Build and test

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

`tests/compare.sh` finds the Python reference in the repository, redirects
`HOME` and every `XDG_*` directory to a temporary tree, and runs both
implementations over:

- 12 colours: black, white, two greys, the six saturated primaries, and
  `#afc6ff` / `#44464f` from `.config/hypr/hyprland/colors.lua`;
- both modes and all nine explicit `--scheme` names (plus the default
  `vibrant`, which falls through to tonal spot);
- with and without `--termscheme`, including `--blend_bg_fg`, non-default
  harmony/threshold/boost, `--transparency` and `--smart`;
- `--debug`, `--cache`, and the `--color` path's no-op cache;
- every JPEG and PNG under `$DOTS_COLORS_IMAGE_DIR` (default
  `/home/aiden/extra/from`), at the default size and at `--size 64` /
  `--size 200`, in both modes, with `--smart`, with `--debug` (including the
  image properties block), with `--termscheme`, with the scheme
  `scheme_for_image.py` picks, and with `--cache`;
- a greyscale PNG, a 1x1 PNG, a file that is not an image, and a missing file;
- `scheme_for_image.py` on the same images, with and without
  `--colorfulness`, plus its error paths.

`terminal/catppuccin.json` does not exist in the repository, so the termscheme
axis uses `terminal/scheme-base.json`, the file `switchwall.sh` actually
passes.

`scheme_for_image.py` needs `cv2` and `numpy`, which the colour venv does not
provide. The script looks for a Python that can import `cv2`; if there is none,
that axis is skipped and the summary says so. To point it at one:

```sh
DOTS_COLORS_SCHEME_PYTHON=python3 \
DOTS_COLORS_SCHEME_PYTHONPATH=/path/to/opencv-python-headless \
    ./tests/compare.sh
```

## Image path

- `calculate_optimal_size` keeps the reference's `--size` square-area rule.
- The bicubic resize is a line-for-line port of Pillow 13's `Resample.c`,
  including its 22-bit fixed-point coefficients and the pass order. RGBA
  images take Pillow's premultiplied `RGBa` path.
- Quantisation is the C++ backend inside `materialyoucolor` 2.0.9: Wu's
  histogram quantiser, weighted squared-error means refinement, and the
  `Score` filter. It reproduces the reference's integer `abs()` on the
  cluster-movement delta, which truncates the distance before the `> 3` test.
- JPEG decoding uses the system `libjpeg-turbo` (`libturbojpeg.so.0`). Its
  IDCT and chroma upsampling are byte-identical to the libjpeg build inside
  Pillow; `zune-jpeg` is not (up to eight levels off per channel, which changes
  the selected seed). The `libjpeg-turbo` package is therefore a runtime
  dependency.
- PNG decoding goes through the `png` crate. Lossless pixels are identical to
  Pillow's.
- GIF follows Pillow's `seek(1)`: the colour path uses the second frame and a
  single-frame GIF fails like the reference. `scheme-for-image` uses the first
  frame, like `cv2.imread`.
- `scheme_for_image.py` uses OpenCV's `INTER_AREA` resize, ported exactly, and
  NumPy's pairwise summation order for the colourfulness mean and standard
  deviation.

## Deliberate differences and limits

- `--color` and malformed input: missing files, invalid JSON and bad hex
  colours exit 1 with a message instead of a Python traceback.
- A fidelity or content scheme over a colour whose temperature range is zero
  (`#000000`, `#ffffff`) makes the reference raise `ZeroDivisionError` in
  `TemperatureCache.complement()`. The binary refuses with a message and exit
  status 1; both write nothing to stdout.
- Python's `FromPaletteOptions` defaults `is_background` to the truthy `bool`
  class, so every role takes the 49/60 tone correction, not only containers.
  That quirk is reproduced.
- Not supported: CMYK JPEG and greyscale PNG with alpha. The reference decodes
  them; this binary exits 1 with a message and empty stdout. 16-bit PNG also
  exits 1, but so does the reference: its pybind layer rejects the pixel list.
- The reference's greyscale-with-alpha result depends on an out-of-bounds read
  (`pixels[i][2]` on a two-element vector), so it is not reproducible.
  `scheme-for-image` likewise refuses these three formats where `cv2.imread`
  would read them.
