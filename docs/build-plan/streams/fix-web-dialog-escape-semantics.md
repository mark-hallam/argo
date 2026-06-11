# fix-web-dialog-escape-semantics

**Owner**: claude or codex — web-only, DEC-006 fast lane.
**Depends on**: none (#2842/#2843/#2844 all MERGED).
**Authored against**: origin/main `3e5b1da91` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-web-dialog-escape-semantics

## Problem (the LAST red file in the whole web vitest suite)

After the harness root-fix (#2841) and the three residual briefs
(#2842 lint, #2843 CSP pin, #2844 quality cluster), exactly ONE failing
test file remains on main: `web/tests/frontend-final-quality.test.ts` —
"pins keyboard and dialog semantics on camera-visible controls" — a
camera-visible dialog/control component's source lacks
`event.key === "Escape"` handling (keyboard dismissal accessibility).
Fixing this makes the FULL `vitest run` green for the first time,
restoring whole-suite green as a meaningful DEC-006 signal.

## Fix

Read the failing assertion to identify the exact component(s) it greps,
then add the standard Escape-to-dismiss handling matching how sibling
dialogs in `web/components/` already do it (grep for an existing
`Escape` handler pattern first — reuse the established idiom, don't
invent a new hook). Keyboard semantics only; no visual change (locked
brand untouched).

## May touch

- the component file(s) the pin names (enumerate in the PR body)
- the test ONLY to extend, never weaken

## Acceptance / verifier

1. `npx vitest run tests/frontend-final-quality.test.ts` green.
2. **Full `npx vitest run`: 0 failed test files** — state the final
   files/tests counts in the PR body (this is the headline).
3. `tsc --noEmit` clean; the touched surface still renders (dev 200).

~10–40 LOC. **🟡 P3 — the last mile to a fully-green web suite.**
