---
name: structure_exit_interview
description: Given outgoing-employee context, produce an interview-question set + suggested tone calibration + a list of topics that need legal-counsel-present. Structuring is autonomous-safe; conducting the interview and sharing notes are gated.
inputs:
  - outgoing_employee_context
  - departure_reason
  - tenure_and_role
outputs:
  - interview_question_set
  - suggested_tone_calibration
  - topics_with_legal_present
  - legal_framework
  - flags
---

# Skill: Structure Exit Interview

Produce a structured exit-interview plan for one departing employee.

1. **Interview question set.** 10-15 open questions covering: role
   fit, manager relationship, team dynamics, growth opportunities,
   compensation context (factual, not negotiation), and
   would-you-return signal. No leading questions; no questions
   that fish for protected-class disclosures.
2. **Suggested tone calibration.** Pick one of: `warm-curious`
   (voluntary exit, positive tenure), `neutral-procedural`
   (performance-based exit, RIF), `careful-legal-aware`
   (departure under dispute / harassment claim / retaliation
   context). Match the departure reason; explain the choice in
   one sentence.
3. **Topics that need legal-counsel-present.** Always flag these
   topics for legal-counsel attendance — they are the load-bearing
   differentiator vs a normal exit chat:
   - **Severance terms** (consideration, release scope, ADEA OWBPA
     21/45-day review + 7-day revocation if 40+).
   - **Post-employment competition** (non-compete enforceability
     varies by state; FTC 2024 rule status).
   - **Whistleblower context** (SOX / Dodd-Frank / state-equivalent).
   - **Retaliation context** (Title VII / FMLA / NLRA / OSHA).
   - **Pending or threatened litigation / EEOC charge.**
4. **`next_action`** is always `structure_exit_interview` (the
   skill itself) — actually conducting the interview is
   operator-routed. Sharing the interview notes externally is
   `share_workforce_data_externally` with
   `requires_approval: true`.

Hard rules:

- **No questions that fish for protected-class info.** "Did your
  age affect your treatment?" is not on the question list. If the
  departing employee raises a protected-class concern voluntarily,
  redirect to a formal complaint channel and escalate.
- **Severance discussion is legal's job, not HR's.** The vertical
  structures the meeting and flags the topic — it does not draft
  severance terms.
- **ADA accommodation history stays out.** Even if the departure
  reason is "could not return after leave," accommodation
  specifics do not appear in the question set. Frame around the
  business decision, not the medical record.
- **Departure under retaliation / harassment dispute escalates
  before structuring.** Set
  `flags: ["departure_under_dispute"]` and route to legal counsel
  before the interview is scheduled.
