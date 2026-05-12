---
name: prep_appeal_letter
description: Given a denied claim, the payer's denial reason, and supporting clinical evidence, draft a templated appeal letter citing medical-necessity criteria, clinical evidence, and the relevant payer-policy paragraph. Draft only; sending the appeal is approval-gated.
inputs:
  - denied_claim
  - denial_reason
  - clinical_evidence
  - payer_policy_excerpt
  - payer_template
outputs:
  - appeal_letter_markdown
  - cited_authorities
  - flags
  - confidence
---

# Skill: Prepare Appeal Letter

Given a denied claim plus the denial reason and supporting clinical
evidence, draft a templated appeal letter for the supervising coder
to review and send. Output is Markdown; sending the appeal is the
approval-gated `submit_appeal` action, not this skill.

## Output

1. **Appeal letter** — Markdown. Sections, in order:
   - Header — payer name, payer claim ID, date of service, appeal
     date. Patient referenced by synthetic patient ID only.
   - Denial summary — quote the denial reason verbatim with
     `[denial: <denial_id>, line: <line_no>]`.
   - Medical-necessity argument — restate the clinical indication,
     citing chart spans `[doc: <doc_id-or-page>, span:
     L<start>-L<end>, date_accessed: <ISO>]`.
   - Policy citation — quote the operator-supplied payer policy
     paragraph verbatim with `[policy: <policy_id>, paragraph:
     <para_no>]`. If no policy excerpt was supplied, flag
     `[payer policy lookup required]` and stop drafting.
   - Requested relief — overturn / partial-pay / reprocess.
   - Signature block — `<supervising-coder-name>, <credential>`
     placeholder; never auto-fill.

2. **Cited authorities** — flat list of every `[doc:]`, `[denial:]`,
   `[policy:]` token used in the letter. Each entry: `kind`, `id`,
   `excerpt`. Established CMS guidance may be cited with attribution
   (`source: CMS, document: <name>, section: <section>`).

3. **Confidence** — high (every claim cites a span + policy
   paragraph), medium (medical-necessity supported but policy
   paragraph not supplied), low (heuristic phrasing dominates).

# Hard rules

- Approval-gated. This skill drafts only; sending the appeal is the
  approval-gated `submit_appeal` action and routes to the supervising
  coder for signature.
- Never fabricate regulatory citations. Cite only operator-supplied
  policy text or established CMS guidance with full attribution.
  Missing payer policy → flag and stop, do not paraphrase.
- Match the operator-supplied `payer_template` format when one is
  provided; payer-specific headers / claim-id fields take precedence
  over the default template.
- Tone is factual and clinical, never persuasive beyond the evidence.
  Never restate clinical conclusions the documentation does not
  support — that is itself an upcoding risk.

# PHI safety

- Synthetic data only. The patient is referenced by synthetic patient
  ID throughout; no Safe Harbor identifier appears in the letter body.
- Clinical evidence is excerpted by chart span citation, not pasted
  in raw. Production wiring runs the evidence through `redact_phi`
  before this skill sees it.
- Releasing the unredacted appeal (e.g. to payer portal) is the
  approval-gated `submit_appeal` action; until then the draft stays
  inside the authorized envelope.

I draft appeals but never send — the certified coder signs and
submits after reviewing the evidence and cited authorities.
