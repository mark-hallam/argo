# web-compare-3-new-rows — add 3 defensible competitor-parity rows to /compare

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-compare-3-new-rows`.
**Priority**: P2 — wave-3 competitor-parity audit identified 3 high-defensibility rows we don't currently claim. They'd strengthen the page from 8/10 to 9+/10.
**Sizing**: ~80-150 LOC (3 row inserts + competitor evidence + tests).

## Why this brief exists

From the wave-3 competitor-parity audit (`docs/skycot/audit/competitor/2026-05-20-wave-3-competitor-parity.md`):

> Top 3 to ADD:
> 1. "Re-verify every action months later?" — 10/10 defensibility.
>    `argo audit verify` walks the hash chain and reproves rows.
>    Nobody else ships this.
> 2. "Cross-app ⌘K palette" — 9/10. We just shipped it
>    (commit c11a0d57).
> 3. "BYOK via AWS/GCP/Azure KMS" — 9/10. Build-flavor-gated today,
>    architectural foundation exists.

## Goals

1. Add 3 new rows to the /compare 13-row matrix (now becomes 16-row).
2. SKYCOT: ✓ on all three.
3. Competitors (Zapier, n8n, Crew.ai, Lindy): defensible "no" / "?" / "no" / "?" per public evidence.
4. Update the page headline: was "8 of 13 clean wins" → "11 of 16 clean wins" (or whatever the new math is — re-tally honestly).

## Steps

1. Read `web/app/compare/page.tsx` for the matrix shape.
2. Add 3 entries to the rows array.
3. Re-tally the "clean wins" claim in the headline.
4. Verify competitor claims with quick due-diligence (their docs / pricing pages — don't over-claim).
5. Visual sanity: row alignment, gradient, mobile scroll wrap still works.

## may-touch

- `web/app/compare/page.tsx` (matrix + headline + summary)
- Tests if any exist

## must-NOT-touch

- Existing 13 rows (only additive)
- Mobile-scroll table wrap (already shipped wave-2)

## Acceptance criteria

- [ ] /compare matrix has 16 rows.
- [ ] Each new row has a clear question + SKYCOT answer + 4 competitor answers.
- [ ] Headline re-tallied honestly.
- [ ] Visual sanity at 375 / 768 / 1024 / 1440 px.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/competitor/2026-05-20-wave-3-competitor-parity.md`
- Commit `c11a0d57` — ⌘K palette shipped
