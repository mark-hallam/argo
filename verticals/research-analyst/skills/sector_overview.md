---
name: sector_overview
description: Given a sector and scope, produce a 1-page brief with market size, top-5 players, and 3 trends, every claim inline-cited.
inputs:
  - sector
  - scope_geography
  - scope_timeframe
outputs:
  - executive_summary
  - market_or_topic_overview
  - top_players
  - trends
  - sources_consulted
  - confidence
---

# Skill: Sector Overview

Given a sector and the analyst's stated scope, produce a 1-pager that
the operator can hand to a generalist reader in under 5 minutes.

1. **Executive summary** — 3 bullets. Each bullet ends with an inline
   citation `[source: <url>, tier: <1|2|3>, date_accessed: <ISO>]`.
2. **Market size** — the most recent primary or tier-1 sizing figure
   in the requested geography and timeframe. Include the currency, the
   year, and the methodology if disclosed. If no primary figure is
   available, say so and use the best tier-2 with caveat.
3. **Top-5 players** — by revenue or by share, whichever is sourced.
   Anonymize private companies if a primary figure does not exist
   (e.g. "a top-quartile vendor in the niche") rather than guessing.
   For each row: name, segment, sourced revenue / share, evidence link.
4. **3 trends** — pick three that have ≥2 independent sources. Each
   trend names the driver, the time horizon, and the contrarian view.
5. **Sources consulted** — flat list with tier rating.
6. **Confidence** — high / medium / low / speculative, calibrated to
   the source mix.

If the sector touches a regulated industry and you cannot locate a
primary regulator filing, set `flags: ["regulated_without_primary"]`
and escalate. Do not paraphrase tier-2 trade press as primary.
