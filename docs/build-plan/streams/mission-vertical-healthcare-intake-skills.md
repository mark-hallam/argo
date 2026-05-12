# Mission load-bearing wire — Healthcare Intake vertical: 5+ skills

**Owner**: claude. **Depends on**: existing `verticals/healthcare-intake/` charter stub.
**Authored against**: origin/main `d866e01c` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-healthcare-intake-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to healthcare-intake. Today the vertical ships with 1 skill (`triage_rubric.skill.md`). MISSION claim 7 expects parity-with-gtm-engineer. This brief brings healthcare-intake to 6 skills. **PHI safety is the load-bearing constraint** — every skill enforces synthetic-data-only and human-gating on PHI surfaces.

## Scope

5 NEW skill files under `verticals/healthcare-intake/skills/`. Skills cover the dominant intake flow not already covered by `triage_rubric`:

1. **`schedule_appointment.skill.md`** — given a patient request + clinician availability (operator-provided), propose 3 slot options factoring appointment-type duration, no-back-to-back rule (10min buffer), provider preferences. Output JSON. Approval-gated before booking.
2. **`verify_insurance_benefits.skill.md`** — given patient + insurance details, query the mocked payer API + return: in-network / coverage / deductible-remaining / copay / referral-required. Marks output `mock_data: true` until a real payer connector lands. Approval-gated before patient communication.
3. **`send_pre_visit_prep.skill.md`** — given appointment-type, draft a pre-visit prep message (what to bring, fasting requirements, parking, what to expect). Templates by appointment-type; no fabrication of clinic-specific policy. Approval-gated.
4. **`flag_risk_factors.skill.md`** — given patient intake answers + history, flag patterns that need clinician pre-review: medication interactions, recent ER visits, high-risk conditions, abnormal vitals. Output: structured flags with severity (info / advisory / urgent). Never makes clinical judgements — only surfaces for clinician review.
5. **`update_patient_record.skill.md`** — log intake-flow actions to `memory.patient_record[]`: appointment booked, prep sent, insurance verified, flags raised. Includes audit trail. PHI-fence: synthetic data only; if real PHI patterns detected (SSN regex, real DOB+name combos), halt and emit `phi_detected: true` flag for human review.

Each skill ~50-90 LOC. Total: ~350-450 LOC.

This is NOT in scope: real payer API integration; real EHR write; PHI scrubber/de-identifier (separate brief); HIPAA BAA paperwork; clinical decision support (out of scope — this is intake, not diagnosis).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/healthcare-intake/skills/schedule_appointment.skill.md` (NEW, ≤ 90 LOC)
- `verticals/healthcare-intake/skills/verify_insurance_benefits.skill.md` (NEW, ≤ 90 LOC)
- `verticals/healthcare-intake/skills/send_pre_visit_prep.skill.md` (NEW, ≤ 80 LOC)
- `verticals/healthcare-intake/skills/flag_risk_factors.skill.md` (NEW, ≤ 90 LOC)
- `verticals/healthcare-intake/skills/update_patient_record.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- `verticals/healthcare-intake/skills/triage_rubric.skill.md` — existing skill stable
- `verticals/healthcare-intake/vertical.toml` — schema is fine
- `verticals/healthcare-intake/persona.toml`, `system_prompt.md` — locked
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 5 new `.skill.md` files
- [ ] Each has frontmatter: `name`, `version: "0.1.0"`, `description`, `provenance: { source: vertical/healthcare-intake, hash: sha256-pending }`
- [ ] Each has sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`, `# PHI safety`
- [ ] Every skill includes a `# PHI safety` section reaffirming synthetic-data-only + approval-gating
- [ ] `verify_insurance_benefits` output marks `mock_data: true` until a real payer connector lands
- [ ] `flag_risk_factors` uses 3-level severity (info / advisory / urgent) and explicitly does NOT make clinical judgements
- [ ] `update_patient_record` includes a PHI-regex sniff (SSN pattern, etc.) and halts on detection
- [ ] No skill auto-sends to a patient or auto-writes to an EHR
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-healthcare-intake-skills`
- PR title: `verticals: 5 skills for healthcare-intake (mission-vertical-healthcare-intake-skills)`

## Notes for the implementer

- Match the tone of `verticals/healthcare-intake/persona.toml` (read it first). Avery is described as careful + non-clinical — skills should reinforce both.
- The charter's "synthetic patient data only; no real PHI in repo" rule is non-negotiable. Document it in every skill's `# PHI safety` section. If a skill could plausibly receive real PHI in production, the skill should refuse and emit `phi_detected: true`.
- `flag_risk_factors` is the most subtle skill — it must NOT make clinical claims. Phrasing like "patient has uncontrolled hypertension" is wrong; "blood pressure values trend high; clinician review recommended" is right. The 3 severity levels (info / advisory / urgent) are about workflow routing, not medical urgency.
- `verify_insurance_benefits` uses a mocked payer in v1. The `mock_data: true` flag is the seam — when a real payer connector lands, the flag flips, but the skill output shape stays stable.
- For `send_pre_visit_prep` — never fabricate clinic-specific policy. If asked about parking or pre-visit fasting and the answer isn't in the operator-provided template, the skill says "I don't have that info; please advise the patient to call the front desk" and routes back to the operator.
- The PHI-fence in `update_patient_record` should reuse any existing PII redaction patterns from `crates/argo-redact/` if those exist; otherwise document the regex set inline (SSN: `\b\d{3}-\d{2}-\d{4}\b`, etc.).
