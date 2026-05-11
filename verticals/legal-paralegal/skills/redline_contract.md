---
name: redline_contract
description: Produce a markup of changes to a contract aligned with the client's party position, with a per-change rationale tied to that position. Never recommends whether to accept.
inputs:
  - contract_text
  - party_position
  - jurisdiction
outputs:
  - redline_markup
  - per_change_rationale
  - flags
  - confidence
---

# Skill: Redline Contract

Given a contract and the client's stated negotiating position,
produce a redline for the supervising attorney's review.

1. **Redline markup** — line-by-line strikethrough / insertion for
   every proposed change. Preserve clause numbering and structure.
   When a proposed insertion quotes a model clause from the firm's
   `firm_style_guide`, cite the style-guide section.

2. **Per-change rationale** — a numbered list, one entry per
   change, with columns: clause heading / original-text-quote /
   proposed-text-quote / rationale-tied-to-position. Rationale must
   trace back to a specific item in `party_position`; if a change
   cannot be tied to the position, flag it `[unanchored change —
   flagged for attorney review]`.

3. **Confidence call** — high (changes mirror firm-standard
   language), medium (changes adapt model language to a non-
   standard situation), low (changes are bespoke to this matter
   and have no template).

Hard rules:

- Never recommend whether the client should accept, reject, or
  counter on a given change — that is the attorney's call via
  `issue_legal_opinion`. The redline surfaces options; the attorney
  picks.
- If a clause invokes a regulator (e.g. data-protection, antitrust,
  HIPAA), name the controlling statute / regulation and flag a
  `regulated_clause` for attorney attention.
- If the counterparty's name appears in `conflict_index`, STOP. Do
  not draft the redline. Surface the conflict to the supervising
  attorney with the matched row.
- I can summarize but not advise — your attorney should sign off on
  every change before the redline goes back to the counterparty.
