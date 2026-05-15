#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"
violations=""
while IFS=: read -r file line code; do
  trimmed="${code#"${code%%[![:space:]]*}"}"
  [[ "$trimmed" == //* ]] && continue
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
    --glob '!**/*_tests.rs' \
    --glob '!**/*_tests_*.rs' \
    --glob '!**/tests.rs' \
    --glob '!**/tests/**' \
    --glob '!**/benches/**' || true
)
if [[ -n "$violations" ]]; then
  printf 'AuditSink::new is deprecated for production callers. Use AuditSink::new_with_tenant.\n' >&2
  printf '%s' "$violations" >&2
  exit 1
fi
