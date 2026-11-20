# dots-hyprconfig

Phase 5b: a drop-in replacement for
`.config/quickshell/dotfiles/scripts/hyprland/hyprconfigurator.py`.

The shell writes single-line `hl.config({...})` statements into
`~/.config/hypr/hyprland/shellOverrides/main.lua`. This tool rewrites those
lines in place, or appends one when the key is absent.

## Interface

```text
dots-hyprconfig [--file PATH] [--set KEY VALUE] [--reset KEY] ...
```

Defaults: file `~/.config/hypr/hyprland.conf`; `~` is expanded through `HOME`.
`--set` and `--reset` repeat. A value of `[[EMPTY]]` turns that set into a
reset, exactly like the Python. No keys prints the same error and exits 0.
Argument errors exit 2, as argparse does.

Values are formatted the Python way: `true` and `false` stay bare, anything
`float()` accepts stays bare (`42`, `-3`, `.5`, `1e3`, `NaN`, `inf`, `1_000`),
everything else is wrapped in double quotes. Nested keys are colon-separated
and become `a={b={c=value}}`.

## Matching

The Python builds one regex per key and replaces an entire matching line. The
translation keeps that shape, including its quirks:

- only a line whose first non-space text is `hl.config({` can match;
- nested parts must be on the same line, with a literal `{` immediately after
  each `=`;
- a line starts matching at that prefix, so an inline second key is lost when
  the first key matches;
- `a` and `ab` are distinct, and the match is case-sensitive;
- reset wins over set on the same line; a set key whose line was removed is
  appended again at the end;
- duplicate `--set` keeps the last value at the first key position;
- an absent key is appended, adding a missing final newline to the last line.

Reading and writing follow Python text mode: `\r\n` and `\r` become `\n`, CRLF
files are rewritten with LF. Existing mode bits are preserved; a new file or
directory uses `0644`/the process umask. The replacement is a temporary file in
the same directory followed by `rename`. A failed save prints
`Error saving file: ...` and still exits 0, as the Python does.

## Verification

`tests/compare.sh` copies the real `hyprland/general.lua`, `colors.lua`,
`custom/general.lua` and `shellOverrides/main.lua` into a temporary tree, adds
the synthetic cases, redirects all XDG paths and compares file bytes, mode,
stdout and exit status against the Python reference:

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

Measured 2025-11-21: 28 cases, 28 matched, 0 failed. Cases cover every value
family (string, number, bool, comma/list-like string), two- and three-level
nesting, indentation, tabs, CRLF, no final newline, absent keys, absent files,
duplicate sets, reset/set collisions, `[[EMPTY]]`, inline keys, prefix keys and
argparse errors.

The only output-order difference is deliberate: Python iterates `set` for the
`Removed ...` lines, so more than one reset prints in hash order; this binary
prints them in argument order. The harness sorts those lines. The QML caller
sends one key per process and never sees the difference.

## Deliberate differences

- Invalid UTF-8 in the config is read lossily; Python raises `UnicodeDecodeError`.
  Hyprland configs are UTF-8.
- Save errors print Rust's `io::Error` text instead of Python's `[Errno ...]`
  text; both go to stdout and exit 0.
- No caller is switched over; this file is the replacement to point
  `HyprlandConfig.qml` at.
