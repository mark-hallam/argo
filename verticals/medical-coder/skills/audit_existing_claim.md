---
name: audit_existing_claim
description: Given a submitted claim plus the underlying documentation, produce a per-line code-vs-documentation mismatch report with unbundling flags and medical-necessity gaps. Findings only; never auto-corrects.
inputs:
  - submitted_claim
  - encounter_documentation
  - payer_id
outputs:
  - audit_findings
  - unbundling_flags
  - medical_necessity_gaps
  - flags
  - confidence
---

# Skill: Audit Existing Claim

Given a submitted claim (CMS-1500 / UB-04 / 837P / 837I line items)
and the underlying encounter documentation, produce a finding
report. Read-only — never proposes corrections, amends, or
adjustments.

1. **Audit findings** — per-claim-line table. For each line:
   - `line_no`, `submitted_code`.
   - `documentation_citation` — `[doc: <doc_id-or-page>, span:
     L<start>-L<end>, date_accessed: <ISO>]` or
     `[no supporting documentation]`.
   - `mismatch_class` — `supported` / `over_documented` (doc
     supports different code, same complexity) / `under_documented`
     (doc supports lower-complexity code — potential upcoding) /
     `unsupported`.
   - `note` — one-sentence summary.

2. **Unbundling flags** — list of NCCI (National Correct Coding
   Initiative) pair violations. For each:
   - `pair` — two CPT codes forming the bundle.
   - `canonical_code` — single bundled code the payer expects.
   - `policy_citation` — NCCI quarterly edit or payer-specific rule.
   Findings only; never auto-merges. Corrections route to the
   supervising coder via approval-gated `issue_correction`.

3. **Medical-necessity gaps** — lines where the procedure is
   technically billable but documentation does not establish the
   medical necessity required by the payer's LCD / NCD. Cite the
   LCD / NCD by name from `cited_authorities` or flag
   `[LCD / NCD lookup required — flagged for coder review]`.

4. **Confidence** — high (every line maps to an explicit span, no
   lookup pending), medium (one or more lookups pending), low
   (multiple `unsupported` or `under_documented`).

Hard rules:

- Audit is observations, not corrections. Never emits an amendment,
  never calls `amend_submitted_claim` / `issue_correction`.
- Matching `denied_codes_history` row → surface prior denial reason
  as context. Do not assume prior denial recurs.
- Hospice / DME / Medicare Part B audit findings escalate per the
  system-prompt rules.

I audit but never correct — the certified coder signs off before
any adjustment goes to the payer.
