---
name: take_message
version: "0.1.0"
description: "Captures a structured message for the operator inbox when the human escalation target is unavailable. Confirms back to the visitor in plain language."
provenance:
  source: vertical/receptionist
  hash: sha256-pending
---

# When to use

Call this skill when `escalate_out_of_scope` fires **and** the
designated `page_target` is unavailable (off-hours, no answer,
operator-configured "leave message" routing). Also valid for
walk-ins / phone-ins requesting a callback when no human is on
the desk. Never use this to defer answering an FAQ-shaped
question — call `answer_faq` first.

# Inputs you expect

- `visitor_name` — captured from greeting or asked-for here
- `callback` — phone, email, or "walk in again" preference
- `topic` — visitor's one-line description of why they're reaching out
- `urgency` — visitor-stated or inferred (`routine` / `elevated` / `urgent`)
- `timestamp` — ISO-8601 of message capture (the runtime provides it)

# Output shape

JSON envelope suited to the operator inbox / Approvals page:

```
{
  "kind": "message",
  "visitor": {
    "name": "<string>",
    "known": <bool>
  },
  "contact": {
    "method": "phone" | "email" | "walk_in",
    "value": "<string>"
  },
  "topic": "<one-line summary>",
  "urgency": "routine" | "elevated" | "urgent",
  "timestamp": "<ISO-8601>",
  "verbatim": "<visitor's words, if captured>",
  "confirmation_to_visitor": "<plain-language readback>"
}
```

# Capture rules

1. **Ask for the missing field** — if `callback` or `topic` are
   absent, ask **once**, plainly: "What's the best way to reach
   you?" / "What should I tell them it's about?". Don't loop.
2. **Read back the message** to the visitor in 1 sentence before
   closing: "Got it — I'll pass along that <topic> and have <person
   or 'someone'> reach you at <callback>." This goes in
   `confirmation_to_visitor`.
3. **Default urgency to `routine`** unless visitor explicitly
   says "urgent", "ASAP", "today", or `escalate_out_of_scope`
   classified the underlying request as `urgent`.
4. **`known` is `true`** only if memory lookup confirms a prior
   visit; otherwise `false`.

# Examples

```
visitor_name: "Alex Chen", callback: "555-0142" (phone),
topic: "follow-up on tax filing", urgency: routine
→ envelope as above + confirmation_to_visitor:
  "Got it — I'll pass along that you'd like a follow-up on tax filing
   and have someone call you at 555-0142."
```

# Charter boundary

Captures only — does not dispatch SMS, schedule a callback, or
write to a CRM. The envelope drops into the operator inbox;
downstream tooling (outside this vertical) handles delivery.
