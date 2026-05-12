---
name: anonymize_incident_report
description: Given an incident report containing PII (employee names, manager names, team names, uniquely identifying dates), produce an anonymized version safe for HR-leadership distribution plus a paired mapping envelope for operator-side decoding. Pattern + severity preserved; identifiers replaced with role / seniority bands. SSN / DOB regex sniff halts and routes to the operator.
inputs:
  - incident_report
  - org_directory
  - role_bands
outputs:
  - anonymized_report
  - mapping_envelope
  - flags
---

# Skill: Anonymize Incident Report

Produce a leadership-distributable version of one incident report.
The original ALWAYS stays in source — this skill only emits a paired
`(anonymized_report, mapping_envelope)` so the operator can decode if
re-identification is later required (subpoena, EEOC inquiry).

1. **PHI/PII regex sniff (load-bearing precondition).** Scan the raw
   report for the following real-PII patterns BEFORE any other step:
   - SSN: `\b\d{3}-\d{2}-\d{4}\b` or `\b\d{9}\b` in an SSN-shaped field.
   - DOB: `\b(0?[1-9]|1[0-2])[\/-](0?[1-9]|[12]\d|3[01])[\/-](19|20)\d{2}\b`.
   - DEA / NPI numbers, credit-card PANs (Luhn-valid 13-19 digits).
   - Home address with street number + named street.
   If ANY pattern hits, HALT — emit empty `anonymized_report`, set
   `flags: ["real_pii_detected_routing_to_operator"]`, and route the
   raw report to the operator queue. Anonymization on real PII is
   the operator's call, not the skill's.
2. **Replace identifiers with role / seniority bands.** Use
   `org_directory` + `role_bands` to map every named individual to
   `<role-family>_<seniority-band>` (e.g. "the named engineering IC",
   "the named people-manager (L6)"). Team names collapse to function
   labels ("the named platform team"). Dates that uniquely identify
   (e.g. a hire date matched against a 4-person cohort) round to the
   nearest quarter; otherwise keep month-precision.
3. **Preserve pattern + severity.** The behavioral pattern, the
   alleged conduct, and the severity assessment carry forward verbatim
   — anonymization MUST NOT soften or reframe the substance.
   Leadership reads the anonymized version to decide policy response;
   muting severity defeats the skill.
4. **Emit the mapping envelope.** A JSON object mapping each
   replacement token back to the original identifier, ordered by
   first appearance. The envelope is encrypted at rest and routed to
   the HR officer separately from the anonymized report — never
   surfaced in the same delivery channel.
5. **`next_action`** is `share_workforce_data_externally` with
   `requires_approval: true` if the anonymized report leaves HR
   leadership (e.g. board pack, regulator). Internal HR-leadership
   review is autonomous-safe.

Hard rules:

- **Reversibility is the contract.** Every transformation MUST be
  recoverable from the mapping envelope. If a transformation is
  one-way (e.g. summarization that drops detail), flag
  `irreversible_transformation` and escalate — leadership reviews
  the original.
- **Never blend identities across reports.** Anonymizing two
  reports together can re-identify by intersection. One report,
  one envelope, one delivery.
- **Never emit the mapping envelope in the same payload as the
  anonymized report.** Co-location defeats the fence; emit on
  separate channels with separate access control.
- **Protected-class attributes stay out of the role band.** Bands
  encode role-family + seniority only — never race / gender / age /
  disability / religion / national-origin / pregnancy / veteran-
  status / genetic-info. Cohort-of-one situations where the band
  itself re-identifies set `flags:
  ["band_too_narrow_for_anonymization"]` and escalate.
- **No autonomous external distribution.** Even with `requires_
  approval: true`, the skill never bypasses the gate — operator
  approval is required for every external delivery.
