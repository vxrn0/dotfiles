#!/usr/bin/env bash
# Keep a rolling three-day window of clipboard history.
#
# cliphist stores no timestamps, so age cannot be queried. Instead each run records the
# newest entry id, and the next run deletes everything up to that id. Run every three
# days: what survives is exactly the entries copied since the previous run.
set -u

state="${XDG_STATE_HOME:-$HOME/.local/state}/cliphist-prune.id"
db="${CLIPHIST_DB:-$HOME/.cache/cliphist/db}"
cliphist=(cliphist -db-path "$db")

newest=$("${cliphist[@]}" list | head -n1 | cut -f1)
[[ -z "$newest" ]] && exit 0

if [[ -s "$state" ]]; then
    mark=$(<"$state")
    [[ "$mark" =~ ^[0-9]+$ ]] && "${cliphist[@]}" list \
        | awk -F'\t' -v m="$mark" '$1+0 <= m+0' \
        | "${cliphist[@]}" delete
fi

mkdir -p "$(dirname "$state")"
printf '%s\n' "$newest" > "$state"
