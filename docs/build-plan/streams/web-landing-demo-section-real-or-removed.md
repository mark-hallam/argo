# web-landing-demo-section-real-or-removed

**Owner**: claude or codex — **P1 MARKETING THEATER (shakedown F1).**
**Depends on**: none.
**Authored against**: origin/main `bff053a10` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-landing-demo-section-real-or-removed

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md (operator-contracted shakedown).

## Problem
/landing#demo renders a full video player ("Watch a teammate get hired") whose
play button does NOTHING. Confirmed dead in the shakedown. The demo MP4 is an
operator-only artifact that does not exist yet.

## Fix
Until the MP4 exists: point the play button at the LIVE demo workspace journey
(`/auth/signup` demo CTA) with copy "Try it live — 5 minutes, no email", OR
collapse the section. Keep the section component so dropping in the real video
later is a one-line change. NO fake video, NO autoplaying mock.

## Verify
vitest: clicking the play CTA navigates (no dead handler); section renders
the live-demo variant when no video URL is configured.
