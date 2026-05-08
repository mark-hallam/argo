#!/usr/bin/env bash
# Enforce sqlx prepared-statement caching on hot-path query sites.
#
# Rule: hot-path files must not build SQL strings via
# format!("SELECT|INSERT|UPDATE|DELETE ..."). Such strings bypass sqlx's
# per-connection prepared-statement cache and re-parse on every call.
# Use sqlx::query("... ? ...").bind(v) or query!/query_as! macros.
#
# Exception: identifier substitution (table/column name from trusted
# code, not data) cannot be parameterised. Mark such a line with
# `// audit-allow: identifier substitution` on the same line or the
# line immediately above the offending format!().
#
# Inventory tracks brief claude-perf-store-prepared-statements.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

shopt -s nullglob
files=(
  crates/argo-store/src/*query*.rs
  crates/argo-cli/src/handlers/audit_*.rs
  crates/argo-cli/src/handlers/session_*.rs
  crates/argo-cli/src/handlers/message_*.rs
)
shopt -u nullglob

if [ "${#files[@]}" -eq 0 ]; then
  echo "OK: no hot-path files matched the inventory globs."
  exit 0
fi

pattern='format!\("(SELECT|INSERT|UPDATE|DELETE)'
offences=()

for f in "${files[@]}"; do
  while IFS=: read -r lineno match; do
    [ -z "$lineno" ] && continue
    # Same-line marker.
    if printf '%s' "$match" | grep -q 'audit-allow:'; then
      continue
    fi
    # Previous-line marker.
    prev_no=$((lineno - 1))
    if [ "$prev_no" -ge 1 ]; then
      prev=$(sed -n "${prev_no}p" "$f")
      if printf '%s' "$prev" | grep -q 'audit-allow:'; then
        continue
      fi
    fi
    offences+=("$f:$lineno: $(printf '%s' "$match" | sed 's/^[[:space:]]*//')")
  done < <(grep -nE "$pattern" "$f" || true)
done

if [ "${#offences[@]}" -gt 0 ]; then
  echo "FAIL: ad-hoc SQL via format!() in hot-path files (${#offences[@]} site(s)):" >&2
  for o in "${offences[@]}"; do
    printf '  %s\n' "$o" >&2
  done
  cat >&2 <<'HINT'

Convert each offending line to a parameterised query:
  sqlx::query("SELECT ... WHERE x = ?").bind(value)
or use the compile-time-checked macros:
  sqlx::query!(...) / sqlx::query_as!(...)

If the SQL legitimately needs identifier substitution (table/column
name from trusted code, not data), add this marker on the same line
or the line immediately above:
  // audit-allow: identifier substitution
HINT
  exit 1
fi

echo "OK: ${#files[@]} hot-path file(s) clean."
