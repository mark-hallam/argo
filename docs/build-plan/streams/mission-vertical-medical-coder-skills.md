# Mission load-bearing wire — Medical-Coder vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/medical-coder/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-medical-coder-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. medical-coder ships with 4 skills (audit_existing_claim / propose_cpt / propose_icd10 / redact_phi). Brief brings to 6. **PHI safety load-bearing** — synthetic-data-only enforcement.

## Scope

2 NEW skill files:

1. **`detect_upcoding_risk.skill.md`** — given a claim + the underlying clinical notes, flag patterns suggesting upcoding (CPT/ICD10 selected at higher reimbursement than the documentation supports). Output JSON: `{ codes_at_risk: [{code, reason, evidence_quote, severity: info|advisory|urgent}], recommended_action: send_as_is | downcode_to_<X> | request_clarification_from_provider }`. **Never auto-downcodes.** Always routes urgent severity to operator.
2. **`prep_appeal_letter.skill.md`** — given a denied claim + denial reason + supporting clinical evidence, draft a templated appeal letter citing applicable medical-necessity criteria + clinical evidence + relevant payer-policy paragraph. Output Markdown. Approval-gated. References the patient by synthetic ID only.

## Files you may touch

- `verticals/medical-coder/skills/detect_upcoding_risk.skill.md` (NEW, ≤ 90 LOC)
- `verticals/medical-coder/skills/prep_appeal_letter.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] Each has a `# PHI safety` section reaffirming synthetic-data-only + approval-gating
- [ ] `detect_upcoding_risk` 3-severity vocab; never auto-downcodes
- [ ] `prep_appeal_letter` references payer-policy paragraph by citation
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-medical-coder-skills`
- PR title: `verticals: 2 skills for medical-coder (mission-vertical-medical-coder-skills)`

## Notes

- Upcoding detection is a compliance-critical pattern. Document in `# Hard rules`: "This skill never auto-downcodes. It flags. Operators downcode after reviewing the evidence."
- Appeal-letter templates should match payer-specific format requirements when known (operator-provided templates). Avoid fabricating regulatory citations — only cite operator-supplied policy text or established CMS guidance with attribution.
