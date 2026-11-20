# dots-prune

Phase 5c: replacement for `.config/quickshell/dotfiles/scripts/clipboard/prune.sh`.

cliphist has no timestamps. Each run stores the newest entry id in
`$XDG_STATE_HOME/cliphist-prune.id` (default
`$HOME/.local/state/cliphist-prune.id`). The next run deletes every entry whose
id is less than or equal to that mark, then stores the current newest id.
`CLIPHIST_DB` overrides `$HOME/.cache/cliphist/db`.

The numeric comparison copies awk's `$1+0`: a missing or non-numeric first
field counts as zero, and only a numeric prefix is used. An empty history exits
0 without touching the state file. An empty or non-numeric state file means "no
mark": nothing is deleted, but the new mark is still written. The `cliphist
delete` pipeline still runs with empty input when nothing matches.

## Verification

`tests/compare.sh` stubs `cliphist` with a real little database, redirects all
XDG paths, and compares the database, state file, exit status and command log
after one or more runs:

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

Measured 2025-11-21: 12 cases, 12 matched, 0 failed. Cases cover first run,
delete-all, partial deletion, invalid/empty/zero/huge marks, no final newline,
empty database, non-numeric ids, a failing `cliphist list`, and two consecutive
runs.

The command log is compared sorted because the shell starts `list` and `delete`
concurrently in one pipeline; their two log lines can appear in either order.
The resulting database and state are deterministic.

## Deliberate differences

- `cliphist` stderr is no longer redirected; it was never redirected in this
  script. There is nothing here that used `2>/dev/null`.
- A state write failure now exits 1 with an error. The shell would have failed
  on the `printf` as well, but without an explanatory line.
