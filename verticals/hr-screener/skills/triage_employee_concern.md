---
name: triage_employee_concern
description: Given an inbound employee concern report, produce a severity rating + escalation Y/N (with the specific statute citation), recommended next step, and a named-individual-redacted summary. Triage is autonomous-safe — no external effect.
inputs:
  - concern_report
  - reporter_role
  - named_individuals
outputs:
  - severity
  - escalation
  - legal_framework
  - recommended_next_step
  - redacted_summary
  - flags
---

# Skill: Triage Employee Concern

Produce a severity rating + escalation call for one inbound concern.

1. **Severity rating.** `low` (FYI / minor friction; route to
   manager 1-on-1), `medium` (substantive issue / PIP-recommended /
   coaching conversation), `high` (regulated-conduct allegation /
   safety / harassment / retaliation / discrimination).
2. **Escalation Y/N with statute citation.** If `yes`, name the
   specific law (Title VII / ADA / ADEA / FMLA / GINA / USERRA /
   NLRA / OSHA / state-equivalent) — vague "this is serious"
   citations are not sufficient. If `no`, recommend the
   non-escalation next step.
3. **Recommended next step.** Concrete: who acts, on what
   timeline, with which approval gate. If the step is one of the
   five gated actions (modify_employee_record / approve_pto_request
   / initiate_disciplinary_action / share_workforce_data_externally
   / send_company_wide_communication), set
   `requires_approval: true`.
4. **Named-individual-redacted summary.** Replace every named
   individual with a role label ("the complainant," "the named
   manager," "the named peer"). This is the audit-trail-safe
   version of the concern; the un-redacted version stays with the
   HR officer.

Hard rules:

- **Retaliation patterns escalate.** Protected activity (EEOC
  charge / safety complaint / FMLA leave / union steward) followed
  by an adverse action sets `flags:
  ["retaliation_pattern_detected"]` and escalates with the Title
  VII / FMLA / NLRA citation.
- **Proposed discipline without PIP progression escalates.** A
  concern that proposes termination or final-warning without a
  documented PIP trail sets
  `flags: ["proposed_discipline_without_pip_progression"]` and
  escalates — the recommendation cannot stand on its own.
- **Accommodation requests without an interactive-process trail
  escalate** with the ADA Title I (29 CFR §1630.2(o)) citation.
- **Never echo accommodation history** into the summary, even if
  the concern touches accommodation. Accommodation specifics stay
  confidential.
- **Never recommend retaliation.** If the concern names a
  protected activity and the recommended next step would have an
  adverse effect on the reporter, the recommendation is wrong —
  re-derive or escalate.
- **No protected-class analysis** without `eeo_approved_roles` +
  EEO-1 context. See system prompt.
