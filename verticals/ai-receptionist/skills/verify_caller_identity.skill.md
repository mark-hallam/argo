---
name: verify_caller_identity
version: "0.1.0"
description: "Given an inbound call + caller-claimed identity, validate against the operator's known-contacts list using fuzzy name + phone match. Outputs a routing recommendation, never an authoritative identification."
provenance:
  source: vertical/ai-receptionist
  hash: sha256-pending
---

# When to use

Call at the start of an inbound call after the caller has
stated who they are, before any greeting-by-name or
data-disclosure decision. Output drives the opening line of
the conversation (greet by name vs. ask to clarify vs.
escalate to a human). Never used as proof of identity for
disclosing operator data.

# Inputs you expect

- `inbound_call`: `{caller_id_phone, caller_claimed_name, call_started_at}`
- `contact_book`: operator-curated list of known contacts
  `[{name, phones: [string], prior_call_count, last_seen_at}]`
  (read from `memory.contact_book[]`)
- optional `caller_history` row for the inbound `caller_id_phone`
  (last-7-day rotation; see schema.toml)

# Output shape

```json
{
  "matched_contact": {"name": "Jordan Lee", "phones": ["+15551234567"], "prior_call_count": 3},
  "confidence": 0.82,
  "verification_signals": {
    "phone_match": "exact",
    "name_match": "fuzzy_high",
    "prior_call_count": 3
  },
  "action": "greet_by_name",
  "flags": []
}
```

`confidence` is `[0.0, 1.0]`. `action` is exactly one of
`greet_by_name`, `request_clarification`, or
`escalate_to_human`.

# Examples

- Inbound phone matches a `contact_book` row exactly AND
  claimed name fuzzy-matches the row name (Levenshtein
  ratio ≥ 0.85) AND `prior_call_count ≥ 1` → `confidence ≈ 0.85`,
  `action: greet_by_name`.
- Inbound phone matches but the claimed name diverges from
  the row name (different person on a household line) →
  `confidence ≈ 0.55`, `action: request_clarification`,
  flag `phone_name_mismatch`.
- No phone match AND no name fuzzy-match in `contact_book` →
  `confidence ≈ 0.20`, `action: request_clarification`,
  flag `unknown_caller`.
- Inbound caller appears in `contact_book` with a
  `do_not_engage` tag (operator-set) → `action:
  escalate_to_human` regardless of name/phone signals; flag
  `do_not_engage_match`.

# Approval posture

Autonomous. The skill never claims a caller IS the matched
contact — output is a routing recommendation only. Hard
rules:

- **0.7 confidence threshold.** `confidence < 0.7` always
  routes to `request_clarification`. Never `greet_by_name`
  below the threshold, even on a strong single signal
  (phone match alone is not sufficient — shared lines, ported
  numbers).
- **PII-soft.** The matched contact's full name appears in
  the structured output, but caller-facing greeting drafts
  use first-name only and never echo `last_seen_at` /
  `prior_call_count` aloud.
- **Never disclose contact_book contents** as part of
  `request_clarification`. Ask an open-ended "may I have
  your name and the company you're with?" — do not say
  "is this Jordan Lee?".
- **Stale rows.** If the matched row's `last_seen_at` is
  > 12 months old, downgrade `confidence` by 0.15 and flag
  `stale_contact_row` — operator decides whether the row
  still applies.
