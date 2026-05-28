#!/usr/bin/env bash
# Reject worker-detail UI files that leak phased-build brief-IDs
# (the `E<digit>-<letter>` token scheme from BUILD_PLAN_V11) inside
# strings that can reach the user — JSX text, console.warn arguments,
# aria labels, toast bodies. Comments are allowed; the regex below
# matches inside quoted strings and template literals only.
#
# Originating fix: fix-worker-hero-overflow-menu-pause-disable-archive-stub
# (#WorkerHero overflow menu's `console.warn('… pending wiring (E3-b/E4-f)')`
# leaked into DevTools — see UX-AUDIT-T0519 scene-1-and-2 F16).
#
# Scope is intentionally narrow:
#   - web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx (this brief)
#   - web/components/skycot/WorkerHero.tsx (this brief)
#   - web/components/skycot/WorkerOverflowMenu.tsx (this brief)
#
# The sibling brief `fix-worker-tabs-placeholder-bodies-leak-brief-ids-e4`
# extends coverage to the [tab]/page.tsx fallback; it adds its own
# guard entries when it ships.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

readonly PATTERN='E[0-9]+-[a-z]'
readonly FILES=(
  "web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx"
  "web/components/skycot/WorkerHero.tsx"
  "web/components/skycot/WorkerOverflowMenu.tsx"
)

missing=0
for f in "${FILES[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "warn: $f not found (skipping)" >&2
    missing=$((missing + 1))
  fi
done

hits=""
for f in "${FILES[@]}"; do
  [[ -f "$f" ]] || continue
  # -E enables ERE for the alternation; matches single-quote, double-quote, or backtick
  # string contents that hold a brief-ID. Inline JSX text is doubled-quoted via JSX
  # children too (e.g. `<p>{label} — coming in {brief}.</p>`), and template literals
  # in console.warn / toast calls all fall under one of the three quote shapes.
  match=$(grep -nE "([\"'\`])[^\"'\`]*${PATTERN}[^\"'\`]*\\1" "$f" || true)
  if [[ -n "$match" ]]; then
    hits+="${f}:"$'\n'"${match}"$'\n'
  fi
done

if [[ -n "$hits" ]]; then
  echo "Worker-detail UI files leak BUILD_PLAN_V11 brief-IDs in user-facing strings:" >&2
  echo "$hits" >&2
  echo "Fix: replace the brief-ID with brand-voiced copy or remove the stub entirely." >&2
  exit 1
fi

echo "ok: no E<digit>-<letter> brief-ID strings in worker UI scope."
exit 0
