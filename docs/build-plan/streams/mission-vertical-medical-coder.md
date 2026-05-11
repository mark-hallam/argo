# Mission load-bearing wire — Medical-coder vertical (HIPAA path)

**Owner**: claude. **Depends on**: existing vertical-template scaffold (`verticals/legal-paralegal/` brief and `verticals/research-analyst/` MERGED #1337 as canonical citation-grade pattern).
**Authored against**: origin/main `281508ec` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-medical-coder`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #154 ("Medical-coder vertical (HIPAA path)"). **Tier A (vertical).** Mirrors the legal-paralegal UPL-boundary load-bearing pattern but for ICD-10 / CPT / HCPCS coding. Drafts only — coder proposes codes from documentation; **billing submission** routes through human certified-coder review + auto_approval gate. Establishes the HIPAA-aware "PHI in / typed codes out" pattern for the vertical-template system.

## Scope

NEW content-only `verticals/medical-coder/` (11 files, ≤ 500 LOC total — pattern budget: system ≤ 150, persona ≤ 35, skills 4×≤65, schema ≤ 35, auto_approval ≤ 25, inventory ≤ 55, vertical.toml ≤ 55, eval/seed.jsonl ≥ 8 fixtures):

- `vertical.toml` — id=`medical-coder`, version=`0.1.0`, no MCP optional (EHR integrations are separate per-vendor briefs); `[escalation].human_approval_required = ["submit_claim", "amend_submitted_claim", "issue_correction", "release_unredacted_pheno"]`
- `prompts/system.md` — **HIPAA constraint load-bearing**: never echo unredacted PHI back into draft outputs (mask 9-digit SSN-shape, dates-of-birth, phone-pattern, address-pattern); never combine ICD codes from different encounter dates without explicit operator override; cite the documentation passage for every code proposal (line span or page reference); escalation triggers (E&M-level-mismatch / unbundling-risk / billable-without-medical-necessity / hospice-coding / DME / Medicare-Part-B-specific)
- `prompts/persona.md` — Sage, CPC-certified-shape (drafts only; "I propose codes; I never submit"); methodical, never speculates beyond the documentation
- 4 skills:
  - `propose_icd10` — encounter documentation in; proposed ICD-10-CM codes + sequencing-rationale + per-code documentation-citation out
  - `propose_cpt` — procedure documentation in; proposed CPT codes + modifiers + RVU estimate (advisory) + per-code documentation-citation out
  - `audit_existing_claim` — submitted claim + documentation in; per-line code-vs-documentation mismatch report + unbundling flags + medical-necessity gaps out
  - `redact_phi` — raw documentation in; redacted version w/ tagged-span list of removed PHI elements (per HIPAA Safe Harbor 18 categories) out
- `memory/schema.toml` — `payer_rules` (per-payer CMS / commercial coding-policy snippets), `cited_authorities` (CMS NCD / LCD / coding clinic Q&A cache), `firm_style_guide` (sequencing preferences), `denied_codes_history` (per-payer denial reasons we've seen)
- `eval/seed.jsonl` — 8+ fixtures: 2 clean ICD-10 proposals (one chronic / one acute), 1 CPT modifier-25 case, 1 audit-existing-claim finding 2 unbundling violations, 1 redact_phi w/ all 18 categories present, 3 escalation (Medicare-hospice / unbundling-risk-explicit / billable-without-medical-necessity)
- `auto_approval.toml` — wake list includes propose_icd10 + propose_cpt + audit_existing_claim + redact_phi only; submit/amend/correct/release_unredacted intentionally absent
- `inventory.json` — sample `firm_style_guide` (sequencing rules + modifier preferences); sample `payer_rules` (1 CMS rule + 1 commercial rule, synthetic); sample `denied_codes_history` (5 entries, synthetic)

NO Rust. NO migrations. NO crate changes. Pattern is identical to the just-merged research-analyst (#1337) + the queued legal-paralegal brief — re-use the file shapes 1:1.

This is NOT in scope: actual EHR connectors (Epic / Cerner / Athena are separate per-vendor briefs); 837P/837I claim-file generation (separate); RVU calculator deep-detail (advisory-only in v1); E&M-level decision-trees (the brief uses a simple "documentation supports level X" rule, not the full CMS table); HIPAA business-associate agreement automation (separate compliance brief); actual PHI-detection ML (the regex-based PHI redaction in `redact_phi` is the v1 surface — production should use a dedicated PHI-detection MCP).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/medical-coder/vertical.toml` (NEW, ≤ 55 LOC)
- `verticals/medical-coder/prompts/system.md` (NEW, ≤ 150 LOC)
- `verticals/medical-coder/prompts/persona.md` (NEW, ≤ 35 LOC)
- `verticals/medical-coder/skills/propose_icd10.md` (NEW, ≤ 65 LOC)
- `verticals/medical-coder/skills/propose_cpt.md` (NEW, ≤ 65 LOC)
- `verticals/medical-coder/skills/audit_existing_claim.md` (NEW, ≤ 65 LOC)
- `verticals/medical-coder/skills/redact_phi.md` (NEW, ≤ 65 LOC)
- `verticals/medical-coder/memory/schema.toml` (NEW, ≤ 35 LOC)
- `verticals/medical-coder/eval/seed.jsonl` (NEW, ≥ 8 fixtures)
- `verticals/medical-coder/auto_approval.toml` (NEW, ≤ 25 LOC)
- `verticals/medical-coder/inventory.json` (NEW, ≤ 55 LOC)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory
- Any frontend / web crate

## Acceptance criteria

- [ ] `argo vertical list` includes `medical-coder 0.1.0`
- [ ] `argo vertical lint medical-coder` exits 0 errors (warns of the same shape as research-analyst / legal-paralegal baseline are acceptable — vertical-specific approval-gated action names are not built-in tool names)
- [ ] `argo vertical show medical-coder --json` returns valid envelope (id / name / version / persona / memory_entries / eval_seed_count ≥ 8 / vertical_toml)
- [ ] All 4 skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] System prompt names HIPAA explicitly and binds drafts-only behavior to it (mirroring the UPL pattern in legal-paralegal)
- [ ] `redact_phi` skill output includes the 18 HIPAA Safe Harbor categories tagged-span list (document the list in the skill file)
- [ ] `eval/seed.jsonl` has ≥ 8 fixtures, ≥ 3 of which are escalation cases (Medicare-hospice / unbundling-risk / billable-without-medical-necessity)
- [ ] `auto_approval.toml` wake list contains only the 4 propose/audit/redact skills; submit/amend/correct/release_unredacted gated through `[escalation].human_approval_required`
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-medical-coder`
- PR title: `vertical: medical-coder (HIPAA + ICD-10/CPT drafts-only) (mission-vertical-medical-coder)`

## Notes for the implementer

- Treat the just-merged `verticals/research-analyst/` (#1337) and the queued `verticals/legal-paralegal/` brief as 1:1 templates — same file shape, same YAML frontmatter, same `[escalation]` block style. The HIPAA pattern in this vertical mirrors UPL in legal-paralegal: name the constraint explicitly, bind drafts-only behavior to it.
- The 18 HIPAA Safe Harbor identifiers are: names, geographic subdivisions smaller than state, dates (other than year), phone, fax, email, SSN, MRN, health-plan #, account #, certificate/license #, vehicle ID / serial #, device ID / serial #, URLs, IP addresses, biometric IDs, full-face photographs, and any other unique identifying number / characteristic / code. The `redact_phi` skill should call out each tagged span by category.
- ICD-10-CM is the diagnosis-code system (US 2015+). CPT is procedure-codes. The `propose_icd10` and `propose_cpt` skills are intentionally separate because the documentation analysis is different (Dx looks for diagnoses + signs/symptoms; CPT looks for procedures + supplies).
- "Sequencing" matters in ICD-10 — primary Dx vs secondary. The skill should propose ORDERED codes with rationale. Eval fixtures should check both the codes AND the order.
- Unbundling is the practice of billing multiple codes when one bundled code would apply — Medicare's biggest audit target. `audit_existing_claim` should flag the National Correct Coding Initiative (NCCI) pair violations as the canonical unbundling-risk type.
- DO NOT add real ICD-10 / CPT code tables to the vertical — those are licensed datasets (AMA owns CPT) and shipping them in inventory is a license violation. The vertical refers to codes generically; the calling LLM uses its training-data knowledge of common codes; production users would wire in a real coding-database MCP separately.
- The "amend_submitted_claim" approval-gated action is load-bearing — once a claim is submitted, edits trigger CMS audit attention. The vertical's escalation must require human review.
- Two escalation fixtures should exercise the unbundling boundary specifically: one where a draft proposes 3 unbundled codes (refuse + escalate w/ NCCI-pair explanation) and one where existing-claim has 2 unbundled (audit_existing_claim returns the findings, doesn't auto-correct).
