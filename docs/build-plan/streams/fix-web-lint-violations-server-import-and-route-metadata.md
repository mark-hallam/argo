# fix-web-lint-violations-server-import-and-route-metadata

**Owner**: claude or codex — web-only, DEC-006 fast lane.
**Depends on**: none.
**Authored against**: origin/main `1735b11c3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-web-lint-violations-server-import-and-route-metadata

## Problem (verified by conductor's own full vitest run, 2026-06-12)

Two REAL lint violations in production web source, each pinned by an
existing failing lint-test (surfaced once #2841 cleared the harness
noise):

1. `web/__tests__/lint/no-server-imports.test.ts` (2 tests) — exactly one
   client file statically imports a server-only module ("expected 1 to
   be 0"). Run the lint script the test wraps to identify the file.
2. `web/__tests__/lint/route-metadata.test.ts` (2 tests) — exactly one
   `page.tsx` has no inline or inherited route metadata.

## Fix

Fix the PRODUCT source, not the tests: move/indirect the server-only
import behind the proper seam (mirror how sibling client files consume the
same data); add the missing route `metadata` export (match the metadata
shape of sibling pages in the same route group). Do not weaken either
lint test.

## May touch

- the one offending client file + the one offending `page.tsx` (identify
  via the lint scripts; name them in the PR body)
- NO test changes except if a lint allowlist entry is the genuinely
  documented pattern (justify in the PR body if so)

## Acceptance / verifier

1. `npx vitest run __tests__/lint/no-server-imports.test.ts __tests__/lint/route-metadata.test.ts` — 4/4 green.
2. `tsc --noEmit` clean; touched routes still render (curl 200 on dev).

~10–40 LOC. **🟡 P3 — real product-source lint debt with existing
verifiers.**
