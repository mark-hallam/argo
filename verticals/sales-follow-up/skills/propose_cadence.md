---
name: propose_cadence
description: Given a lead's stage + last-touch + engagement-history, propose the next-touch (day + channel), an escalation threshold after N consecutive no-responses, and a dormancy threshold past which the lead drops out of cadence. Drafting only — no external effect.
inputs:
  - lead_stage
  - last_touch_at
  - engagement_history
  - cadence_policy
outputs:
  - recommended_next_touch
  - escalation_after_n_no_response
  - dormancy_threshold_recommendation
  - flags
---

# Skill: Propose Cadence

Recommend the next-touch shape without sending anything.

1. **Read the stage.** `cadence_policy` maps each stage
   (warm / engaged / dormant) to a day-offset and a preferred
   channel. If the stage isn't in the policy, flag
   `unknown_stage` and escalate — the vertical never invents
   a cadence rule.
2. **Compute next-touch.** `last_touch_at + day_offset` for
   the stage. Express in the operator's timezone with both
   the date and the channel (`email` / `linkedin_message`).
   Never propose same-day touches; minimum is +1 business day.
3. **Engagement-history overrides.** If the lead opened or
   replied to the last touch, shorten the offset (warm
   acceleration). If two consecutive touches went unread,
   lengthen the offset and flag `engagement_decay`.
4. **Escalation threshold.** Default is 3 consecutive
   no-responses → escalate to the operator with
   `escalate_after_failures = 3`. Operator overrides win.
   Never auto-drop a lead silently.
5. **Dormancy threshold.** Default is 90 days since the last
   reply → recommend moving the lead to
   `lead_stage = dormant`, which the operator commits in
   their CRM. The vertical surfaces the recommendation; it
   never mutates `lead_index` itself.

Hard rules:

- **No new touches inside a holiday window.** If
  `cadence_policy.respect_holidays` is true and the proposed
  date falls on a regional holiday, push to the next business
  day and flag `holiday_shift`.
- **Approval gates.** `recommended_next_touch` is a
  *recommendation*; the actual `schedule_call` /
  `add_to_sequence` actions require operator approval. This
  skill never schedules.
- **Dormancy does not equal reject.** A dormant lead can be
  re-engaged on a fresh signal — `research_lead_signals` is
  the canonical re-entry point. Don't recommend deletion.
- **If `engagement_history` is empty AND
  `last_touch_at` is null** — flag `no_history` and ask the
  operator to seed at least one prior touch before the
  cadence machinery makes any recommendation.
