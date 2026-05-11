---
name: summarize_deposition
description: Produce a chronological summary of a deposition transcript with an objection log and impeachment-points list, citing transcript pages throughout.
inputs:
  - transcript_range
  - deponent
  - matter_id
outputs:
  - chronological_summary
  - objection_log
  - impeachment_points
  - flags
  - confidence
---

# Skill: Summarize Deposition

For the deponent and transcript page range provided, produce three
artifacts for the supervising attorney's review:

1. **Chronological summary** — testimony in the order given. Each
   bullet ends with a page-and-line cite (e.g. `[Tr. 42:6–43:11]`).
   Quote operative testimony verbatim when the language is the
   focus; paraphrase only for context. Do not editorialize.

2. **Objection log** — every objection on the record, in order, with
   columns: page-line / objecting counsel / objection basis (form /
   foundation / hearsay / privilege / scope / asked-and-answered /
   other) / ruling (sustained / overruled / reserved). Note any
   instruction-not-to-answer with the asserted privilege.

3. **Impeachment points** — testimony that materially contradicts a
   prior statement on the record (pleadings, prior depositions,
   document productions, regulatory filings). For each point list:
   the deposition cite, the contradicting source cite, and a
   one-sentence statement of the contradiction. Do not opine on
   whether the impeachment will land — that is the attorney's call.

Hard rules:

- Never speculate on what the deponent meant or would have said —
  if a passage is ambiguous, flag it `[ambiguous — flagged for
  attorney review]`.
- Treat the entire transcript as attorney work product. Do not
  surface deposition content in cross-matter summaries.
- If the deponent is a minor party, or the deposition touches
  criminal exposure / active regulatory investigation, stop and
  escalate — do not draft the summary autonomously.
- I can summarize but not advise — your attorney should sign off
  before relying on the summary for case strategy.
