#!/usr/bin/env bash
# Heuristic audit of `ArgoError` variant misuse.
#
# Reports counts of call sites where the chosen variant likely doesn't
# match the canonical contract (see `crates/argo-types/src/error.rs`
# module-level docs). WARNING-ONLY by default; pass `--strict` to make
# any non-zero count fail with exit 1.
#
# Pairs with `docs/skycot/ERROR_VARIANT_AUDIT.md`. Tracked under brief
# `claude-error-types-consolidation`; the follow-up
# `claude-error-types-rewrite-callsites` mechanically rewrites and flips
# CI to strict.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

STRICT=0
[ "${1:-}" = "--strict" ] && STRICT=1

# Each line: <recommended-variant>|<one-line-rationale>|<rg pattern>
PATTERNS=(
  "Io|local FS / process I/O — use ArgoError::Io|ArgoError::Provider\(format!\(\"(read |write |create |remove |copy |mkdir |read_dir |cwd|current_dir|file_type )"
  "Schema|caller-supplied bad input — use ArgoError::Schema|ArgoError::Provider\(format!\(\"(invalid |missing |bad |unknown |unparseable )"
  "Schema|missing field — use ArgoError::Schema|ArgoError::Provider\([^)]*missing field"
  "Invariant|internal data corruption — use ArgoError::Invariant|ArgoError::Provider\(format!\(\"(uuid |decode |timestamp |script |unreachable)"
  "Io|reqwest is downstream HTTP transport — fold into Provider, but reqwest::Error in Internal/Invariant is wrong|ArgoError::(Internal|Invariant)\([^)]*reqwest"
  "Schema|serde_json parse failure on caller input is Schema, not Internal|ArgoError::(Internal|Invariant)\([^)]*serde_json"
)

total=0
echo "ArgoError variant-usage audit (heuristic, warning-only by default)"
echo "------------------------------------------------------------------"
for entry in "${PATTERNS[@]}"; do
  variant="${entry%%|*}"
  rest="${entry#*|}"
  rationale="${rest%%|*}"
  pattern="${rest#*|}"
  count=$({ { grep -rIn --include='*.rs' --exclude-dir=target -E "$pattern" crates 2>/dev/null || true; } \
    | { grep -vE '^crates/argo-types/src/error\.rs:' || true; } | wc -l | tr -d ' '; })
  total=$((total + count))
  printf '  %4d  → recommend %s — %s\n' "$count" "$variant" "$rationale"
done
echo "------------------------------------------------------------------"
echo "Total flagged sites: $total"
echo
echo "See docs/skycot/ERROR_VARIANT_AUDIT.md for the full audit. Rewrites"
echo "land via brief claude-error-types-rewrite-callsites."

if [ "$STRICT" -eq 1 ] && [ "$total" -gt 0 ]; then
  echo "FAIL: --strict mode enabled and $total flagged sites remain." >&2
  exit 1
fi
