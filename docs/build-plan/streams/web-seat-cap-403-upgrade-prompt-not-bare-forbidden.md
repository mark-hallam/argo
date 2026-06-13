# web-seat-cap-403-upgrade-prompt-not-bare-forbidden

**Owner**: claude or codex — **P2 CONVERSION (soak finding, 2026-06-13).**
**Depends on**: none.
**Authored against**: origin/main `a9ee7745c` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-seat-cap-403-upgrade-prompt-not-bare-forbidden

Source: docs/skycot/audit/2026-06-13-e2e-soak-15x.md (accumulation run) — a
hire that hits the Free-tier `max_workers=1` seat cap returns 403 and the SPA
shows only a bare red "Forbidden" banner.

## Problem
Hiring a 2nd teammate on the Free tier is correctly blocked by the seat cap
(`tenant_quota` max_workers=1, enforced in argo-store) → `POST /api/v1/workers`
403. But the UI surfaces a bare "Forbidden" — a prime upgrade moment rendered
as a scary auth error. The user can't tell they hit a PLAN LIMIT vs a bug.

## Fix
When the hire POST returns 403 specifically because of the worker seat cap,
the SPA shows a friendly, on-brand prompt: "You've reached your Free plan's
1-teammate limit — upgrade to Pro to hire up to 5" with a link to /billing
(or /pricing). Distinguish the seat-cap 403 from a generic auth 403: have the
gateway return a structured error code (e.g. `{error:{code:"seat_cap",
limit:1, tier:"free", upgrade_to:"pro"}}`) and branch the SPA on it. Do NOT
weaken the cap — only the messaging.

## Verify
vitest: a seat_cap 403 renders the upgrade prompt (not "Forbidden"); a generic
403 still renders the auth error. Route/gateway test: the hire seat-cap path
returns the structured code. Branch + PR.
