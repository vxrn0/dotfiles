#!/usr/bin/env bash
# Guards this tree against the mistakes it keeps inviting: secrets in the index,
# broken shell, and a README pointing at a missing preview.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
fail=0

pass() { printf 'ok    %s\n' "$1"; }
fail_msg() { printf 'FAIL  %s\n' "$1"; fail=1; }

while IFS= read -r script; do
    if bash -n "$script"; then pass "syntax $script"; else fail_msg "syntax $script"; fi
done < <(git ls-files '*.sh')

if git ls-files | grep -qE '(^|/)(\.env|fish_variables|credentials|id_rsa|.*\.key)$'; then
    fail_msg "secret or machine state is tracked"
else
    pass "no tracked secrets"
fi

if grep -qE '\.github/preview(-rounded)?\.webp' README.md && { [ -f .github/preview.webp ] || [ -f .github/preview-rounded.webp ]; }; then
    pass "README preview"
else
    fail_msg "README preview"
fi

if extras/install.sh --help | grep -q '^usage: install.sh'; then
    pass "install.sh --help"
else
    fail_msg "install.sh --help"
fi

exit "$fail"
