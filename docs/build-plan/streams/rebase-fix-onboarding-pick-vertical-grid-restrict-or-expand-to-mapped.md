# rebase-fix-onboarding-pick-vertical-grid-restrict-or-expand-to-mapped

Authored against: origin/main `0c7cff50f5` on 2026-05-29
Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-onboarding-pick-vertical-grid-restrict-or-expand-to-mapped

## Why this brief exists

PR #2475 (140 LOC, 2026-05-27) targeted F10 [P2]: `/onboarding/pick-vertical` renders 7 vertical cards in demo mode (Marketplace Seller, Recruiter, Customer Support, Marketing Intern, GTM Engineer, Sales SDR, Accounts Receivable) but the demo-mode first-hire short-circuit only maps 4 to sample workers — picking Sales SDR fall-throughs to `DEMO_FALLBACK_WORKER = Mariah`, which renders Mariah's marketplace-lister charter on `/workers/<id>` (vocabulary mismatch the demo camera could land on).

Bug is STILL load-bearing:
- `VERTICAL_TO_SAMPLE_WORKER` at `web/lib/onboarding/use-first-hire.ts:121` still maps only 4 verticals (marketplace-seller, recruiter, customer-support, marketing-intern).
- `DEMO_FALLBACK_WORKER` at line 128 still defaults to Mariah.

The original PR's approach (Path a) — filter the rendered grid to the 4 cleanly-mapped verticals + add an honest "Demo · 4 verticals available" note — STILL applies.

But the file shape has drifted:
- **#2491** replaced per-card demo chip with one banner on pick-vertical.
- **#2504** added the wizard polish cluster (wrap + scroll + banner + eyebrow).
- **#2524** expanded install-vertical receipt with memory + gates.
- **#2551** deduped per-step demo banners against global shell banner.

`web/app/(setup)/onboarding/pick-vertical/__tests__/demo-mode-relabel.test.tsx` evolved in #2491 + #2504.

The cleaner long-term fix is **Path (b)** — expand `VERTICAL_TO_SAMPLE_WORKER` to all 7 verticals (sales-sdr → Sasha, finance-ar → Aria, gtm-engineer → Gemma). But that requires creating 3 new sample personas + adding them to `web/lib/sample-team.ts`, which is bigger surface.

This rebase brief gives the implementer the choice:
- **Path (a) — quick fix** (matches PR #2475): filter demo grid to 4 mapped verticals.
- **Path (b) — proper fix**: expand the map to 7 verticals and add the missing sample personas.

PR #2475 was closed for re-author.

## Source of truth

- Original brief: `docs/build-plan/streams/fix-onboarding-pick-vertical-grid-restrict-or-expand-to-mapped.md`
- Closed PR: #2475
- Current main shape:
  - `web/lib/onboarding/use-first-hire.ts:121-128` (4-of-7 mapping still active).
  - `web/app/(setup)/onboarding/pick-vertical/page.tsx` evolved through #2491, #2504, #2524, #2551.

## Acceptance

Same as original brief (Path a) OR Path b. Reject any path where picking "Sales SDR" / "GTM Engineer" / "Accounts Receivable" lands on Mariah's persona card.

## Boundary

- Same files as original brief.
- Coordinate with `rebase-fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker` — if that brief lands first and removes the `VERTICAL_TO_SAMPLE_WORKER` short-circuit entirely (replacing it with description-driven roster growth), this brief becomes moot. Re-evaluate before claim.

## Verification

- `npx vitest run lib/onboarding/__tests__/use-first-hire.test.ts 'app/(setup)/onboarding/pick-vertical/__tests__/demo-mode-relabel.test.tsx' tests/onboarding-pick-vertical.test.ts`
- `npx tsc --noEmit`
- `npx next lint`
