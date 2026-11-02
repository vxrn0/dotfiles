#!/usr/bin/env bash
# Local CI: same checks the workflow runs.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec bash tests/test_config.sh
