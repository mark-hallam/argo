#!/usr/bin/env bash
# Lint: refuse static `next/headers` / `next/cookies` imports in client-
# bundled files under web/. Recurrence-prevention for P0 #2027, where
# `web/auth.config.ts` statically imported `next/headers` and broke the
# SPA client SSR chunk at cold dev start (every in-product route 500'd
# until the import was removed).
#
# Detection heuristic ("is this file client-bundled?"):
#   ALL .ts / .tsx files under the target directory are treated as
#   client-eligible EXCEPT files whose path matches one of the
#   documented server-only patterns:
#     - auth.ts                 (NextAuth server-only entry)
#     - middleware.ts           (edge runtime, server-side)
#     - instrumentation.ts      (Next.js OpenTelemetry hook, server-only)
#     - *.server.ts / *.server.tsx  (explicit convention)
#     - **/route.ts / **/route.tsx  (App Router route handlers)
#     - **/*.test.ts(x), __tests__/**, tests/**  (vitest + playwright,
#       never enter the SPA bundle)
#
#   Note: `auth.config.ts` is deliberately NOT exempt — it's the file
#   the original P0 #2027 regressed in, and it must stay client-safe.
#
# Per-file opt-out: include the comment
#   // allowlist:no-server-imports-in-client
# in the first 5 lines of the file. Use sparingly; the canonical fix is
# to convert the import to a dynamic `await import("next/headers")`
# inside a server-only function body, as web/lib/auth/session-cookie.ts
# does.
#
# Limitation: static-import detection is line-scoped — multi-line import
# forms split across newlines are not caught. The codebase convention is
# single-line imports; if a multi-line form lands, tighten this script.
#
# Usage:
#   bash scripts/check-no-server-imports.sh           # scans <repo>/web/
#   bash scripts/check-no-server-imports.sh <dir>     # scans <dir>
#
# Exit codes: 0 = clean, 1 = violation, 2 = bad invocation.
#
# Performance: ~1 grep pass over .ts/.tsx files (excluding node_modules
# / build-output dirs). Completes in <1s on the current web/ tree.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="${1:-$REPO_ROOT/web}"

if [ ! -d "$TARGET_DIR" ]; then
  echo "check-no-server-imports: target dir not found: $TARGET_DIR" >&2
  exit 2
fi

TARGET_DIR="$(cd "$TARGET_DIR" && pwd)"

# Single recursive grep across all .ts/.tsx files in the target dir.
# Build/vendor dirs are pruned via `--exclude-dir`. The regex matches a
# static `import ... from "next/headers"` (or 'next/cookies') line —
# dynamic `await import("...")` is intentionally NOT matched.
IMPORT_REGEX="^[[:space:]]*import[^;]*from[[:space:]]*[\"']next/(headers|cookies)[\"']"

HITS=$(grep -rnE \
  --include='*.ts' \
  --include='*.tsx' \
  --exclude-dir=node_modules \
  --exclude-dir=.next \
  --exclude-dir=storybook-static \
  --exclude-dir=playwright-report \
  --exclude-dir=test-results \
  --exclude-dir=.git \
  "$IMPORT_REGEX" \
  "$TARGET_DIR" 2>/dev/null || true)

violations=()
violation_lines=()

# Iterate over `path:line:content` records and apply path-allowlist +
# per-file opt-out filtering.
while IFS= read -r record; do
  [ -z "$record" ] && continue
  file="${record%%:*}"
  rest="${record#*:}"
  rel="${file#$TARGET_DIR/}"

  case "$rel" in
    auth.ts) continue ;;
    middleware.ts) continue ;;
    instrumentation.ts) continue ;;
    *.server.ts|*.server.tsx) continue ;;
    */route.ts|*/route.tsx) continue ;;
    *.test.ts|*.test.tsx) continue ;;
    __tests__/*) continue ;;
    tests/*) continue ;;
  esac

  # Per-file opt-out marker (first 5 lines of the file).
  if head -n 5 "$file" 2>/dev/null \
       | grep -qE '//[[:space:]]*allowlist:no-server-imports-in-client'; then
    continue
  fi

  violations+=("$rel")
  violation_lines+=("$rest")
done <<<"$HITS"

if [ "${#violations[@]}" -eq 0 ]; then
  echo "check-no-server-imports: OK ($(basename "$TARGET_DIR"))"
  exit 0
fi

{
  echo "check-no-server-imports: FAIL"
  echo
  echo "The following file(s) under $TARGET_DIR statically import from"
  echo "'next/headers' or 'next/cookies'. These are server-only APIs;"
  echo "static imports are bundled into Client Components by Turbopack"
  echo "and break the SPA at cold start (P0 #2027 regression class)."
  echo
  for i in "${!violations[@]}"; do
    printf '  %s\n      %s\n\n' "${violations[$i]}" "${violation_lines[$i]}"
  done
  echo "Fix one of:"
  echo "  - Move the import into a server-only file (rename to"
  echo "    *.server.ts, or match auth.ts / middleware.ts /"
  echo "    app/**/route.ts naming)."
  echo "  - Convert to a dynamic \`await import(\"next/headers\")\`"
  echo "    inside a server-only function body (see"
  echo "    web/lib/auth/session-cookie.ts)."
  echo "  - As a last resort, add \`// allowlist:no-server-imports-in-client\`"
  echo "    in the first 5 lines (requires justification in code review)."
} >&2

exit 1
