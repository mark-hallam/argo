---
name: propose_icd10
description: Given encounter documentation, propose ordered ICD-10-CM Dx codes with sequencing rationale and per-code documentation citation. Drafts only; never submits.
inputs:
  - encounter_documentation
  - encounter_date
  - patient_age_band
outputs:
  - proposed_codes
  - sequencing_rationale
  - flags
  - confidence
---

# Skill: Propose ICD-10-CM Diagnosis Codes

Given encounter documentation for a single encounter date, propose
the diagnosis codes the supervising certified coder should consider.

1. **Proposed codes** — ordered list. Primary Dx first (the condition
   chiefly responsible for the encounter as documented in the
   chief-complaint / assessment), then secondary Dx (co-existing
   conditions that affect treatment or management). For each code:
   - `code` — ICD-10-CM (e.g. `E11.9`, `J18.9`); name the code system.
   - `description` — official ICD-10-CM short description.
   - `citation` — `[doc: <doc_id-or-page>, span: L<start>-L<end>,
     date_accessed: <ISO>]` pointing to the supporting documentation.
   - `evidence_class` — `documented` (named explicitly), `implied`
     (signs / symptoms support; Dx not named), or `history` (prior
     encounter, requires operator override to carry forward).

2. **Sequencing rationale** — one paragraph naming why primary is
   primary, citing the chief-complaint span. If sequencing depends
   on the payer's policy, name the payer rule.

3. **Confidence** — high (every code is `documented` and supported by
   an explicit line span), medium (one or more `implied`), low (any
   `history` carry-forward without operator override).

Hard rules:

- Never propose a code without a documentation citation. If a code
  has no supporting span, flag it
  `[unsupported by documentation — flagged for coder review]` and
  drop it from `proposed_codes`.
- Never combine ICD codes from different encounter dates. If the
  documentation spans multiple dates, propose codes per-date and
  surface the cross-date aggregation as a flag for operator review.
- Hospice-related Dx (`Z51.5`, `Z51.81` in hospice context) routes
  to the supervising coder — do not auto-draft. Set
  `flags: ["hospice_coding"]` and escalate.
- "Rule out" / "suspected" Dx without confirmatory documentation in
  the same encounter are coded to the highest level of certainty
  the documentation supports (often signs / symptoms), not to the
  unconfirmed Dx. Cite the rule.
- If the operator requests a Dx code that the documentation does not
  support, flag the mismatch as a potential upcoding risk — do not
  silently propose. Surface both views and route to the coder.

I propose codes but never submit — the certified coder signs off
before anything goes to a payer.
