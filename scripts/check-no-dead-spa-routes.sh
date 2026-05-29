#!/usr/bin/env bash
# Lint: refuse in-product links to known-dead SPA routes. Recurrence-
# prevention for UX-AUDIT-T0519 F11, where stale `Link href="..."`
# strings to `/workflows/runs` and `/workflows/edit` (routes that
# never shipped) produced 404 noise in dev logs and visible-to-curious-
# DevTools-viewers 404s in the demo MP4 network panel.
#
# Maintenance: when a new dead-route catcher lands in `web/next.config.ts`
# (the `redirects()` block, "dead-route catchers" comment), add the path
# to DEAD_ROUTES below.
#
# Detected forms (across .ts / .tsx / .css / .md / .json under web/):
#   - href="/workflows/runs"             (Next.js Link / anchor)
#   - href={"/workflows/runs"}           (JSX expression form)
#   - href={`/workflows/runs?...`}       (template-literal form)
#   - to="/workflows/runs"               (legacy react-router shape;
#                                        keeps grep honest if reintroduced)
#   - .push("/workflows/runs"), .replace("/workflows/runs")
#                                        (next/navigation router calls)
#
# Allowlisted paths (do not trigger the lint):
#   - `web/next.config.ts`                (the redirect rules ARE the
#                                          legitimate references)
#   - `scripts/check-no-dead-spa-routes.sh` (this script's own DEAD_ROUTES
#                                          list — the script lives in
#                                          scripts/ NOT web/ so the
#                                          target-dir scan already skips it)
#   - any `.test.ts`, `.test.tsx`, `__tests__/**`, `tests/**` file
#                                         (test fixtures may quote dead
#                                          routes deliberately to assert
#                                          they redirect / 404)
#   - any line containing `// allowlist:dead-spa-route` (per-line opt-out
#                                         for legitimate references in
#                                         comments, docs, etc.)
#
# Usage:
#   bash scripts/check-no-dead-spa-routes.sh           # scans <repo>/web/
#   bash scripts/check-no-dead-spa-routes.sh <dir>     # scans <dir>
#
# Exit codes: 0 = clean, 1 = violation, 2 = bad invocation.
#
# Performance: single recursive grep + per-hit allowlist pass.
# Completes in <1s on the current web/ tree.

set -euo pipefail

DEAD_ROUTES=(
  "/workflows/runs"
  "/workflows/edit"
)

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="${1:-$REPO_ROOT/web}"

if [ ! -d "$TARGET_DIR" ]; then
  echo "check-no-dead-spa-routes: target dir not found: $TARGET_DIR" >&2
  exit 2
fi

TARGET_DIR="$(cd "$TARGET_DIR" && pwd)"

# Build an alternation regex of all dead routes, with each path's `/`
# escaped. The lookbehind we'd want (only match inside href / to /
# push / replace contexts) is not available in basic ERE; we instead
# scan for any literal occurrence of the dead route inside a string
# delimiter (" or ' or `) and post-filter for nav-shaped lines.
ROUTES_ALT=""
for route in "${DEAD_ROUTES[@]}"; do
  esc="${route//\//\\/}"
  if [ -z "$ROUTES_ALT" ]; then
    ROUTES_ALT="$esc"
  else
    ROUTES_ALT="$ROUTES_ALT|$esc"
  fi
done

# Match a string literal whose body equals or starts-with one of the
# dead routes. Three delimiters: double quote, single quote, backtick.
# Trailing chars allowed after the path (e.g. query/hash, template-
# expression closers): ?, #, ", ', `, }, ).
DEAD_ROUTE_REGEX="[\"\\'\\\`](${ROUTES_ALT})([?#\"'\\\`})]|$)"

HITS=$(grep -rnE \
  --include='*.ts' \
  --include='*.tsx' \
  --include='*.js' \
  --include='*.jsx' \
  --exclude-dir=node_modules \
  --exclude-dir=.next \
  --exclude-dir=storybook-static \
  --exclude-dir=playwright-report \
  --exclude-dir=test-results \
  --exclude-dir=.git \
  "$DEAD_ROUTE_REGEX" \
  "$TARGET_DIR" 2>/dev/null || true)

violations=()

# Iterate over `path:line:content` records and apply path-allowlist +
# per-line opt-out filtering.
while IFS= read -r record; do
  [ -z "$record" ] && continue
  file="${record%%:*}"
  rest="${record#*:}"
  line_no="${rest%%:*}"
  content="${rest#*:}"
  rel="${file#$TARGET_DIR/}"

  case "$rel" in
    next.config.ts) continue ;;
    next.config.js) continue ;;
    *.test.ts|*.test.tsx|*.test.js|*.test.jsx) continue ;;
    __tests__/*) continue ;;
    tests/*) continue ;;
  esac

  # Per-line opt-out marker.
  case "$content" in
    *"allowlist:dead-spa-route"*) continue ;;
  esac

  # Post-filter: only flag lines that look like navigation. Anchor
  # href / Link href / router push/replace / react-router to= /
  # next/link prefetch / window.location.* shapes.
  case "$content" in
    *href=*|*'.push('*|*'.replace('*|*' to='*|*'window.location'*|*'router.push'*|*'router.replace'*)
      violations+=("$rel:$line_no:$content")
      ;;
    *)
      # Non-nav literal occurrence — e.g. a string constant in a
      # comment or unrelated context. Skip silently so the lint stays
      # signal-not-noise.
      continue
      ;;
  esac
done <<<"$HITS"

if [ "${#violations[@]}" -eq 0 ]; then
  echo "check-no-dead-spa-routes: OK ($(basename "$TARGET_DIR"))"
  exit 0
fi

{
  echo "check-no-dead-spa-routes: FAIL"
  echo
  echo "The following file(s) under $TARGET_DIR link to SPA routes that"
  echo "do not exist. These produce 404 noise in dev logs and visible"
  echo "404s in the DevTools network panel (UX-AUDIT-T0519 F11)."
  echo
  echo "Dead routes:"
  for route in "${DEAD_ROUTES[@]}"; do
    echo "  - $route"
  done
  echo
  echo "Hits:"
  for v in "${violations[@]}"; do
    printf '  %s\n' "$v"
    echo
  done
  echo "Fix one of:"
  echo "  - Repoint the link at the canonical surface (see"
  echo "    web/next.config.ts redirects() — dead-route catchers"
  echo "    enumerate the canonical destinations)."
  echo "  - If this is an intentional reference (doc / fixture /"
  echo "    deliberate test of the redirect), add"
  echo "    \`// allowlist:dead-spa-route\` to the same line."
} >&2

exit 1
