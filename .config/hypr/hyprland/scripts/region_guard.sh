#!/usr/bin/env bash
# Decide whether a region keybind still needs its external fallback.
#
# Each region bind is registered twice in Hyprland: a global shortcut that Quickshell
# handles, and an exec fallback for when the shell is not running. Both fire on the same
# key press, so the fallback must wait for the shell to do its part.
#
# Exit 0: the shell handled it (or a picker is already up) - the fallback must not run.
# Exit 1: run the fallback.
#
# A fixed sleep was not enough: the selector layer took 4-104 ms to appear in testing, and
# under load it took longer, so the fallback fired anyway and both pickers opened at once.

# A selection UI is already on screen; a second one would fight it.
pgrep -x slurp >/dev/null && exit 0

# No shell at all: run the fallback immediately instead of waiting for nothing.
pgrep -x qs >/dev/null || exit 1

# The shell is up, so give it a moment to map the selector layer. On a cold start
# the first RegionSelection takes seconds to instantiate (QML compile), and a short
# wait let the fallback fire anyway, so both pickers opened. A warm press shows the
# layer in ~270 ms; five seconds covers the cold path and only costs a broken shell
# the same wait before its fallback.
for _ in $(seq 1 100); do
    hyprctl layers 2>/dev/null | grep -q 'quickshell:regionSelector' && exit 0
    sleep 0.05
done

# The shell is running but never opened the selector (not loaded yet, or crashed).
exit 1
