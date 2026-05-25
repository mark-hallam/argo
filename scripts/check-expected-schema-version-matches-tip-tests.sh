#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$REPO_ROOT" ]] || { echo "must run inside argo git repo" >&2; exit 1; }
cd "$REPO_ROOT"

SCRIPT="$REPO_ROOT/scripts/check-expected-schema-version-matches-tip.sh"
[[ -x "$SCRIPT" ]] || { echo "script not executable: $SCRIPT" >&2; exit 1; }

TMPDIR="$(mktemp -d -t expected-schema-version-tip.XXXXXX)"
OUT=""
RC=0

cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

run_check() {
  local root="${1:-$REPO_ROOT}"
  set +e
  OUT="$(ARGO_SCHEMA_CHECK_REPO_ROOT="$root" "$SCRIPT" 2>&1)"
  RC=$?
  set -e
}

pass() { printf '  ✅ %s\n' "$*"; }
fail() {
  printf '  ❌ %s\n' "$*" >&2
  echo "  output was:" >&2
  printf '%s\n' "$OUT" >&2
  exit 1
}

echo "Scenario 1: current checkout has EXPECTED_SCHEMA_VERSION matching migration tip."
run_check "$REPO_ROOT"
[[ "$RC" -eq 0 ]] || fail "expected exit 0, got $RC"
echo "$OUT" | grep -q "matches tip migration v" || fail "missing success message"
pass "current checkout passes"

FIXTURE="$TMPDIR/repo"
mkdir -p "$FIXTURE/crates/argo-store/src/migrations"
cat > "$FIXTURE/crates/argo-store/src/lib.rs" <<'RUST'
pub const EXPECTED_SCHEMA_VERSION: u32 = 83;
RUST
touch "$FIXTURE/crates/argo-store/src/migrations/v83.rs"
touch "$FIXTURE/crates/argo-store/src/migrations/v84.rs"

echo "Scenario 2: fixture adds v84.rs while EXPECTED_SCHEMA_VERSION remains 83."
run_check "$FIXTURE"
[[ "$RC" -eq 1 ]] || fail "expected exit 1, got $RC"
echo "$OUT" | grep -q "EXPECTED_SCHEMA_VERSION=83 lags behind tip migration v84" \
  || fail "missing lag message"
echo "$OUT" | grep -q "Bump crates/argo-store/src/lib.rs:1 to 84" \
  || fail "missing precise bump location"
pass "v84 fixture fails with actionable message"

echo "✅ All 2 scenarios passed."
