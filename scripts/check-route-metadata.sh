#!/usr/bin/env bash
# Lint: every page.tsx under web/app/(app)/ must declare its own
# per-route metadata (so the browser tab reads "<page> · SKYCOT" and
# never falls back to the (app) group default).
#
# Recurrence-prevention for the F4 leak (the original "Approvals"
# default in `web/app/(app)/layout.tsx` was inherited by 9 sibling
# routes, causing the browser tab to read "Approvals · SKYCOT" on
# /genesis, /brand, /retro, /billing, /verticals, /orchestration,
# /support/inbox, /brand/components and /billing/subscribe — a
# camera-readable error during demo recording).
#
# Detection: a page.tsx satisfies the rule if EITHER
#   (a) the page.tsx itself contains `export const metadata`, OR
#   (b) any ancestor layout.tsx between the page's directory and the
#       `(app)` group root (exclusive — the group layout's "default"
#       does not count) contains `export const metadata`.
#
# Usage:
#   bash scripts/check-route-metadata.sh           # scans <repo>/web/app/(app)/
#   bash scripts/check-route-metadata.sh <web>     # scans <web>/app/(app)/
#
# Exit codes: 0 = clean, 1 = orphan page found, 2 = bad invocation.
#
# Performance: O(pages × depth); completes in <1s on the current tree
# (~60 pages × ≤5 levels deep).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEB_DIR="${1:-$REPO_ROOT/web}"
GROUP_ROOT="$WEB_DIR/app/(app)"

if [ ! -d "$GROUP_ROOT" ]; then
  echo "check-route-metadata: (app) group not found at $GROUP_ROOT" >&2
  exit 2
fi

has_metadata() {
  # Grep for module-scope `export const metadata` (Next.js requires
  # this to be top-level, not inside a function or conditional).
  grep -q "^export const metadata" "$1" 2>/dev/null
}

violations=()

while IFS= read -r page; do
  # Check the page itself first.
  if has_metadata "$page"; then
    continue
  fi

  # Walk up from the page's parent dir to (but not into) GROUP_ROOT,
  # looking for a layout.tsx with metadata.
  dir="$(dirname "$page")"
  found=0
  while [ "$dir" != "$GROUP_ROOT" ] && [ "$dir" != "/" ]; do
    layout="$dir/layout.tsx"
    if [ -f "$layout" ] && has_metadata "$layout"; then
      found=1
      break
    fi
    dir="$(dirname "$dir")"
  done

  if [ "$found" -eq 0 ]; then
    violations+=("$page")
  fi
done < <(find "$GROUP_ROOT" -name "page.tsx" -type f | sort)

if [ "${#violations[@]}" -gt 0 ]; then
  echo "check-route-metadata: ${#violations[@]} page(s) lack per-route metadata" >&2
  echo "" >&2
  echo "Each page must declare its own title — either as" >&2
  echo "  export const metadata = { title: \"<Page>\" };" >&2
  echo "in the page.tsx (server components only), or via a sibling" >&2
  echo "or ancestor layout.tsx in the route subtree." >&2
  echo "" >&2
  for p in "${violations[@]}"; do
    rel="${p#$REPO_ROOT/}"
    echo "  - $rel" >&2
  done
  exit 1
fi

echo "check-route-metadata: clean (all pages under web/app/(app)/ declare per-route metadata)"
exit 0
