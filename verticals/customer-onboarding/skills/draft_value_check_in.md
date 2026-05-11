---
name: draft_value_check_in
description: Given usage data and the account's stated goals, draft a day-7 value-anchored check-in that highlights realized value and surfaces one open question.
inputs:
  - account_profile
  - playbook_state
  - usage_signals
  - engagement_thresholds
outputs:
  - outreach_draft
  - engagement_summary
  - next_action
  - flags
---

# Skill: Draft Value Check-in (Day 7)

Produce a value-anchored day-7 check-in. The message names what
the account has actually used, ties it back to the goals they
bought for, and asks one open question.

1. **Read usage.** Pull `usage_signals.logins_unique_days`,
   `features_touched`, `seats_activated_pct`, `tickets_count`
   from `playbook_state`. Compare each against
   `engagement_thresholds`.
2. **Tie to goals.** For each stated goal in
   `account_profile.goals`, name the 1-2 features the account
   has touched that move that goal forward. If a goal has zero
   matching feature usage, surface it — that is the open
   question, not a happy-path message.
3. **Realized value paragraph.** Specific and short: "Your team
   has signed in on 5 of the last 7 days, touched the X and Y
   features, and activated 4 of 6 seats." No marketing language.
4. **One open question.** A single, scoped question the customer
   can answer in a sentence. "Is Z still the right milestone for
   week 2?" beats a bulleted agenda.
5. **Engagement summary.** Tabulate observed vs. threshold for
   each signal. Mark `green` / `yellow` / `red` per signal.
6. **Next action.** `send_email_external` gated behind approval.
   If churn-keyword fires in any prior reply on
   `playbook_state.reply_history`, do NOT draft — flag and
   escalate.

Hard rules:

- Never claim value the data doesn't support. If
  `features_touched = 0`, the message acknowledges that.
- One question per check-in. Bulleted questions push the
  customer away.
- Set `flags: ["churn_keyword_in_reply"]` and stop drafting if
  any prior reply contains a churn keyword.
