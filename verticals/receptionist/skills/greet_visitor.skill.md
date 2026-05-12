---
name: greet_visitor
version: "0.1.0"
description: "Opens the conversation with a context-matched greeting (walk-in / phone-in / message) in the operator's brand voice (formal or casual)."
provenance:
  source: vertical/receptionist
  hash: sha256-pending
---

# When to use

Call this skill **once per conversation**, at the very first
inbound turn. After the opener fires, switch to `identify_intent`
on subsequent messages. If the visitor's first message already
contains a request ("Hi, what are your hours?"), still emit the
greeting but keep it short — the request answer follows in the
same reply.

# Inputs you expect

- `channel` — one of `walk_in` / `phone_in` / `message`
- `time_of_day` — `morning` / `afternoon` / `evening` (local clock)
- `tone` — `formal` or `casual`, read from `prompts/persona.md`
  at invoke-time. Do **not** add a runtime knob; the persona is
  the source of truth.
- `visitor_name` — optional; present only for known recurring
  visitors via memory lookup.

# Output shape

A single greeting line (no JSON envelope, no trailing question).
1 sentence, ≤ 18 words. Plain Markdown text.

# Templates by channel × tone

| Channel | Formal | Casual |
|---|---|---|
| `walk_in` | "Good <morning/afternoon/evening>, welcome." | "Hey — come on in!" |
| `phone_in` | "Good <morning/afternoon/evening>, thanks for calling." | "Hi, thanks for ringing!" |
| `message` | "Hello, thanks for reaching out." | "Hi! Thanks for messaging." |

If `visitor_name` is present, append `", <name>"` before the
terminal punctuation — formal `"Good morning, Maria."`, casual
`"Hey Maria — come on in!"`.

# Examples

```
channel=walk_in, time_of_day=morning, tone=formal
→ "Good morning, welcome."

channel=phone_in, time_of_day=afternoon, tone=casual, visitor_name="Sam"
→ "Hi Sam, thanks for ringing!"

channel=message, time_of_day=evening, tone=formal
→ "Hello, thanks for reaching out."
```

# Anti-patterns

- "I'd be happy to assist you with that today!" — corporate filler.
- "As an AI receptionist…" — breaks the persona.
- Emojis in the greeting.
- Opening with a question ("How may I help you today?") — leaves
  the visitor to do the work. State the greeting; the next turn
  identifies intent.

# Charter boundary

This skill **never** schedules, dispatches SMS, or writes to a
CRM. Those scopes belong to the sibling `ai-receptionist`
vertical. Greeting only.
