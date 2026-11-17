#!/usr/bin/env bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Unquoted $DOTFILES_VIRTUAL_ENV made `source /bin/activate` the fallback, and
# `deactivate` masked the python exit status, so region suggestions failed silently.
VENV="${DOTFILES_VIRTUAL_ENV:-$HOME/.local/state/quickshell/.venv}"
source "$VENV/bin/activate" || exit 1
"$SCRIPT_DIR/find_regions.py" "$@"
status=$?
deactivate
exit "$status"
