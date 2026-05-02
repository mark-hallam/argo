---
name: identify_intent
version: "0.1.0"
description: "Maps an inbound visitor message to one of seven canonical intent labels for receptionist routing."
provenance:
  source: vertical/receptionist
  hash: sha256-pending
---

# When to use

Call this skill on **every** inbound message. The intent label
drives the response template AND the escalate flag.

# The seven intents

| Intent | What it looks like |
|---|---|
| `greeting` | "hi", "hello", "good morning", any opener with no embedded request |
| `info_request` | "what are your hours?", "where are you located?", "do you take walk-ins?" — answerable from the FAQ memory |
| `book_meeting` | "I'd like to schedule…", "can I meet with X?", "what's your availability?" |
| `support_issue` | "my account isn't working", "I can't log in", "the thing crashed" — anything technical |
| `complaint` | "this is unacceptable", "I'm frustrated", "you billed me wrong" — emotional + grievance |
| `out_of_scope` | anything genuinely outside the receptionist's remit (legal advice, medical questions, philosophical debate) |
| `chitchat` | small talk with no real ask: "how's your day", "nice weather" |

# Disambiguation rules

- A greeting **followed by** a request collapses to the request.
  "Hi, what are your hours?" → `info_request`.
- A complaint **wrapped around** a support issue stays as a
  complaint. "Your stupid app keeps crashing" → `complaint`.
- An info_request the FAQ memory CAN'T answer escalates as
  `out_of_scope` (the FAQ defines the receptionist's remit).
- Multiple intents in one message → use the most actionable one.
  "Hi, my login broke and I want to schedule a call to fix it" →
  `support_issue` (escalate now; the meeting comes later).

# When in doubt

Pick `out_of_scope` and escalate. False-negatives on escalation
are much costlier than false-positives at the front desk.
