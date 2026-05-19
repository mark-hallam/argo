# Workers page — design exploration (2026-05-18)

> **⚠ 2026-05-19 update**: The locked pick (`option-C2-holographic-showcase.html`) received a brand-consistency pass alongside all other in-product pages. The "Inter only, no Audiowide" claim below is **stale**. See `DESIGN_SESSION_RESUME.md` and `project_brand_consistency_pass.md` for the current brand recipe (Audiowide hero + purple→pink gradient + warm radial blobs applied uniformly).

Five wildly varied design languages for the in-product "Workers" page (your roster of AI teammates: Jamal, Sara, Priya, Nori, Mariah).

All five use the **locked enterprise identity** from the audit-page pick (description below reflects pre-2026-05-19 state):
- Inter typography only (no Audiowide — that's reserved for marketing/brand moments)
- Inter weight 300 for editorial display numbers, 600 for emphasis, 400 body
- Brand gradient `#8B5CF6 → #0891B2` used sparingly
- JetBrains Mono for monospace data

What differs is the **metaphor** — how the page interprets "what is the workers page?"

## The five lenses

| File | Metaphor | Voice | Best for | Trade-off |
|---|---|---|---|---|
| [option-A-team-roster.html](./option-A-team-roster.html) | Team / HR roster | "Your team" — conversational, humanizing | First emotional connection — your AI teammates feel like people | Less efficient at scale (one card per row) |
| [option-B-fleet-ops.html](./option-B-fleet-ops.html) | Fleet operations dashboard | SRE / Datadog "Hosts" view | Operations engineers, daily power-use, status-at-a-glance | Less emotionally warm |
| [option-C-trading-cards.html](./option-C-trading-cards.html) | Trading-card deck | "Your roster — collect them all" | Viral / shareable / product-marketing screenshots | Risks reading as gimmicky for serious enterprise buyers |
| [option-D-org-network.html](./option-D-org-network.html) | Org chart / force-directed network graph | Structural — see relationships | Understanding team shape, skill overlap, collaboration | Less efficient for daily ops |
| [option-E-slots-marketplace.html](./option-E-slots-marketplace.html) | Filled slots + open slots + marketplace suggestions | Growth-oriented — "you have room to hire" | Activation, expansion-revenue path, marketplace cross-sell | Mixes UI + commercial cross-sell on the same page |

## Shared content

All five render the same 5 seeded workers so you can compare apples-to-apples:

- Jamal — Support · L2 · active · just resolved SUP-29382
- Sara — Invoice ops · L3 · active · extracted INV-2024-08741
- Priya — Recruiting · L2 · active · drafted rejection email awaiting approval
- Nori — Quote ops · L3 · active · flagged 32% discount waiting on operator
- Mariah — Sales ops · L2 · in training (62%) · drafted RFP

KPI shape: 5 workers total · 4 active · 1 training · 17 actions today · 7 pending you · $0.094 spend.

## How to pick

```bash
open ~/code/argo/docs/skycot/design-explorations/2026-05-18/workers-page/*.html
```

Five-second gut on each, then say one of:

1. "A — team roster" → I narrow with 2-3 variations (more / less density, with / without "currently doing" panel, different stat layouts)
2. "B — fleet ops" → I narrow (more / less mono, with / without sparklines, status as left-bar vs pill)
3. "C — trading cards" → I narrow (more / less gradient, holographic effects, different card aspect ratio)
4. "D — org network" → I narrow (different layout algorithm, with / without sidebar, force-directed vs hierarchical)
5. "E — slots + marketplace" → I narrow (more / less marketplace, suggestions inline vs separate section)
6. Combos / hybrids → e.g. "A's warmth with E's slot meter"
7. "None — try X" → another round

## Which is which for which buyer

- **Daily power-user (operator running ops daily):** B — fleet table is most efficient
- **Buyer / first-impression / demo screenshot:** C trading cards is the most viral; A roster is the most humanizing
- **Enterprise compliance + structural buyer:** D org network shows "this is a team, not isolated bots"
- **Growth / activation / expansion revenue:** E slots + marketplace is built for this
