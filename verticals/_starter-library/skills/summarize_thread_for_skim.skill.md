---
name: summarize_thread_for_skim
version: "0.1.0"
description: "Condenses a multi-reply thread (email, chat, comment trail) into a chronological skim — who said what, what got decided, what's still open."
provenance:
  source: vertical/_starter-library
  hash: sha256-pending
---

# When to use

The operator was added late to a long thread (or has been away)
and needs to catch up without reading every message. Use any
time the question is "what did I miss in this thread?"

# Inputs

- `messages`: an ordered list of `{from, timestamp, body}`.
- `lens` (optional): the angle to lead with — `"decisions"`,
  `"action items"`, `"open questions"`, `"sentiment"`. Default:
  balanced.
- `max_words`: cap on the output. Default 200.

# Output

Three sections:
- **Timeline** — 3-7 bullets, each prefixed with author + short
  timestamp, naming the move they made (proposed X / pushed
  back on Y / agreed to Z). Skip filler messages.
- **Decided** — what the thread converged on (empty if
  unresolved).
- **Open** — questions or asks still unanswered, with who is
  expected to respond.

# Examples

- 40-message email thread → tight timeline highlighting the 5
  pivotal turns, a clean "Decided" + "Open" split.
- Thread that didn't reach a decision → "Decided" empty, "Open"
  enumerates the unanswered questions and who owns each.

# Approval posture

Read-only. Summary is for catch-up; skill does not reply on the
operator's behalf or notify thread participants.

# Notes

- Skip messages that add no signal (one-word "+1", "thanks"
  replies, calendar bot pings). Keep them out of the timeline.
- Attribute moves to the person who made them, not "someone
  said." A timeline without attribution is unusable.
- If the thread is short (≤ 5 messages), return it lightly
  trimmed rather than artificially "summarising" — short
  threads read faster than their summaries.
- Quote a decisive sentence verbatim where it shaped the
  outcome — paraphrase loses the load-bearing phrasing.
