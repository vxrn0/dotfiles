# dots-applycolor

Phase 5c: replacement for `.config/quickshell/dotfiles/scripts/colors/applycolor.sh`.

It reads `$XDG_STATE_HOME/quickshell/user/generated/material_colors.scss`,
fills `terminal/kitty-theme.conf` and `terminal/sequences.txt`, and pushes the
kitty theme to every live `$XDG_RUNTIME_DIR/kitty.sock*`:

```text
dots-applycolor
```

Templates come from `$XDG_CONFIG_HOME/quickshell/dotfiles/scripts/colors`, the
same installed location the shell calls. The script's `SCRIPT_DIR` is not used
because the binary lives in `~/.local/bin`; this is the one path-resolution
change.

The SCSS cuts follow the shell pipeline: name is everything before the first
`:`, the value is the second space-separated field after that colon (or the
whole field when there is no space), then everything before the first `;`. One
leading `#` is removed before substituting into `name #`; `$alpha` becomes
`100`. Every occurrence on a line is replaced.

`kitten @ --to unix:<socket> set-colors --all --configured <file>` is tried on
every socket. If none succeeds, `SIGUSR1` goes to every pid from `pidof kitty`,
the script's fallback for a kitty without remote control. The terminal escape
file is written to `/dev/pts/<digits>` only, as before.

The config gate is `.appearance.wallpaperTheming.enableTerminal` in
`$XDG_CONFIG_HOME/dotfiles/config.json`. A missing file enables theming, exactly
like the shell; a parse error disables it and prints the error.

## Verification

`tests/compare.sh` runs both implementations inside bubblewrap with `/dev/pts`
bound to a fake directory, so no escape sequence reaches a real terminal. It
stubs `pgrep`, `kitten` and `pidof`, redirects every XDG path, and compares the
state tree, fake terminal contents, stdout, exit status, command logs and the
`SIGUSR1` outcome:

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

Measured 2025-11-21: 11 cases, 11 matched, 0 failed. Cases cover kitty running
and stopped, two sockets plus a non-socket, disabled theming, missing and
invalid config, a string `"true"`, missing kitty or both templates, successful
and failing `kitten`, the `pidof` signal fallback, and a missing SCSS.

## Deliberate differences

- `kitten` stderr is no longer redirected to `/dev/null`. A failing push is
  visible; the shell hid it and only signalled kitty afterwards.
- The shell ran `apply_kitty &` and `apply_anyterm &`; this runs them in order.
  The files and logs are the same, only the interleaving changes.
- `DOTS_PTS_DIR` selects the terminal directory. The default is `/dev/pts`; the
  variable exists so the comparison can be hermetic.
- sed replacement text is literal. A value containing `&` or a backslash would
  follow sed's special replacement rules in the shell and is inserted as-is
  here. Generated values are hex colours.
- A failing `pidof` and a failed `kill` now print an error. The shell had
  `kill ... 2>/dev/null`; the empty-pid case stays quiet because there is
  nothing to signal.
