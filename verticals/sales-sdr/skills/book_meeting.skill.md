---
name: book_meeting
version: "0.1.0"
description: "Propose 3 candidate meeting times against operator-available blocks and draft a calendar-hold message. Drafting only — approval-gated, never auto-sends, never auto-books."
provenance:
  source: vertical/sales-sdr
  hash: sha256-pending
---

# When to use

The prospect has signaled interest (replied "yes, let's talk",
asked for a demo, accepted next-step). Pulls operator-available
time blocks from the gcal connector and the prospect's stated
timezone if known.

# Inputs you expect

- prospect (name + email + timezone if known)
- interest signal (the exact prospect message that gated this step)
- operator-available time blocks (from gcal connector)
- meeting duration (default 30 min)

# Output shape

A single fenced JSON block:

```json
{
  "requires_approval": true,
  "proposed_times": [
    {"start": "<ISO-8601>", "end": "<ISO-8601>", "tz_for_prospect": "<IANA tz>"}
  ],
  "hold_message": "<draft message body, includes the 3 options>",
  "reschedule_etiquette": "<one-sentence note on how to handle a no-thanks-to-these-times reply>",
  "calendar_event_draft": {
    "title": "<title>",
    "duration_minutes": 30,
    "attendees": ["<prospect email>", "<operator email>"]
  }
}
```

# Rules

- **`requires_approval: true` is always present.** This skill
  never books a slot; it stages a draft. The operator approves
  before the gcal connector emits an invite.
- **Three options, not one.** Single-option proposals coerce the
  prospect; three give them room. Spread across at least two
  days when the operator's calendar allows.
- **No same-day, no after-hours.** Minimum +1 business day in
  the prospect's timezone; default window 9am-5pm in that
  timezone unless the operator explicitly opted-in to wider
  hours.
- **Mirror the prospect's timezone in the hold message.** The
  draft text always shows times in the prospect's timezone
  first; the operator's timezone is parenthetical at most.

# Examples

Prospect Priya (Northwind Fintech, US/Eastern) replied "yes,
happy to chat next week." Operator has Mon/Wed/Thu open afternoons
ET. Output: three 30-min options across Mon 2pm / Wed 11am / Thu
3pm ET, hold-message draft "Picking from a few options below —
happy to take whichever fits…", `reschedule_etiquette: "If none
work, ask Priya for two windows that do and propose against those
— don't volley back another three."`, `requires_approval: true`.
