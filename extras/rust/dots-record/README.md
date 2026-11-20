# dots-record

Phase 5c: replacement for `.config/quickshell/dotfiles/scripts/videos/record.sh`.

```text
dots-record [--region "x,y WxH"] [--fullscreen] [--sound]
```

The first run starts `wf-recorder`; a second run stops it. The shell version
does not keep a state file: it asks `pgrep -x wf-recorder` and uses `pkill -x`.
This binary keeps that contract, so a recorder started by either implementation
can be stopped by the other.

- Recording directory: `.screenRecord.savePath` from
  `$HOME/.config/dotfiles/config.json`, or `$HOME/Videos` when the field is
  missing, null or empty.
- Output: `recording_YYYY-MM-DD_HH.MM.SS.mp4` inside that directory. The
  timestamp comes from `localtime_r`, so `TZ` behaves like `date`.
- `--fullscreen` adds `-o <focused monitor>` from `hyprctl monitors -j`.
- `--region` adds `--geometry <region>`. Without both flags, `slurp` selects a
  region; a cancelled selection notifies and exits 1.
- `--sound` adds `--audio=<default sink>.monitor`, falling back to the first
  source line containing `monitor`.
- Arguments are passed to `-x yuv420p -f <file>` in that order. The final
  `exec` means the caller sees `wf-recorder`'s own PID and exit status.

## Verification

`tests/compare.sh` stubs `pgrep`, `pkill`, `notify-send`, `slurp`, `hyprctl`,
`pactl` and `wf-recorder`, redirects every XDG path, and diffs the recording
tree, exit status and every argument log:

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

Measured 2025-11-21: 15 cases, 15 matched, 0 failed. Cases cover fullscreen,
default and fallback audio, no audio source, explicit region, slurp success and
cancellation, stop, custom/missing/null/invalid config, a non-zero recorder
exit, unknown flags and a failing `hyprctl`.

## Deliberate differences

- A missing or unreadable config, and a failing `pactl get-default-sink`, now
  print an error to stderr. The shell swallowed both with `2>/dev/null` and
  silently used the fallback.
- `--region` with no value made the shell loop forever (`shift 2` on one
  argument never advances). The binary treats the region as empty and continues.
- Notifications keep the script's text exactly: the start notification shows
  just the file name, the stop notification the directory.
