#!/usr/bin/env bash
# lint-no-audit-sink-new-in-production_tests.sh — golden tests for
# the AuditSink::new production guard. Asserts: (1) production
# violation FAILS, (2) `*_test_helpers.rs` SKIPPED, (3) `tests/`
# integration file SKIPPED, (4) file-level `#![allow(deprecated)]`
# SKIPPED.
set -uo pipefail
SCRIPT="$(cd "$(dirname "$0")" && pwd)/lint-no-audit-sink-new-in-production.sh"
PASS=0 FAIL=0

ok()   { echo "✓ $1"; PASS=$((PASS+1)); }
ko()   { echo "✗ $1" >&2; FAIL=$((FAIL+1)); }
mkrepo() {
  local d="$1"
  git -C "$d" init --initial-branch=main >/dev/null 2>&1
  git -C "$d" config user.name lint-test
  git -C "$d" config user.email lint-test@test.invalid
  git -C "$d" config commit.gpgsign false
  mkdir -p "$d/scripts"
  cp "$SCRIPT" "$d/scripts/lint-no-audit-sink-new-in-production.sh"
  chmod +x "$d/scripts/lint-no-audit-sink-new-in-production.sh"
}
case_run() {  # name expected_rc fixture_path fixture_body extra_assert_pattern
  local name="$1" want="$2" path="$3" body="$4" pat="${5:-}"
  local tmp; tmp="$(mktemp -d)"; mkrepo "$tmp"
  mkdir -p "$(dirname "$tmp/$path")"; printf '%s' "$body" > "$tmp/$path"
  git -C "$tmp" add . >/dev/null && git -C "$tmp" commit -m i >/dev/null
  local rc=0 out err
  out="$(mktemp)"; err="$(mktemp)"
  ( cd "$tmp" && bash scripts/lint-no-audit-sink-new-in-production.sh ) >"$out" 2>"$err" || rc=$?
  [[ "$rc" == "$want" ]] && ok "$name: rc=$want" || ko "$name: rc=$rc want=$want"
  if [[ -n "$pat" ]]; then
    grep -q -- "$pat" "$out" "$err" 2>/dev/null && ok "$name: pattern '$pat'" || ko "$name: pattern '$pat' missing"
  fi
  rm -rf "$tmp" "$out" "$err"
}

PROD='use argo_audit::AuditSink; use std::sync::Arc;
pub fn s(x: Arc<()>) -> AuditSink { AuditSink::new(x) }
'
HELPER='use argo_audit::AuditSink; use std::sync::Arc;
pub(super) fn s(x: Arc<()>) -> AuditSink { AuditSink::new(x) }
'
INTG='use argo_audit::AuditSink; use std::sync::Arc;
#[test] fn t() { let _ = AuditSink::new(Arc::new(())); }
'
DEPR='//! legacy shim.
#![allow(deprecated)]
use argo_audit::AuditSink; use std::sync::Arc;
pub fn s(x: Arc<()>) -> AuditSink { AuditSink::new(x) }
'

case_run "production"        1 crates/argo-cli/src/handlers/prod_route.rs              "$PROD"   "prod_route.rs"
case_run "test-helper"       0 crates/argo-cli/src/handlers/routes_x_test_helpers.rs   "$HELPER" "OK: 0 production"
case_run "integration-test"  0 crates/argo-cli/tests/integration.rs                    "$INTG"   "OK: 0 production"
case_run "allow-deprecated"  0 crates/argo-cli/src/handlers/legacy_shim.rs             "$DEPR"   "OK: 0 production"

echo
echo "=== Results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
