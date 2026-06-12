# pricing-copy-alignment-option-d

**Owner**: claude or codex — web-only (DEC-006), LOCKED BRAND.
**Depends on**: none (copy can land ahead of mechanics; honest "coming for
public launch" phrasing where a mechanic is not yet live).
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh pricing-copy-alignment-option-d

## Problem (DEC-053)

The pricing page + checkout copy must state Option D truthfully: Free =
100 actions/mo; Pro = $49 per named teammate incl. an action allowance,
then metered overage at a stated rate, with a self-set hard cap and
"pause a teammate, stop its billing"; Enterprise = bring-your-own keys +
flat platform fee + SSO/SLA. Current copy promises per-teammate billing
the code only now grows, and says nothing about allowance/overage/caps.

## Fix

Derive all numbers from `v1-tiers.toml` (grep how /pricing reads tiers
today — single source, DEC-035; extend the toml→props plumbing if a field
is missing, no hardcoded numbers in JSX). State the cap behavior honestly
("usage stops at your cap — no surprise bills"). FAQ entries: overage
math example, pause semantics, BYOK what-it-means. Keep locked brand;
update the marketing-cta-vocab + pricing pin tests rather than weakening
them (extend assertions to the new copy).

## Acceptance
1. /pricing renders Option D from v1-tiers.toml values; zero hardcoded
   prices in JSX (grep pin). 2. Full vitest stays 0-failed (update pins
   honestly); tsc clean; /pricing 200.

~60-120 LOC. **🟠 P2 — the promise and the code say the same thing;
public-flip blocker.**
