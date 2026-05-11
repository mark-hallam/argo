---
name: draft_welcome_email
description: Given an account profile, draft a day-0 welcome email that names the buyer + champion, restates the goals the account bought for, and offers 3 concrete setup pointers.
inputs:
  - account_profile
  - operator_signature
  - playbook_template
outputs:
  - outreach_draft
  - next_action
  - flags
---

# Skill: Draft Welcome Email (Day 0)

Produce a personalized welcome message for the named account
within the day-0 window (≤ 24h of contract signature).

1. **Name the people.** Open by naming the buyer and the main
   champion from `account_profile`. If either is missing, do not
   draft a generic "Welcome team" — flag the gap and ask the
   operator for the missing name.
2. **Restate the goals.** Echo the 1-2 goals the account bought
   for, in the customer's own language where available. No
   marketing language; no "value realization journey".
3. **Three setup pointers.** Concrete, specific, ordered:
   admin account, first-integration, first-value milestone. Pull
   the exact milestone from `playbook_template.first_value`.
4. **Smallest next step.** Offer a 15-minute setup call or a
   single link to the admin invite flow — never a multi-step
   onboarding agenda.
5. **Sign-off.** Use `operator_signature`; never invent a name.

Hard rules:

- Do not promise an SLA, a discount, a feature on the roadmap,
  or any commitment that isn't in `account_profile.contract`.
- Do not reference seat-count or pricing in the welcome — that
  belongs in the day-3 setup check.
- Output is a `outreach_draft` only. The `next_action` must be
  `send_email_external` with `requires_approval: true`.
- If the account profile lacks a buyer name, champion, or stated
  goal, set `flags: ["missing_account_profile_fields"]` and stop
  drafting — escalate to the operator.
