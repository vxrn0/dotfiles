# dots-theme

Phase 1 of the Rust backend: one binary that replaces the orchestration in
`.config/quickshell/dotfiles/scripts/colors/switchwall.sh`.

Arguments, monitor lookup, the video wallpaper flow, config/state writes, the
matugen call and the `applycolor.sh` call are Rust. The colour science is not:
`matugen` and `generate_colors_material.py` remain external calls, exactly as in
the bash script.

## Build and test

```sh
cargo build --release --offline
cargo test --offline
./tests/sandbox.sh
```

The sandbox script needs the real `matugen`, `jq`, `ffmpeg`, `flock` and the
Python venv at `~/.local/state/quickshell/.venv` (or `DOTFILES_VIRTUAL_ENV`).
It redirects `HOME` and every `XDG_*` directory into a temporary tree, puts a
fake `hyprctl`, `gsettings`, `mpvpaper`, `pkill`, `pgrep` and `notify-send` in
front of `PATH`, and never talks to the compositor.

It runs the real `switchwall.sh` and this binary over three inputs: a still
image, `--color #336699`, and a short video. It then compares the full state
tree, file modes, and the captured matugen, applycolor, gsettings and mpvpaper
argument lists. The helper paths contain the sandbox root, so both runs use the
same root path and the restore script's timestamp is normalised.

## Interface

Accepted arguments, matching what the script actually accepts:

```text
dots-theme <image>
dots-theme --image <image> [--mode dark|light] [--type <scheme>] [--noswitch]
dots-theme --color <#RRGGBB|RRGGBB|clear> [--mode dark|light] [--type <scheme>]
```

There is no `-h`/`--help`: the bash script has none, and unknown flags are
treated as a positional image path. `--type auto` and an invalid type run
`scheme_for_image.py` when an image exists and fall back to
`scheme-tonal-spot` otherwise. `--noswitch` reuses `wallpaper.path` from the
shell config.

Paths are the same as in the script:

- config: `$XDG_CONFIG_HOME/dotfiles/config.json`
- state: `$XDG_STATE_HOME/quickshell`
- cache: `$XDG_CACHE_HOME/quickshell`
- colour scripts: `$XDG_CONFIG_HOME/quickshell/dotfiles/scripts/colors`
- Python: `$DOTFILES_VIRTUAL_ENV/bin/python3`, otherwise
  `$HOME/.local/state/quickshell/.venv/bin/python3`, otherwise `python3`

The JSON writes use a unique temporary file next to `config.json`, mode 0600,
then `rename`. Missing config files remain a no-op, as in bash.
`user/generated/wallpaper/path.txt` and the colour cache are still written by
matugen and `generate_colors_material.py`; this binary only supplies the same
arguments and paths.

## Deliberate differences from the bash script

- A missing option value is an error. The bash loop re-runs the same argument
  after a failed `shift` and can spin forever.
- Failures of `hyprctl`, `matugen`, `ffmpeg`, `gsettings`, colour generation and
  `applycolor.sh` exit non-zero with a message. The bash script swallowed most
  of them with `2>/dev/null` or ignored their status.
- `--noswitch` loads `wallpaper.path` only when no explicit image was given.
  The bash assignment is order-dependent; the normal launcher and settings
  calls use `--noswitch` without an image.
- Not ported in phase 1: the interactive upscale prompt (notify-send/upscayl),
  Gemini wallpaper categorization, and the focused-monitor/cursor arithmetic.
  The cursor values were never read again, and `post_process` ignored the
  screen-size arguments it received.
- `post_process` still starts `code/material-code-set-color.sh` in the
  background, as the script does.

The binary is not wired into the desktop yet. The bash and Python scripts are
unmodified and remain the fallback until a later phase installs a wrapper.
