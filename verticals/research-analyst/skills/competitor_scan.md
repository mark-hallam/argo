---
name: competitor_scan
description: Given a target company, produce a 5-competitor matrix comparing pricing, segments, strengths, and weaknesses, with citations per cell.
inputs:
  - target_company
  - scope_geography
  - scope_segment
outputs:
  - competitor_matrix
  - methodology_note
  - sources_consulted
  - confidence
---

# Skill: Competitor Scan

Build a fixed-column 5-competitor matrix relative to the named target.

1. **Selection** — pick the 5 closest competitors by segment overlap
   and geography. Document the selection rule in `methodology_note`.
   Anonymize private competitors if the only available pricing is
   tier-3 ("Competitor C, private SaaS in the same segment").
2. **Columns (fixed)** — `pricing_model`, `target_segment`,
   `core_strengths` (≤3 bullets), `core_weaknesses` (≤3 bullets),
   `recent_signal` (last 12-month material event — funding, hire,
   product, regulatory). Each cell carries an inline citation or the
   `[unsourced — flagged for analyst review]` marker.
3. **Pricing** — public list prices only. If pricing is gated behind
   sales, write "contact-sales" — never quote anecdotal numbers from
   forums or Reddit.
4. **Strengths / weaknesses** — sourced from primary filings,
   customer reviews on Tier-2 platforms (G2, Gartner Peer Insights)
   with the methodology disclosed, or named analyst-firm reports. Tier-3
   blog posts may surface a hypothesis but cannot be the sole support.
5. **Methodology note** — declare the selection rule, the geography,
   the segment cut, and the date range of evidence consulted.
6. **Sources consulted** + **confidence** as per the system prompt.

If any competitor cell would require citing leaked pricing,
employee-only docs, or paywall-scraped material, leave the cell empty
with `[unsourced — paywall]` or `[unsourced — non-public]` and
escalate. Never fabricate a number to fill the grid.
