---
name: escalate_out_of_scope
version: "0.1.0"
description: "Packages a conversation for human handoff when the question falls outside the FAQ or persona scope (clinical / legal / financial / personal advice, or unknown). Always routes to a human; never attempts to answer."
provenance:
  source: vertical/receptionist
  hash: sha256-pending
---

# When to use

Call this skill the moment a visitor request lands outside the
receptionist's remit. Triggers include `identify_intent` returning
`out_of_scope`, an `answer_faq` confidence miss on a topic that
requires expertise, or any question touching clinical / legal /
financial / personal-advice territory. **Never** attempt to
answer these yourself.

# Inputs you expect

- `transcript` — the full conversation so far
- `routing_rules` — operator-configured map of category → page-target
  (e.g. `clinical → on_call_nurse`, `legal → counsel@`, `unknown → manager`)
- `urgency_signals` — optional flags from the conversation
  (visible distress, time-sensitive phrasing, safety language)

# Output shape

JSON envelope:

```
{
  "classified_as": "clinical" | "legal" | "financial" | "personal" | "unknown",
  "urgency": "routine" | "elevated" | "urgent",
  "page_target": "<routing rule target> | null",
  "transcript": [...],
  "bridge": "<sentence read aloud to the visitor>"
}
```

# Five-category vocabulary

| Category | Examples |
|---|---|
| `clinical` | symptoms, medication questions, diagnosis, dosing, treatment plans |
| `legal` | rights, contracts, liability, regulatory advice, threats of action |
| `financial` | investment advice, account-specific guidance, debt, tax planning |
| `personal` | relationship advice, sensitive life decisions, emotional crisis |
| `unknown` | clearly outside scope but doesn't fit the above buckets |

Pick exactly one. If the request straddles two (e.g. clinical
billing question), pick the higher-stakes one (`clinical` over
`financial`) and let the human triage further.

# Urgency rubric

- `urgent` — explicit safety language ("I'm having chest pain",
  "I'm being threatened"), or routing rules tag the category as
  always-urgent.
- `elevated` — time-sensitive but no immediate safety risk.
- `routine` — default; informational request requiring a human
  but not racing the clock.

# Bridge templates

- `clinical` → "I can't give clinical advice — let me get
  someone who can. Hold on one moment."
- `legal` → "That's a legal question — I'll get the right person
  on the line."
- `financial` → "Account specifics need a person, not me — one
  moment while I connect you."
- `personal` → "I want to make sure you talk to the right person
  for that. Hold on."
- `unknown` → "Let me grab a colleague who can help with that."

# Charter boundary

Always routes; never resolves. This skill does not schedule,
text, or write to a CRM — its sole output is the escalation
envelope. The operator's inbox handles the human side downstream.
