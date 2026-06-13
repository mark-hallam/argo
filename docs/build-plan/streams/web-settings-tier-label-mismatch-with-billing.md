# web-settings-tier-label-mismatch-with-billing

**Owner**: claude or codex — **P3 UX-CONSISTENCY (soak finding).**
**Depends on**: none.
**Authored against**: origin/main `6524852d4` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-settings-tier-label-mismatch-with-billing

Source: docs/skycot/audit/2026-06-13-e2e-soak-15x.md FINAL SCORECARD.

## Problem
The Settings tab shows tier "Pro" while the canonical /billing page shows
"Free" for the same demo tenant (soak final scorecard, 2026-06-13). Two
surfaces read the tier from different sources / a hardcoded default.

## Fix
Make Settings read the tier from the same canonical source /billing uses
(tenant tier from tenant_quota / tier resolver). Grep the Settings tier label
source; remove any hardcoded "Pro". One source of truth for the displayed tier.

## Verify
vitest: Settings + billing render the SAME tier for a given tenant fixture
(free→Free both places). Branch + PR.
