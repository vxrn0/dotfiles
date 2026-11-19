# dots-colors

Phase 2a of the Rust backend: a drop-in replacement for the `--color` path of
`.config/quickshell/dotfiles/scripts/colors/generate_colors_material.py`.

It ports the reference implementation itself (`materialyoucolor` HCT/CAM16,
tonal palettes, the nine scheme variants, the dynamic colour roles, the
temperature cache and terminal harmonisation), not the Material You prose.
Arithmetic order and Python's banker's `round()` are kept so stdout matches
byte for byte.

The image path (`--path`) is phase 2b. It exits non-zero with
`--path is not implemented yet (phase 2b); only --color is supported`.

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
- `--debug`, `--cache`, and the `--path` refusal.

`terminal/catppuccin.json` does not exist in the repository, so the termscheme
axis uses `terminal/scheme-base.json`, the file `switchwall.sh` actually
passes.

## Deliberate differences

- `--path` is not implemented (phase 2b). The reference also writes `--cache`
  only on that path, so `--cache` is a no-op here exactly as it is for
  `--color` in Python.
- Malformed hex colours, missing files and invalid JSON exit 1 with a message
  instead of a Python traceback.
- A fidelity or content scheme over a colour whose temperature range is zero
  (`#000000`, `#ffffff`) makes the reference raise `ZeroDivisionError` in
  `TemperatureCache.complement()`. The binary refuses with a message and exit
  status 1; both write nothing to stdout.
- Python's `FromPaletteOptions` defaults `is_background` to the truthy `bool`
  class, so every role takes the 49/60 tone correction, not only containers.
  That quirk is reproduced.
