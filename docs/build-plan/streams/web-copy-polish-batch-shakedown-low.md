# web-copy-polish-batch-shakedown-low

**Owner**: claude or codex — **P3 POLISH BATCH (shakedown F7+F8+F12+F13 + changelog).**
**Depends on**: none.
**Authored against**: origin/main `bff053a10` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-copy-polish-batch-shakedown-low

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md (operator-contracted shakedown).

## Problem / Fix (batch)
1. /team hero "0 working · 22 actions this week" contradicts "1 teammate
   working" below; count teammate actions not all audit rows, one "working"
   definition (F12).
2. Fresh-hire cosmetics: generated avatar tile (initials+gradient is fine,
   bare letter is not) + role-title subtitle instead of lowercase slug (F13).
3. /about mentions "receptionists" — deleted vertical; swap for a live
   roster example (F7-adjacent).
4. Docs/changelog hero: drop plain-English expansions of API/RSS on
   developer-facing pages (F8).
5. /terms: status.skycot.com referenced before it exists — soften until the
   Week-2 status page lands (F7).
6. /changelog says "UPDATED WEEKLY" but latest entry is 2026-05-20 — add the
   current release wave or drop the cadence claim.

## Verify
vitest copy pins for each; visual spot-check screenshots in PR body.
