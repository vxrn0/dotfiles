# dots-applycolor

Phase 5c: replacement for `.config/quickshell/dotfiles/scripts/colors/applycolor.sh`.

matugen renders the terminal palette and its OSC sequences, so this binary only
pushes the pre-rendered escape file to every live terminal:

```text
dots-applycolor
```

It reads `$XDG_STATE_HOME/quickshell/user/generated/terminal/sequences.txt` and
writes it to every `/dev/pts/<digits>`. The sequences are OSC 4 (palette),
OSC 10 (foreground), OSC 11 (background), OSC 12 (cursor), OSC 17/19
(selection); foot implements all of them, so live terminals recolour without a
restart.

The config gate is `.appearance.wallpaperTheming.enableTerminal` in
`$XDG_CONFIG_HOME/dotfiles/config.json`. A missing file enables theming, exactly
like the shell; a parse error disables it and prints the error. A missing
sequences file is reported and skipped.

## Verification

`tests/compare.sh` runs both implementations inside bubblewrap with `/dev/pts`
bound to a fake directory, so no escape sequence reaches a real terminal. It
redirects every XDG path and compares the state tree, fake terminal contents,
stdout and exit status. The reference script is vendored in `tests/reference`;
it is the version this crate replaced.

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

Measured 2026-11-21: 6 cases, 6 matched, 0 failed. Cases cover enabled and
disabled theming, missing and invalid config, a string `"true"`, and a missing
sequences file.

## Deliberate differences

- `DOTS_PTS_DIR` selects the terminal directory. The default is `/dev/pts`; the
  variable exists so the comparison can be hermetic.
- The shell pushes in the background; this writes in order. The files are the
  same, only the timing changes.
- A failed write is silent, like the shell's `|| true`.
