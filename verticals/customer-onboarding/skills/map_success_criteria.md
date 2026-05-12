---
name: map_success_criteria
description: Given the customer's stated goals and the product's capability surface, draft a 5-bullet success-criteria document. Each bullet is measurable, time-bound, and names a responsible party. Drafting only — sharing with the customer is approval-gated.
inputs:
  - account_profile
  - product_capability_surface
  - playbook_template
  - operator_signature
outputs:
  - success_criteria_doc
  - engagement_summary
  - next_action
  - flags
---

# Skill: Map Success Criteria (Day 0–7)

Draft the customer's success-criteria document. Onboarding docs
longer than five bullets get ignored, so the cap is a hard
operator-respect rule.

1. **Source the goals.** Read `account_profile.goals` verbatim
   where available. If the field is empty or fewer than two
   goals are present, set `flags: ["missing_account_goals"]`
   and escalate — never invent goals from the contract or
   industry assumptions.
2. **Anchor each bullet on a capability.** For each candidate
   bullet, point at one entry in
   `product_capability_surface`. A bullet that doesn't tie to a
   capability the customer's plan includes is dropped, not
   reworded.
3. **Five bullets, no more.** Output exactly five (or fewer if
   the customer has fewer than five matchable goals). Never
   produce a sixth. If you must choose, prefer goals the
   customer named verbatim over inferred goals.
4. **Each bullet is measurable, time-bound, named.** Format:
   `<outcome>, measured by <metric>, by <date or playbook
   day>, owned by <named person from account_profile>`.
   Examples:
   - "First three integrations connected, measured by
     `playbook_state.integrations_count ≥ 3`, by day 14, owned
     by Priya (champion)."
   - "Onboarding ROI report shared internally, measured by
     `playbook_state.exec_share_logged = true`, by day 30,
     owned by James (buyer)."
5. **No vague verbs.** Drop "improve", "leverage", "drive",
   "enable". Replace with a number, a date, or a named
   artefact. If you cannot, the goal is not yet measurable —
   flag it and ask the operator for refinement.
6. **Output.** `success_criteria_doc` is a Markdown string
   containing a single 5-bullet list (no preamble, no
   closing). `next_action` is `share_success_criteria_doc`
   with `requires_approval: true` and `operator_signature` on
   the outbound message.

Hard rules:

- **No marketing language.** "Realize value", "unlock
  potential", "drive transformation" — all forbidden.
- **Never promise a feature on the roadmap** or any
  capability not present in
  `product_capability_surface.available_in_plan`.
- **Customer-facing sharing is approval-gated.** This skill
  drafts the document; the operator approves before any
  outbound action.
- **Responsible party must be a real person.** "The team",
  "stakeholders", "leadership" are not owners. If
  `account_profile` lacks a named owner for a bullet, drop
  that bullet and flag `missing_owner` rather than substitute
  a role.
