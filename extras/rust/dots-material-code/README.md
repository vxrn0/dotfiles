# dots-material-code

Phase 5c: replacement for
`.config/quickshell/dotfiles/scripts/colors/code/material-code-set-color.sh`.

Reads `$XDG_STATE_HOME/quickshell/user/generated/color.txt` (falling back to
`$HOME/.local/state`) and updates
`"material-code.primaryColor"` in all supported settings files:

- `Code/User/settings.json`
- `VSCodium/User/settings.json`
- `Code - OSS/User/settings.json`
- `Code - Insiders/User/settings.json`
- `Cursor/User/settings.json`
- `Antigravity/User/settings.json`
- `Windsurf/User/settings.json`

all under `$XDG_CONFIG_HOME` (default `$HOME/.config`). Missing files are
ignored.

If the key is present, the first quoted value on each matching line is
replaced. The key may be followed by spaces, tabs, a colon and spaces before the
opening quote. If the key is absent from the file, the last line is rewritten
with the same sed expression as the shell, which inserts
`,`, the new key and a closing `}`. That expression produces a lone comma line
after the previous line (for the usual `}` last line); the bug is preserved
because the output file is the contract.

The shell's second sed (`s/,\n,/,/`) is dead: it runs as a separate
invocation after the first, when the last line is already `}`, so it can never
see a newline in its pattern space. It is not reproduced.

## Verification

`tests/compare.sh` copies all seven settings paths into a temporary tree,
redirects all XDG paths, and compares bytes and modes after both scripts ran:

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

Measured 2025-11-21: 16 cases, 16 matched, 0 failed. Cases cover append and
replace, spaces and tabs around the colon, duplicate values on one line and
across lines, no final newline, a missing closing brace, an empty file, a
closing comma, an escaped quote, all seven forks, a mixed set, a symlinked settings file
(which becomes a regular file, as with sed -i), and missing or empty colour
files.

## Deliberate differences

- A missing or unreadable colour file prints an error to stderr. The shell's
  `cat` also printed one; both continue with an empty colour.
- `grep`'s unescaped `.` in `"material-code.primaryColor"` can match one
  arbitrary character; this implementation requires the literal key. Real
  settings use the literal.
- sed's replacement expansion (`&`, `\1`) is not reproduced. The colour is a hex
  string, so no expansion characters occur.
