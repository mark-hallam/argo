---
name: summarize_stakeholder_views
version: "0.1.0"
description: "Synthesises the positions of multiple stakeholders from raw input (emails, transcripts, comments) into a per-stakeholder summary plus a shared agreement/disagreement map."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator has input from multiple people on the same topic
(thread responses, interview notes, comment dump) and needs the
shape of the conversation: who wants what, where they agree,
where they don't. Use any time the deliverable is "summarise
where everyone stands."

# Inputs

- `inputs`: a map `stakeholder → raw_text` (their email,
  transcript excerpt, comment trail).
- `topic`: the one-sentence framing of what they're weighing in
  on. Used to filter off-topic asides.
- `show_attribution`: bool (default true). If false, anonymise
  per-stakeholder summaries with role labels only.

# Output

Three parts:
- **Per stakeholder** — one short paragraph per person/role: their
  position, top concern, and what would change their mind.
- **Where they agree** — bulleted list of shared positions.
- **Where they disagree** — bulleted list of contested points,
  naming which stakeholder holds which side.

# Examples

- Inputs from PM/Eng/Sales on a launch date → 3 paragraphs +
  agreement (everyone wants Q3) + disagreement (PM wants early
  Q3, Eng wants late, Sales wants tied to event).
- Inputs from 5 customers on a feature → role-anonymised version
  with `show_attribution: false`.

# Approval posture

Read-only synthesis. Operator decides what to do with the map.

# Notes

- Never invent a position for a stakeholder who didn't speak on
  the topic. List them as "no comment on this question" rather
  than inferring.
- Attribute positions verbatim where possible (quote a key line).
  Paraphrased positions are the leading source of "but that's
  not what I said" pushback.
- "Where they disagree" must name the disagreement explicitly —
  "they disagree about timing" is too vague; "PM wants July,
  Eng wants September" is right.
