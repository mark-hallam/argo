# fix-landing-trust-strip-mixed-self-serve-vs-early-access-signals

**Owner**: claude.
**Authored against**: origin/main `74077bf676` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-landing-trust-strip-mixed-self-serve-vs-early-access-signals`.
**Source**: ux-audit-2026-05-26-2214 `onboarding-hire-an-ai-teammate.md` F9 [P3].

🟢 **P3 — `/landing` mixes open self-serve signals ("NO CREDIT CARD" chip, "Hire your first teammate" primary CTA) with closed waitlist signals ("EARLY ACCESS · APPLY" chip, "V1.0 SHIPPED · HIRING OUR FIRST 100 TEAMS" hero badge). Pick a story and commit to it across the strip + CTA. Marketing-coherence polish, not a demo blocker — demo doesn't follow the EARLY ACCESS chip.**

## Root cause

`web/app/landing/page.tsx:155-172` renders five trust chips in the hero trust strip:

```
NO CREDIT CARD · SOC 2-READY AUDIT TRAIL · ISO 27001 CONTROLS SUPPORTED · GDPR · HIPAA WORKFLOWS · EARLY ACCESS · APPLY
```

The hero badge at lines 74-77 also reads "V1.0 SHIPPED · HIRING OUR FIRST 100 TEAMS" (early-access framing), but the primary CTA right below is "Hire your first teammate" (open self-serve framing). The two halves contradict.

Operator-decisional question: is `/landing` open self-serve or curated waitlist? The CTA, the chips, and the badge should all say the same thing.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P3] — Landing trust strip mixes "NO CREDIT CARD" with "EARLY ACCESS · APPLY"

## Fix recipe

Operator picks ONE of three positionings; ship the matching change:

**Option A — Open self-serve.** This matches the current CTA + onboarding demo-mint flow.

- Drop "EARLY ACCESS · APPLY" chip from the trust strip (line 155-172).
- Replace hero badge "HIRING OUR FIRST 100 TEAMS" with something more open: "V1.0 SHIPPED · LIVE DEMO BELOW" or just delete the badge.
- Keep "NO CREDIT CARD" + the 3 compliance chips.

**Option B — Curated early-access (waitlist).** This matches the badge.

- Change primary CTA from "Hire your first teammate" → "Apply for early access" (links to a waitlist form, not `/auth/signup`).
- Drop "NO CREDIT CARD" chip (no-CC implies self-serve).
- Keep "EARLY ACCESS · APPLY" + the 3 compliance chips.

**Option C — Hybrid (recommended for the POC demo today).** Self-serve demo workspace + waitlist for real tenant.

- Primary CTA stays "Hire your first teammate" → `/auth/signup` (demo path).
- Secondary CTA "Apply for production access" → waitlist form.
- Drop "EARLY ACCESS · APPLY" chip; replace with "DEMO READY · WAITLIST FOR PROD".
- Keep "NO CREDIT CARD" (true for demo).

Each option is ~15-30 LOC. Option C carries the most copy.

## Acceptance criteria

Pick one positioning before writing the PR:

- [ ] Operator selects A / B / C (note their choice in the PR description)
- [ ] Trust strip chips are internally consistent with the primary CTA and the hero badge
- [ ] No chip / CTA / badge contradicts another
- [ ] `curl -s http://localhost:3000/landing` → no "NO CREDIT CARD" + "EARLY ACCESS · APPLY" co-occurrence
- [ ] vitest spec assertion (optional): a snapshot test that captures the trust-strip chips so future drift is caught

## May-touch

- `web/app/landing/page.tsx:74-77` (hero badge) + `:155-172` (trust strip) — ~15-30 LOC depending on option

## Out of scope

- The compliance chips themselves (those are real claims and stay)
- The pricing page positioning (different surface)
- The `/customers` testimonials (separate concern; not a public-release demo)

## Effort

~30 LOC. ~30 min once the operator picks a positioning. P3 because the demo runbook doesn't follow the EARLY ACCESS chip — only matters for marketing coherence.
