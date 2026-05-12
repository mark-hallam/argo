---
name: answer_faq
version: "0.1.0"
description: "Matches a visitor question against the operator-provided FAQ knowledge base and returns the best answer with a citation. Falls back to human handoff below 0.7 match confidence."
provenance:
  source: vertical/receptionist
  hash: sha256-pending
---

# When to use

Call this skill after `identify_intent` returns `info_request`.
The FAQ knowledge base is operator-curated (memory-seeded);
treat it as the receptionist's bounded source of truth. Do
**not** invent answers, paraphrase from the web, or extend the
business's policy on your own.

# Inputs you expect

- `question` — the visitor's question, verbatim
- `faq` — list of `{id, question, answer}` entries from
  workspace memory (seeded by the operator)
- `confidence_threshold` — defaults to `0.7`; operator may
  override via persona policy

# Output shape

JSON envelope:

```
{
  "match_id": "<faq:id> | null",
  "answer": "<one-sentence answer> | null",
  "citation": "[faq:<id>] | null",
  "confidence": <float 0.0-1.0>,
  "requires_human_followup": <bool>,
  "bridge": "<templated sentence> | null"
}
```

# Match rules

1. Score each FAQ entry against the visitor question by topical
   overlap + intent fit. Pick the highest-scoring entry.
2. If `confidence ≥ 0.7`: return the matched answer + citation;
   `requires_human_followup = false`; `bridge = null`.
3. If `confidence < 0.7`: leave `match_id`, `answer`, `citation`
   as `null`; set `requires_human_followup = true`; emit the
   bridge template `"Let me get a colleague who knows this better."`.
4. Never blend the answer with content from outside the FAQ. If
   the question is partially answered, treat as miss and
   escalate — partial answers are the leading source of FAQ
   complaints.

# Why 0.7

The threshold is the operator's safety knob. Too low → confident
wrong answers cited as house policy. Too high → constant
escalation, frustrating visitor experience. 0.7 is the documented
default; the persona policy file overrides for tenants who want
stricter (0.85) or looser (0.6) behavior.

# Examples

```
question: "Are you open Saturdays?"
faq: [{id: "hours_sat", question: "Saturday hours", answer: "10am-2pm"}]
→ {match_id: "hours_sat", answer: "We're open Saturdays 10am-2pm.",
   citation: "[faq:hours_sat]", confidence: 0.91,
   requires_human_followup: false, bridge: null}

question: "Do you offer financing on a custom order?"
faq: [{id: "payment_methods", ...}, {id: "returns", ...}]
→ {match_id: null, answer: null, citation: null, confidence: 0.42,
   requires_human_followup: true,
   bridge: "Let me get a colleague who knows this better."}
```

# Charter boundary

Answers from the FAQ only. This skill never schedules, sends
SMS, or writes to a CRM. Out-of-scope questions route through
`escalate_out_of_scope`.
