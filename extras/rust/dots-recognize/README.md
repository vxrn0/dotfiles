# dots-recognize

Phase 5c: replacement for
`.config/quickshell/dotfiles/scripts/musicRecognition/recognize-music.sh`.

```text
dots-recognize [-i INTERVAL] [-t DURATION] [-s monitor|input]
```

Defaults are interval 2, duration 30 and source `monitor`, matching the shell.
`getopts` forms are accepted: `-i 2`, `-i2`, `--` ends options, and parsing
stops at the first non-option.

The audio device is `<default sink>.monitor` for `monitor`, or the third field
of `pactl info`'s `Default Source:` line for `input`. An invalid source prints
`Invalid source type` and exits 1. A missing `songrec`, an empty device, or a
device not present in `pactl list short sources` also exits 1.

`songrec listen --audio-device <dev> --request-interval <n> --json
--disable-mpris` is started once. Its stdout is read line by line; the first
line containing `"matches": [` is printed and the command exits 0. After
`DURATION` the child gets SIGTERM and the command exits 0. Durations accept the
`sleep` suffixes `s`, `m`, `h`, `d` and fractions.

The shell used a `mktemp` FIFO; this uses a pipe and creates no temporary file.
The visible contract (one JSON line on stdout, exit 0) is the same.

## Verification

`tests/compare.sh` stubs `songrec` and `pactl`, redirects all XDG paths and the
fake songrec never reaches the network. It compares stdout, exit status and the
argument logs; the timer case is bounded by a real 0.2 s timeout.

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

Measured 2025-11-21: 14 cases, 14 matched, 0 failed. Cases cover monitor and
input sources, default sink failure, a missing default source, a missing device,
match and empty-match lines, no match, timeout kill, attached getopts values,
defaults, invalid source, missing songrec, unknown flag and trailing non-option
arguments.

## Deliberate differences

- No `/tmp/songrec_out_XXXXXX` FIFO is created. A reader of stdout cannot tell
  the difference; `/tmp` is left cleaner.
- A failure to spawn `songrec` exits 1 with an error. The shell would wait
  forever on a FIFO with no writer (until the timer); that is not reproducible
  without also reproducing a hang.
- `pactl get-default-sink` keeps its stderr (the shell did not redirect it).
- An invalid duration prints a warning and runs without a timeout. The shell's
  `sleep` printed its own error and exited immediately, which also left the
  recogniser without a timeout.
