---
name: redact_phi
description: Given raw clinical documentation, produce a redacted version with a tagged-span list of removed PHI elements per the 18 HIPAA Safe Harbor categories. Drafts only; releasing unredacted PHI routes through approval gate.
inputs:
  - raw_documentation
  - source_doc_id
outputs:
  - redacted_text
  - safe_harbor_spans
  - flags
  - confidence
---

# Skill: Redact PHI (HIPAA Safe Harbor)

Given raw clinical documentation, produce a redacted version that
removes the 18 HIPAA Safe Harbor identifiers (45 CFR 164.514(b)(2)),
along with a tagged-span list of every element removed. v1 is regex-
based; production should wire a dedicated PHI-detection MCP.

## The 18 HIPAA Safe Harbor identifiers

1. `name` — patient, relatives, employers, household members.
2. `geo_subdivision` — smaller than state (street, city, county, ZIP).
   First three ZIP digits may be retained if population > 20,000.
3. `date` — birth, admission, discharge, death (year may be retained;
   ages > 89 aggregate to "≥ 90").
4. `phone`. 5. `fax`. 6. `email`. 7. `ssn` (`###-##-####`).
8. `mrn`. 9. `health_plan_number`. 10. `account`.
11. `cert_or_license`. 12. `vehicle_id` (incl. plate). 13. `device_id`.
14. `url`. 15. `ip`. 16. `biometric`. 17. `face_photo`.
18. `other_unique_id` — any other unique identifying number,
    characteristic, or code.

## Output

1. **Redacted text** — input with every matched span replaced by a
   category tag, e.g. `[REDACTED:name]`, `[REDACTED:mrn]`.

2. **Safe harbor spans** — list of every redaction. For each:
   - `category` — one of the 18 above.
   - `original_span` — `L<start>-L<end>` line range.
   - `redaction_token` — replacement written into redacted text.
   - `confidence` — `high` (regex match + contextual cue), `medium`
     (regex only), `low` (heuristic).

3. **Confidence** (overall) — high (every span `high` per-category),
   medium (one or more `low`), low (heuristic matches dominate).

Hard rules:

- Release of unredacted PHI requires the approval-gated
  `release_unredacted_phi` action. Refuse otherwise; surface the
  trigger phrase verbatim. Redacted output may be released;
  raw documentation may not.
- v1 redaction is regex-based and will miss free-text identifiers
  (e.g. "the patient's mother, Mrs. Smith"). Set
  `flags: ["regex_only_v1"]` on every output; recommend a dedicated
  PHI-detection MCP for production.
- Ages ≥ 90 aggregate to "≥ 90"; never preserve the exact age.

I redact for draft release but never release unredacted PHI — the
certified coder signs off before any raw record leaves the
authorized envelope.
