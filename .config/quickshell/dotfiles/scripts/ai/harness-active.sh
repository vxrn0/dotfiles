#!/usr/bin/env bash
# Prints "codex", "claude" and/or "opencode" for every harness with a live session.
# CLI harnesses only: GUI harnesses are desktop apps and show up in the system tray.
# Daemons are skipped: `codex app-server` and `opencode serve` run all the time.
# Chat turns from the AI panel (`codex exec`, `claude -p`, `opencode run`) count as sessions.
ps -eo args= | while read -r args; do
    bin="${args%% *}"
    bin="${bin##*/}"
    case "$bin" in
        codex)    [[ "$args" == *" app-server"* ]] || echo codex ;;
        claude)   echo claude ;;
        opencode) [[ "$args" == *" serve"* ]] || echo opencode ;;
    esac
done | sort -u
