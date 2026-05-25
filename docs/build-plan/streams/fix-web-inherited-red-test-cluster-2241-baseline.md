# Stream fix-web-inherited-red-test-cluster-2241-baseline — fix the 3 pre-existing red web vitest cases that PR #2241 inherited

**Owner**: claude (web tests, JS/TS).
**Depends on**: none.
**Authored against**: origin/main `234ffd9f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-web-inherited-red-test-cluster-2241-baseline`.

🔴 **P1.** Source: round-47 evaluator finding (`round-47-pr-2241-workers-envelope-id-format-pre-merge-verify.md`) + 6th-consecutive carry-forward through r48-r52. The 3 red web vitest cases pre-date #2241 and are masking real regressions every time a PR touches `web/` — every PR triggers the same "3 boundary failures noted as pre-existing" PR-body footnote which is exactly the gate-mask vector evaluator round-45 P0'd for the backend (workspace-clippy / claim_4 carry).

## Scope

Identify, root-cause, and fix the 3 red vitest cases under `web/tests/` that have been flagged as pre-existing in every recent PR body. Concrete starting points:

1. **Run the suite cold**: `cd web && pnpm install && pnpm test` (or whatever the project's test command is — check `web/package.json` scripts). Capture the 3 red cases with their file paths + assertion lines.
2. **Triage**: for each, decide whether it's (a) a real bug to fix, (b) a stale fixture that should be regenerated, or (c) a test that's testing the wrong thing and should be rewritten or deleted.
3. **Ship the fix**: if (a), fix the code; if (b), regenerate the fixture; if (c), rewrite or delete the test with a comment citing this brief.

Acceptable shape variations: if 1 of the 3 is truly intractable (e.g. depends on infra not present locally), the brief allows shipping 2 of 3 fixes + leaving the third with an `it.skip(...)` + comment + ledger row to follow up. But the default is fix all 3.

Boundary-failures footnote pattern (search recent PR bodies for "3 pre-existing web vitest failures" — appears in #2138, #2142, #2241, others) is the index. The PRs themselves cite specific filenames in some cases; cross-reference.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`web/tests/`]

## Files you may touch

- `web/tests/**/*.test.ts` and `web/tests/**/*.test.tsx` — whichever 3 are red.
- `web/lib/**` or `web/app/**` — code under test, if root cause is a real bug.
- `web/tests/__fixtures__/**` — if a fixture is stale.

## Tests

- `cd web && pnpm test` exits 0 (or with the original count minus 3 failures).
- The 3 previously-red cases now PASS (or are explicitly skipped with documented reason).

## Definition of done

- `pnpm test` green for the 3 cases (status 0 in CI-emulating mode).
- PR body includes: (a) the 3 case names + their root causes, (b) a one-line summary per fix, (c) before/after counts ("was: X passing / 3 failing — now: X+3 passing / 0 failing").
- PR body explicitly retires the "3 pre-existing failures noted as boundary" footnote pattern.
- If any case is intentionally skipped, the skip is documented + a follow-up ledger row authored.

Notes: this is unblocking-quality-of-life work, not a critical path. But it eliminates a 6-week-running gate-mask vector and earns goodwill with every future web PR. Don't bundle with unrelated cleanup.
