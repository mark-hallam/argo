#!/usr/bin/env bash
# lint-no-audit-sink-new-in-production.sh — enforce the production
# replacement for the deprecated AuditSink::new constructor.
#
# AuditSink::new is `#[deprecated]` because it drops the tenant_id
# scoping required for cross-tenant audit isolation. Production
# callers must use AuditSink::new_with_tenant. This lint guards the
# constraint.
#
# Documented exceptions (operator runbook): see
# docs/skycot/security/audit-sink-policy.md.
#
# Skip rules (fail-closed: when in doubt, surface the violation and
# let the operator add an explicit exception):
#   - path ends in `_test_helpers.rs`, `_tests.rs`, or `tests.rs`
#   - any path component is `tests` or `benches` (cargo test/bench
#     roots and integration-test crates)
#   - file opens with a top-of-file `#![allow(deprecated)]` inner
#     attribute (per-function `#[allow(...)]` does NOT count)
#   - the matched line is a `//` comment
#   - the matched line is inside a `#[cfg(test)] mod tests { … }` block
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

file_is_test_scope() {
  local f="$1"
  case "$f" in
    *_test_helpers.rs|*_tests.rs|*/tests.rs) return 0 ;;
    */tests/*|*/benches/*)                   return 0 ;;
  esac
  # File-level inner attribute. Scan the first 20 lines so a leading
  # doc comment / module preamble doesn't hide the marker, but a stray
  # mid-file `allow(deprecated)` won't trip the skip.
  if head -n 20 "$f" 2>/dev/null \
       | grep -Eq '^[[:space:]]*#!\[[^]]*allow\([^)]*\bdeprecated\b'; then
    return 0
  fi
  return 1
}

violations=""
while IFS=: read -r file line code; do
  trimmed="${code#"${code%%[![:space:]]*}"}"
  [[ "$trimmed" == //* ]] && continue
  if file_is_test_scope "$file"; then continue; fi
  if awk -v target="$line" '
    NR > target { exit }
    /^[[:space:]]*#\[cfg\(test\)\]/ { saw_cfg = 1 }
    saw_cfg && /^[[:space:]]*mod tests[[:space:]]*\{/ { in_tests = 1 }
    END { exit(in_tests ? 0 : 1) }
  ' "$file"; then
    continue
  fi
  violations+="$file:$line:$code"$'\n'
done < <(
  rg -n 'AuditSink::new\(' crates --type rust \
    --glob '!**/*_test_helpers.rs' \
    --glob '!**/*_tests.rs' \
    --glob '!**/*_tests_*.rs' \
    --glob '!**/tests.rs' \
    --glob '!**/tests/**' \
    --glob '!**/benches/**' || true
)

if [[ -n "$violations" ]]; then
  printf 'AuditSink::new is deprecated for production callers. Use AuditSink::new_with_tenant.\n' >&2
  printf 'See docs/skycot/security/audit-sink-policy.md for the exception process.\n' >&2
  printf '%s' "$violations" >&2
  exit 1
fi

printf 'OK: 0 production AuditSink::new callsites\n'
