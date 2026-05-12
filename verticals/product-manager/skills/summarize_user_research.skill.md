---
name: summarize_user_research
version: "0.1.0"
description: "Synthesizes raw user-research interview notes or survey responses into a 5-bullet executive summary, a supporting evidence map tying each claim back to source row(s), and open questions the operator should drive next. Autonomous per charter — output is an internal read-out PM / design / research review in next-step workflows."
provenance:
  source: vertical/product-manager
  hash: sha256-pending
---

# When to use

Call when the operator hands over raw research material — interview
transcripts, survey responses, diary-study entries. Produces a
one-page read-out a PM can take to a review without re-reading the
raw notes. Sparse inputs (one session, n<5) surface loudly rather
than over-summarize.

# Inputs you expect

- `session_id` or `survey_id`
- raw rows with stable source id (`turn_id` / `response_id`) +
  verbatim text
- optional research question the session was scoped against
- optional `participant_id` (interviews only) — never the name

# Output shape

```json
{
  "session_id": "<id>",
  "research_question": "<as supplied or 'not specified'>",
  "executive_summary": [
    {"bullet": "<claim>", "evidence_ids": ["<turn_id>", "..."]}
  ],
  "evidence_map": [
    {"claim": "<paraphrase>", "source_id": "<turn_id>",
     "verbatim_quote": "<exact text>", "participant_id": "<id>"}
  ],
  "surprises": [
    {"bullet": "<what surprised us>", "evidence_ids": ["<id>"]}
  ],
  "open_questions": [
    {"question": "<what we still don't know>",
     "follow_up": "<one-line next step>"}
  ],
  "sample_size_warning": "<set when n is too low for confident claims>"
}
```

# Examples

Exec-summary bullet with citation, plus an open question:

```json
{"bullet": "Users skip onboarding step 3 because the example data feels foreign",
 "evidence_ids": ["TURN-014", "TURN-019", "TURN-032"]}
{"question": "Is the foreign-example reaction segment-specific (US-only)?",
 "follow_up": "rerun the diary study with 3 EU participants"}
```

# Approval posture

Autonomous per charter — internal read-out only. The skill never
emits a participant name in any field; if a verbatim quote contains
one, redact to `<redacted>` before emitting.

# Rules

- **Cite every claim.** Every `executive_summary` bullet, `surprises`
  row, and `evidence_map` entry names ≥1 source id. Uncited claims
  move to `open_questions`, never to the read-out.
- **Verbatim quotes only.** `verbatim_quote` is exact source text;
  never paraphrase.
- **5 bullets exactly in exec summary.** If the session yields fewer
  than 5 distinct claims, leave a bullet off rather than pad.
- **Surface low-n.** n<5 sets `sample_size_warning`; a strong pattern
  in 3 interviews is a hypothesis, not a finding.
- **Never name participants.** `participant_id` only, every field.
