---
name: draft_lifecycle_sequence
version: "0.1.0"
description: Draft a multi-step lifecycle sequence (drip / onboarding / newsletter / win-back), define its target segment from real criteria, and surface scheduling/broadcasting as approval-gated — drafting is autonomous, sending never fires without approval and no stat or list size is ever invented.
provenance:
  source: vertical/gtm-lifecycle-email
  hash: sha256-pending
---

# When to use

Use when the operator wants a lifecycle email or a multi-step sequence
drafted — a new-user drip, a newsletter, an in-product nudge, or a
win-back flow for dormant users. This is Lena's anchor skill: it drafts
the copy, defines the target segment from real criteria, and surfaces
the send as an approval-gated proposal. Drafting and segmenting are
autonomous; the skill NEVER schedules a sending sequence or sends a
broadcast itself — those stop at the operator.

# Inputs you expect

- `kind` — the sequence type: `drip` | `onboarding` | `newsletter` |
  `in_product_nudge` | `win_back`.
- `segment` — the target audience, defined from real inclusion/exclusion
  criteria. The list size is pulled from the email platform / CRM /
  analytics connectors when wired, from operator paste-ins otherwise.
  A segment with no sourced count is `"unavailable"`, never invented.
- `goal` — what each step should accomplish (e.g. re-activate, drive
  first value, announce a feature).
- `source_facts` — any stats, testimonials, or quotes available to use,
  each with a source. A claim with no source is marked inferred or left
  out — never written as fact.

# Output shape

```json
{
  "segment": {
    "segment_id": "dormant-30d",
    "label": "Signed up, no login in 30 days",
    "inclusion_rules": ["status=signed_up", "last_login>30d"],
    "exclusion_rules": ["status=churned"],
    "size": "unavailable",
    "size_source": null
  },
  "sequence": {
    "kind": "win_back",
    "steps": [
      { "goal": "Remind them of the value they set up",
        "delay": "day 0", "subject": "...", "body": "..." },
      { "goal": "Offer a guided path back in",
        "delay": "day 3", "subject": "...", "body": "..." }
    ]
  },
  "proposed_send": {
    "kind": "sequence",
    "sequence_id": "win-back-dormant-30d",
    "segment": "dormant-30d",
    "recipient_count": "unavailable",
    "requires_approval": true
  }
}
```

`proposed_send` is optional — omit it when the operator only wants a
draft. When present it ALWAYS carries `requires_approval: true`; the
skill never schedules or broadcasts on its own.

# Hard rules — truth boundary (NO THEATER)

- **Never fabricate a stat, quote, testimonial, or open rate.** Each
  claim in the copy cites its `source`, or is marked **inferred**, or is
  left out. A made-up figure is never acceptable.
- **Never invent a list size or segment count.** Report `size` only from
  a sourced count; otherwise `"unavailable"`. Never estimate a list.
- **Thin evidence stays low-confidence.** Do not inflate two data points
  into a headline claim.
- **Sending is approval-gated.** `schedule_sequence` and `send_broadcast`
  both `requires_approval: true` and NEVER auto-fire. Any `proposed_send`
  this skill emits is a proposal handed to the operator, not a send.
- **Never send to the whole list — or any list — on your own.** If asked
  to "just send it", produce the draft and surface the send as gated;
  do not broadcast.

# Sequence-kind routing

Pick the sequence kind by the lifecycle stage:

- new sign-ups not yet at first value → `onboarding` / `drip`
- active base, periodic update → `newsletter`
- in-app moment, contextual prompt → `in_product_nudge`
- dormant / lapsed users → `win_back`

# Example

Synthetic only — no real metrics, no real recipients.

```
kind: win_back
segment: { label: "Signed up, no login in 30 days",
           inclusion_rules: ["status=signed_up", "last_login>30d"] }
goal: re-activate dormant users
source_facts: []   # no sourced stats supplied
```

```json
{
  "segment": {
    "segment_id": "dormant-30d",
    "label": "Signed up, no login in 30 days",
    "inclusion_rules": ["status=signed_up", "last_login>30d"],
    "exclusion_rules": ["status=churned"],
    "size": "unavailable",
    "size_source": null
  },
  "sequence": {
    "kind": "win_back",
    "steps": [
      { "goal": "Remind them of the value they set up",
        "delay": "day 0",
        "subject": "Your Acme workspace is ready when you are",
        "body": "Hi there — your workspace is still set up..." },
      { "goal": "Offer a guided path back in",
        "delay": "day 3",
        "subject": "A 2-minute way back in",
        "body": "Pick up where you left off..." }
    ]
  },
  "proposed_send": {
    "kind": "sequence",
    "sequence_id": "win-back-dormant-30d",
    "segment": "dormant-30d",
    "recipient_count": "unavailable",
    "requires_approval": true
  }
}
```
