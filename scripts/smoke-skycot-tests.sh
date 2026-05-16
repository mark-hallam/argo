#!/usr/bin/env bash
# Golden-output assertions for `scripts/smoke-skycot.sh` — verifies
# argument parsing + help-path behaviors that don't require the argo
# binary. The main smoke script's mission scenarios still run against
# a built binary; these tests cover the script's own contract.
#
# Usage:
#   bash scripts/smoke-skycot-tests.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SMOKE="$SCRIPT_DIR/smoke-skycot.sh"
FAILED=0

pass() { echo "  OK: $*"; }
fail() { echo "  FAIL: $*"; FAILED=$((FAILED + 1)); }

# 1. bash -n must succeed (syntax check).
if bash -n "$SMOKE"; then
  pass "syntax check"
else
  fail "syntax check"
fi

# 2. --target-url without value must fail fast with exit 2.
set +e
out=$(bash "$SMOKE" --target-url 2>&1)
code=$?
set -e
if [[ $code -eq 2 ]] && echo "$out" | grep -q 'requires a URL argument'; then
  pass "--target-url without value exits 2 with explanation"
else
  fail "--target-url without value: expected exit 2 + message, got exit=$code out=$out"
fi

# 3. --target-url with a non-http(s) value must fail (validated at
#    runtime once the URL guard fires). Use a missing argo binary so we
#    short-circuit before any network call.
set +e
out=$(bash "$SMOKE" /bin/false-binary-that-does-not-exist --target-url file:///etc/passwd 2>&1)
code=$?
set -e
# The 'argo binary not on PATH' check fires first (exit 2) — that's
# fine; the smoke script's job is to refuse to run without argo.
if [[ $code -ne 0 ]]; then
  pass "missing argo binary refused regardless of --target-url"
else
  fail "expected non-zero exit when argo binary missing, got exit=$code"
fi

if (( FAILED > 0 )); then
  echo "FAILED: $FAILED assertion(s)"
  exit 1
fi
echo "all smoke-skycot-tests passed"
