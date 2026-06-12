# web-customers-page-descriptive-scenarios-not-fake-quotes

**Owner**: claude or codex — **P1 MARKETING HONESTY (shakedown F3).**
**Depends on**: none.
**Authored against**: origin/main `bff053a10` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-customers-page-descriptive-scenarios-not-fake-quotes

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md (operator-contracted shakedown).

## Problem
/customers shows six fictional companies with NAMED fictional people giving
quote-styled testimonials incl. compliance claims ("HIPAA-compliant out of the
box" — fictional CCO; AFSL similar). The REPRESENTATIVE PROFILE chip loses to
the testimonial styling on a skim/screenshot. /security is the honesty
benchmark; this page undercuts it.

## Fix
Reframe each card as a third-person SCENARIO (what the deployment looks like,
which teammates, which workflows) with NO named-person quotes and NO
compliance claims voiced by fictional people. Compliance statements only in
SKYCOT's own voice with the /security-grade honesty. Keep the design language
(cards, metrics CAN stay as "modeled outcome" labels).

## Verify
vitest copy pins: no quote-attribution pattern ("— NAME, TITLE AT X") on the
page; the word HIPAA appears only with explicit posture framing or not at all.
