#!/usr/bin/env bash
# Meta-tests for `scripts/silent-skip-nightly.sh`. Validates:
#  1. --help prints usage and exits 0.
#  2. --discover-only enumerates ≥40 test targets.
#  3. Discovered lines match the documented "crate target file"
#     shape (3 fields, file starts with `crates/`).
#  4. Unknown flag exits non-zero with a usage banner.
#
# Used by `.github/workflows/silent-skip-nightly.yml.disabled` as a
# pre-flight sanity gate. Should pass on any fresh checkout without
# running cargo or hitting real provider APIs.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUNNER="$SCRIPT_DIR/silent-skip-nightly.sh"

if [[ ! -x "$RUNNER" && ! -f "$RUNNER" ]]; then
  echo "FAIL: runner not found at $RUNNER" >&2
  exit 2
fi

heading() { echo; echo "=== $* ==="; }
fail()    { echo "FAIL: $*" >&2; exit 1; }
ok()      { echo "  OK: $*"; }

heading "1. --help prints usage and exits 0"
help_out=$(bash "$RUNNER" --help 2>&1)
help_rc=$?
[[ $help_rc -eq 0 ]] || fail "--help exited $help_rc"
echo "$help_out" | grep -q "Usage: scripts/silent-skip-nightly.sh" \
  || fail "--help missing 'Usage:' banner"
echo "$help_out" | grep -q "ARGO_SILENT_SKIP_TIMEOUT_SECS" \
  || fail "--help missing timeout env var"
echo "$help_out" | grep -q "\-\-local" \
  || fail "--help missing --local flag"
echo "$help_out" | grep -q "\-\-discover-only" \
  || fail "--help missing --discover-only flag"
ok "usage banner + flag + env-var documentation present"

heading "2. --discover-only enumerates ≥40 test targets"
disc_out=$(bash "$RUNNER" --discover-only 2>/dev/null)
disc_rc=$?
[[ $disc_rc -eq 0 ]] || fail "--discover-only exited $disc_rc"
# Drop the trailing blank line + nothing else.
count=$(echo "$disc_out" | grep -cE ' crates/' || true)
if [[ "$count" -lt 40 ]]; then
  fail "discovered only $count targets (expected ≥40)"
fi
ok "discovered $count targets (≥40)"

heading "3. Discovery lines have 'crate target file' shape"
malformed=$(echo "$disc_out" | grep -E ' crates/' \
  | awk 'NF != 3 || $3 !~ /^crates\// { print NR": "$0 }')
[[ -z "$malformed" ]] || fail "malformed discovery lines: $malformed"
# Sample a line and confirm file actually exists.
sample=$(echo "$disc_out" | grep -E ' crates/' | head -1)
sample_file=$(echo "$sample" | awk '{ print $3 }')
[[ -n "$sample_file" ]] || fail "could not extract sample file path"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
[[ -f "$REPO_ROOT/$sample_file" ]] \
  || fail "discovered file does not exist on disk: $sample_file"
ok "discovery output shape valid; sample file exists"

heading "4. Unknown flag exits non-zero with usage banner"
bad_out=$(bash "$RUNNER" --bogus-flag 2>&1)
bad_rc=$?
[[ $bad_rc -ne 0 ]] || fail "unknown flag exited 0"
echo "$bad_out" | grep -q "unknown flag" \
  || fail "unknown-flag message missing"
ok "unknown flag rejected with usage banner"

echo
echo "✓ silent-skip-nightly meta-tests PASSED"
