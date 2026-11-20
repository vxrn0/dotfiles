# dots-keyring

Phase 5c: replacements for `.config/quickshell/dotfiles/scripts/keyring/unlock.sh`
and `is_unlocked.sh`. Two binaries share one library because the scripts are
two entry points to the same check.

```text
dots-keyring-is-unlocked
dots-keyring-unlock
```

`is-unlocked` asks `busctl --user get-property org.freedesktop.secrets
/org/freedesktop/secrets/collection/login org.freedesktop.Secret.Collection
Locked`; only the exact reply `b false` counts as unlocked. It prints
`Keyring is unlocked` / `Keyring is locked` to stderr and exits 0 / 1.

`unlock` first runs the same check and exits 1 when the keyring is already open.
Otherwise it takes `UNLOCK_PASSWORD` when non-empty, or prompts on stderr and
reads one line with terminal echo disabled. Leading and trailing spaces and tabs
are trimmed exactly like bash `read`; EOF prints `No password given`. An empty
result prints `Empty password`; both exit 1. It then runs
`killall -q -u $(whoami) gnome-keyring-daemon`, pipes the password without a
newline to `gnome-keyring-daemon --daemonize --login`, waits, and prints the
final blank line. Exit status is 0 even if the daemon fails, as in the shell.

## Verification

`tests/compare.sh` stubs `busctl`, `whoami`, `killall` and
`gnome-keyring-daemon`, redirects all XDG paths, and compares exit status,
stdout, stderr, every argument log and the exact daemon stdin bytes:

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

Measured 2025-11-21: 13 cases, 13 matched, 0 failed. Cases cover unlocked,
locked, busctl failure, already unlocked, env password, prompted password,
empty env, spaces-only password, trimming, empty line, EOF, killall failure and
daemon failure.

## Deliberate differences

- The prompt disables echo through `termios` instead of shell `read -s`; both
  leave the terminal in the same state. There is no TTY in the harness, where
  the reference also just reads the pipe.
- A failure to spawn `gnome-keyring-daemon` is printed. The shell printed
  `command not found` too, then continued and exited 0; this does the same.
- The shell exported the daemon's output variables into its own short-lived
  process, where they were lost on exit. The binary reads and discards that
  output; the visible behaviour is identical.
