---
name: draft_memo
description: Draft an IRAC-structured legal memo (Issue / Rule / Application / Conclusion) with primary-source citations for the supervising attorney's review.
inputs:
  - issue
  - jurisdiction
  - facts
outputs:
  - memo
  - sources_consulted
  - confidence
  - flags
---

# Skill: Draft Memo

Given an issue, the controlling jurisdiction, and the facts on the
record, draft an IRAC-structured memo the supervising attorney can
review and sign. This is a draft; never finalize, send, or share.

Structure the output with these four sections, in order, each
explicitly labeled:

1. **Issue** — one-sentence framing of the legal question, naming
   the jurisdiction and the operative cause of action. If the issue
   is unclear from the facts, list the candidate framings and ask
   the attorney to pick — do not resolve ambiguity.
2. **Rule** — the controlling statute / regulation / case law in
   the named jurisdiction. Cite each authority in Bluebook form
   inline (case name + reporter + court + year, or statute section).
   Secondary sources may add context but cannot stand alone.
3. **Application** — apply each element of the rule to the facts on
   the record. Quote operative clause / statutory language verbatim.
   Where the record is silent on a required element, mark the gap
   `[fact gap — flagged for attorney follow-up]`.
4. **Conclusion** — describe the likely outcome under the rule as
   applied, with a confidence call (high / medium / low). Do NOT
   recommend a course of action — recommendation is the attorney's
   call via `issue_legal_opinion`.

Always end the memo body with: "I can summarize but not advise —
your attorney should sign off before relying on this."

Hard rules:

- If the jurisdiction is unclear or the matter touches more than one
  state, flag the choice-of-law question; do not pick.
- If any authority cited is in `cited_authorities` with a signal
  tag of `criticized` or `overruled`, surface that tag in the Rule
  section — never cite overruled law as good law.
- If facts hint at criminal exposure, regulatory investigation, a
  minor party, or a conflict in `conflict_index`, STOP and escalate
  rather than draft.
