---
name: setup_health_check
description: Given an account, score setup completeness against the operator's checklist and suggest the next unblocking action.
inputs:
  - account_profile
  - playbook_state
  - setup_checklist
outputs:
  - health_score
  - engagement_summary
  - next_action
  - flags
---

# Skill: Setup Health Check (Day 3)

Score the account against `setup_checklist` and produce the
day-3 check-in draft if a gap is found.

1. **Checklist items.** Read each from `setup_checklist` and
   score boolean `complete` against `playbook_state`. Default
   items: admin invited, SSO enabled, first integration
   connected, first 3 seats activated, billing verified. The
   operator can extend this in `inventory.json`.
2. **Completeness ratio.** Compute `complete_count / total`.
   Compare against `engagement_thresholds.setup_min_complete`
   (default 0.6 by day 3). Below threshold = `yellow`. Below 0.3
   = `red`. Any blocking-tagged item missing = `red` regardless.
3. **Engagement summary.** Name each missing item explicitly:
   "SSO not enabled (blocking), 2 of 5 seats activated, billing
   verified, admin invited, first integration connected".
4. **Next action.** If `health_score = green`, recommend
   continuing the playbook to day 7. If `yellow` or `red`, draft
   a check-in (a separate `outreach_draft`) that names the
   missing piece in plain English and offers a 15-minute
   unblocking call. Gate `send_email_external` behind approval.
5. **Stuck activation escalation.** If `playbook_state.logins =
   0` on day 5 or later, set `flags:
   ["stuck_activation_no_login"]` and escalate — do not send the
   normal day-3 check-in.

Hard rules:

- Never paper over a blocking gap with a happy-path message.
- Numbers carry units: "2 of 5 seats" not "low seat activation".
- If `setup_checklist` is empty or `playbook_state` is missing,
  set `flags: ["missing_state"]` and ask the operator.
