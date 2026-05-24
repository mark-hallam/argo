# marketing-replace-fabricated-social-proof

**Owner**: claude.
**Status note**: 🟡 **PRE-GO-LIVE — do not claim yet.** Operator deferred 2026-05-24 during the demo SWOT audit. To be reclaimed before any public-launch traffic.
**Authored against**: origin/main `a2a8405f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh marketing-replace-fabricated-social-proof`.

🔴 **P0 legal-risk PRE-GO-LIVE** — fabricated testimonials + made-up customer stats on the marketing site (and one onboarding page) are deceptive-conduct territory under Australian Consumer Law (ACCC) and US FTC guidance. Must be replaced before any public-launch traffic, ads, or press outreach.

## Why this brief exists

The 2026-05-24 demo SWOT audit observed every fabricated trust-signal on the public marketing site (`marketing/`) and on `/onboarding/welcome`. The current copy uses invented customer names, invented case-study percentages, and invented dollar-savings claims — all presented as if they were real.

## Inventory of fabrications to replace

### Marketing landing (`marketing/`)

| Surface | Current copy | Type |
|---|---|---|
| Top badge | "v1.0 shipped · Hiring our first 100 teams" | Half-true — v1.0 isn't actually shipped yet (no demo MP4) |
| TRUSTED BY strip | "100+ OPS, HR & REVENUE TEAMS" + 6 logos (Nimbus.io, Hartwell & Co., aether/labs, FOUNDRY YARD, Cohort, Mercer Health) | Fabricated — none are real customers |
| Customer testimonial | "It felt like hiring an associate, except they finished their first task in the time it would have taken me to draft the brief. Six months in — half our outbound runs through SKYCOT workers, and our auditor can replay every action." — Riley Kovacs, VP Customer Success · Nimbus.io | Fabricated quote + fabricated person + fabricated company |
| Stats block | 53% Outbound delegated within 90 days · $8.2k Per-month per-worker savings · 4.2x Approvals processed per operator hour · 100% Audit chain coverage | First three are fabricated — last one is real (audit chain coverage IS 100% by construction) |

### Onboarding welcome (`web/app/onboarding/welcome/`)

| Surface | Current copy | Type |
|---|---|---|
| Footer trust strip | "2,847 COMPANIES THIS MONTH" | Fabricated metric |

## Operator decision needed before authoring the replacement copy

Three replacement strategies — operator picks one (or instructs hybrid):

**Option A — Honest founder voice.** Replace fabricated customers with the operator's own attribution. Example: "Built in Melbourne by Mark Hallam after 12 years of [context]." Replace the testimonial slot with a founder-letter pull-quote. Remove the stats block entirely.
  - Pros: Honest, no legal exposure, distinctive vs the SaaS-template look
  - Cons: Less "credibility" signal than logo-stripped social proof

**Option B — Anonymised real pilots.** Wait until 3-5 real pilot customers are onboarded, then replace with anonymised real quotes ("VP Operations at a 200-person SaaS scaling outbound" — quote real). Remove logos entirely or use category icons ("HR · Ops · Sales").
  - Pros: Real signal once we have it; defensible
  - Cons: Delays go-live until pilots ship

**Option C — Aspirational with disclaimer.** Keep the stats as "model expectations based on internal benchmarks" with a footnote. Remove logos and testimonial.
  - Pros: Lets us launch faster than B
  - Cons: Footnotes hurt conversion; "expected" disclaimers are weak signal

## Scope (deferred — author replacement plan after operator decision)

This brief intentionally STOPS at the inventory + options. The replacement-copy plan is a separate brief authored after the operator picks A / B / C.

## Files this brief will eventually touch

- `marketing/components/Hero.tsx` (or equivalent hero component)
- `marketing/components/TrustedBy.tsx` / `Logos.tsx`
- `marketing/components/Testimonial.tsx`
- `marketing/components/Stats.tsx`
- `web/app/onboarding/welcome/page.tsx`
- `marketing/lib/copy.ts` if marketing copy is centralised

A `grep` pass before the implementation brief will lock the exact file list.

## Acceptance criteria (placeholder — will fill in after decision)

- [ ] Zero fabricated company names visible in `marketing/`
- [ ] Zero fabricated testimonials visible in `marketing/`
- [ ] Zero unverifiable claim-numbers visible in `marketing/` and `web/app/onboarding/`
- [ ] "v1.0 shipped" badge accurate per `docs/skycot/demos/v1-mission-mvp.mp4` existence
- [ ] Legal-review checklist in PR body confirming ACL/FTC defensibility per chosen option

## Verification

`rg -i 'nimbus|hartwell|aether|foundry yard|mercer health|riley kovacs|2,847 companies|\$8\.2k|53% outbound|4\.2x approvals' marketing/ web/app/` returns ZERO matches after the replacement lands.

## Cross-references

- 2026-05-24 demo SWOT findings — conversation log, claude session post-PR `a2a8405f4`
- ACCC misleading-conduct guidance: `https://www.accc.gov.au/business/advertising-and-promoting-your-business/false-or-misleading-claims`
- FTC endorsement guides: `https://www.ftc.gov/business-guidance/resources/ftcs-endorsement-guides-what-people-are-asking`
