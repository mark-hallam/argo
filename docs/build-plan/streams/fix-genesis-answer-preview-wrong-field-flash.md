# fix-genesis-answer-preview-wrong-field-flash

**Owner**: claude or codex — **P3 COSMETIC (re-test N4).**
**Depends on**: none.
**Authored against**: origin/main `2319a7a69` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-genesis-answer-preview-wrong-field-flash

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md ("Re-test 2026-06-13" section — operator-contracted shakedown re-test).

## Problem
In the Genesis "talk it through" captured panel, a carve-out answer ("Never
threaten legal action") briefly rendered inside the "When does this teammate
run?" (cadence) input before the correct cadence answer overwrote it.
Self-corrects; cosmetic only.

## Fix
Grep the Genesis captured-panel field-mapping (the live "What we've captured"
component). The transient mis-bind is likely a stale-answer flash before the
field router assigns the latest turn. Gate each field on its resolved slot so
an answer never paints in the wrong input even for a frame.

## Verify
vitest: feeding answers in sequence never places answer N in field N-1's slot;
visual spot-check.
