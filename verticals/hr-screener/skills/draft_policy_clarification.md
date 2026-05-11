---
name: draft_policy_clarification
description: Given an ambiguous policy + a scenario, produce a plain-language clarification + a list of edge cases the current policy does not cover + recommended policy-update language. Drafts only — sending or publishing the clarification routes through approval.
inputs:
  - ambiguous_policy
  - scenario
  - policy_handbook
outputs:
  - clarification
  - edge_cases_not_covered
  - recommended_policy_update_language
  - legal_framework
  - flags
---

# Skill: Draft Policy Clarification

Produce a plain-language clarification for one ambiguous policy
applied to one scenario.

1. **Read the policy text.** Pull the exact policy from
   `policy_handbook` in memory. Cite the section number + last
   amended date. Do not invent a policy that is not in the
   handbook — set `flags: ["policy_not_found"]` and escalate.
2. **Plain-language clarification.** State, in plain English, how
   the policy applies to the scenario. ≤ 200 words. No jargon, no
   "stakeholder," no "alignment" — employees and managers both
   read this draft.
3. **Edge cases not covered.** List 3-5 specific scenarios the
   current policy does NOT clearly cover. Each edge case names a
   concrete circumstance + which way the policy would currently
   point (ambiguously / not at all / contradictorily).
4. **Recommended policy-update language.** Propose specific text
   that would close the ambiguity. The recommendation must:
   - Cite the relevant statute or regulation (Title VII / ADA /
     FMLA / FLSA / state-equivalent) when one applies.
   - Stay in the policy's existing voice and structure.
   - Be drafted as a *recommendation* — `next_action` is
     `send_company_wide_communication` with
     `requires_approval: true`, gated through HR-officer + (if
     touching a regulated area) legal.

Hard rules:

- **Do not give legal advice.** Naming a statute is transparency
  about which framework applies, not legal advice (UPL — see
  legal-paralegal). When the scenario asks "is this legal?", set
  `flags: ["legal_question_needs_counsel"]` and route to legal
  counsel.
- **Do not echo policy text that does not exist.** If the
  handbook is silent on the scenario, say so — invent nothing.
- **Do not propose a policy change that would conflict with
  superseding law.** E.g., a leave-policy update that would
  shorten FMLA-protected leave is invalid on its face. Flag
  `policy_update_conflicts_with_statute` and escalate.
- **No protected-class carve-outs.** Updates that differentially
  apply to protected classes require pay-equity / disparate-impact
  analysis under the EEO-1 gate. Without it, refuse and
  escalate.
