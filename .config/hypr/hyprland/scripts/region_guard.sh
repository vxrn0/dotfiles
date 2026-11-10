#!/usr/bin/env bash
# Decide whether a region keybind still needs its external fallback.
#
# Each region bind is registered twice in Hyprland: a global shortcut that Quickshell
# handles, and an exec fallback for when the shell is not running. Both fire on the same
# key press, so the fallback must never run while the shell owns the bind.
#
# The old version waited for the selector layer and gave up after five seconds. A cold
# first press compiles the RegionSelection QML and can take longer than that, so the
# fallback fired while the shell was opening its own picker: two pickers at once.
#
# Exit 0: the shell owns the bind - the fallback must not run.
# Exit 1: no shell - run the fallback.

# A selection UI is already on screen; a second one would fight it.
pgrep -x slurp >/dev/null && exit 0

# Alive shell means the global shortcut handles the press, even on a slow cold start.
pgrep -x qs >/dev/null && exit 0

exit 1
