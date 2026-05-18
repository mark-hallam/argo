# Audit page — design exploration (2026-05-18)

Following the optometrist method: 5 wildly different design languages for the same page (the in-product audit / activity log).

All five use the **locked brand identity** from the marketing landing pick:
- Audiowide for display headings + section labels + page titles
- Inter for body / UI
- Brand palette: purple `#8B5CF6`, pink `#EC4899`, coral `#FF6B6B`, yellow `#FCD34D`, teal `#14B8A6`
- Worker initials avatars share the rounded-square + Audiowide letterspacing treatment from the marketing hero

What differs is the **metaphor** — how the page interprets the question "what's an audit log?"

## The five lenses

| File | Metaphor | Voice | Best for | Trade-off |
|---|---|---|---|---|
| [option-A-editorial-feed.html](./option-A-editorial-feed.html) | Editorial feed / activity newspaper | Conversational, story-style | Trust-building with non-technical buyers | Less efficient at scale |
| [option-B-forensic-chain.html](./option-B-forensic-chain.html) | Forensic / blockchain-explorer | Cryptographic, hash-visible | Compliance buyers, SOC 2 conversations | Looks "scary" to non-technical |
| [option-C-data-table.html](./option-C-data-table.html) | Database / Linear-issue table | Power-user, keyboard-driven | Ops engineers using it daily | Less screenshot-worthy |
| [option-D-card-stream.html](./option-D-card-stream.html) | Slack feed / Twitter stream | "Your team is talking" | Daily-glance + inline approve/reject | Less efficient at scale |
| [option-E-timeline-flow.html](./option-E-timeline-flow.html) | Vertical timeline with gradient spine | "The day in flow" | Demo recording screenshot · Product Hunt | Less efficient than the table |

## Shared content

All five render the same 10 seeded audit rows so you can compare apples-to-apples:

- Jamal resolved SUP-29382 (KB article SK-413) — succeeded — $0.004
- Sara extracted INV-2024-08741 line items — succeeded — $0.012
- Priya screened Diego Alvarez vs JD-2024-08 (score 8.2) — succeeded — $0.014
- Nori validated QT-2024-7283 against pricing rules — succeeded — $0.008
- Mariah drafted RFP for Tomás Industries (1240 words) — succeeded — $0.021
- Nori flagged 32% discount on QT-2024-7283 — **pending you**
- Sara refused duplicate invoice INV-2024-08739 — refused
- Priya drafted rejection email to Sara Lin — **pending you** — $0.009
- Mariah scheduled Q3 renewal review with Acme Tech — succeeded
- Jamal escalated SUP-29355 to @lead-support — succeeded

Plus the same KPIs: 17 actions today, 7 pending you, $0.094 spend, 100% chain verified.

## How to pick

Open all 5. Five-second gut on each.

```bash
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/audit-page/*.html
```

Then tell me one of:

1. **"B — forensic"** → I narrow with 2-3 variations of B (more / less mono, more / less density)
2. **"E — timeline"** → I narrow on E (different spine treatment, hour density, more / fewer event details)
3. **"D's cards with B's hash visibility"** → I'll merge
4. **"None — try one more variant: a hybrid of E's timeline + A's editorial text style"** → another round

The audit page is the most likely **demo screenshot moment**. If virality matters most, E is engineered for that. If trust matters most for compliance buyers, B. If daily ops usage matters most, C. If brand voice / approachability matters, D. If category-defining feel matters, A.
