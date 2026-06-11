# fix-web-vitest-sessionprovider-harness-debt

**Owner**: claude or codex — web/test-only, DEC-006 fast lane.
**Depends on**: none.
**Authored against**: origin/main `e8b569be4` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-web-vitest-sessionprovider-harness-debt

## Problem (worker-reproduced on clean main, 2026-06-12, PR #2839 body)

A full `pnpm --dir web vitest run` on clean `origin/main` (`167feed0e`) has
**15 test FILES / 30 tests failing**, dominated by the next-auth harness
error *"`useSession` must be wrapped in a `<SessionProvider />`"* (the
#2839 worker reproduced the identical failure set on the untouched main
checkout — full file list in the PR #2839 body). Consequences: the
web-only DEC-006 fast lane's "vitest green" requirement is only satisfiable
per-touched-file, every web PR pays a triage cost to prove failures are
pre-existing, and real regressions can hide inside the known-red set.

## Fix

Root-fix the harness, not the tests: add the missing provider wrapping at
the shared test-utils level (a `renderWithProviders` helper or a vitest
setup-file mock for `next-auth/react`'s `useSession` — grep
`web/test-utils` + `vitest.setup` for the existing harness shape first and
extend it; do NOT hand-wrap 15 files individually if a shared seam exists).
Any failure in the set that is NOT the SessionProvider class gets
diagnosed: fix if trivial, otherwise document per-file in the PR body with
the failure mode.

## May touch

- `web/` test files, test-utils, vitest setup/config only — NO production
  component changes (if a fix seems to need one, STOP and split a brief).

## Acceptance / verifier

1. Full `pnpm --dir web exec vitest run` on the PR tree: 0 failing test
   FILES (or every remaining failure documented per-file in the PR body
   with evidence it predates this brief and is not SessionProvider-class).
2. `tsc --noEmit` clean.

~50–150 LOC (mostly harness). **🟡 P2 — restores "vitest green" as a
meaningful whole-suite signal for the DEC-006 lane.**
