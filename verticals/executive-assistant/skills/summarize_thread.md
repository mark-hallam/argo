---
name: summarize_thread
description: Reduce a long email thread to a 5-bullet summary plus the decision points outstanding for the principal.
inputs:
  - thread_id
  - messages
outputs:
  - summary
  - decision_points
  - deadline
---

# Skill: Summarize Thread

Use when a thread is >5 messages or when the principal is being
looped in late and needs context fast.

1. **Lead with the ask.** Bullet 1: one sentence on what the thread
   wants from the principal. If the thread is informational, say so.
2. **Five bullets max.** Each bullet is one sentence. Compress
   ruthlessly; prefer "Acme proposed X at $Y" over verbatim quotes.
3. **Decision points.** List every open question that needs the
   principal's input. Empty list if the thread is FYI-only.
4. **Deadline.** Extract the implicit response window if any. Look for
   explicit dates, "by EOW", "before the board meeting", or invite
   RSVPs. Return `null` if none stated.
5. **New since last touch.** If the principal previously participated,
   highlight what's changed: new attendees, new dollar amounts, new
   commitments, scope shifts.

Never fabricate decisions. If a bullet would require inferring intent
beyond what the messages state, label it `inferred` and shorten it.

Output shape:

```json
{
  "summary": [
    "Acme is asking for a renewal commitment by Friday.",
    "Their proposal is $180K/yr, up from $150K (a 20% increase).",
    "Legal flagged the indemnity clause as more aggressive than v1.",
    "Procurement has not yet weighed in.",
    "Our champion at Acme (Jane Doe) has changed roles since last touch."
  ],
  "decision_points": [
    "Accept the 20% increase, counter, or escalate to negotiation?",
    "Re-engage legal on indemnity before responding?"
  ],
  "deadline": "2026-05-15"
}
```
