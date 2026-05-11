---
name: cite_check
description: Validate every citation in a draft document for Bluebook form and produce a signal-strength tag (good-law / criticized / overruled / not-found) per citation.
inputs:
  - draft_document
  - jurisdiction
outputs:
  - per_citation_table
  - shepardizing_date
  - flags
  - confidence
---

# Skill: Cite Check

Walk every legal citation in the input draft and produce a
per-citation table for the supervising attorney's review. Three
columns per row:

1. **Parsed citation** — the citation as it appears in the draft.
2. **Bluebook form** — the canonical Bluebook citation, corrected
   if the draft is non-conforming. Note any departures (improper
   abbreviation, missing pin cite, wrong reporter, parallel-cite
   omission). Use the firm's `firm_style_guide` for any local
   variants the firm prefers over default Bluebook.
3. **Signal-strength tag** — one of:
   - `good-law` — current, controlling, not negatively treated.
   - `criticized` — treated negatively by at least one persuasive
     authority, but not overruled in the relevant jurisdiction.
   - `overruled` — overruled or abrogated in the relevant
     jurisdiction; cannot stand as authority for the proposition.
   - `not-found` — citation does not resolve in the searched
     reporters / databases; flag for attorney follow-up.

Record the `shepardizing_date` (today's date in ISO format) so the
attorney knows when the check ran; signal strength may shift after
the check.

Update the `cited_authorities` memory entry with each validated
citation, its tag, and the shepardizing date so future memos can
re-use the validation within the firm's freshness window.

Hard rules:

- Never silently downgrade an `overruled` citation to `good-law`;
  the tag must surface in the table.
- If the input draft cites a tier-3 source (blog post, AI summary,
  unnamed aggregator) as authority for a load-bearing proposition,
  add a `flags: ["tier3_as_authority"]` entry and route to the
  attorney.
- I can summarize but not advise — your attorney should sign off on
  the corrected citations before they enter a filing.
