# V2.3 — Healthcare Intake demo vertical

**Owner**: claude. **Depends on**: G-genesis (#790),
claude-vertical-finance-ar (#937),
C-gcal merged. Vertical demo #7.

## Scope

Healthcare Intake Coordinator demo vertical — proves the
platform handles HIPAA-adjacent flows where every action
that touches PHI requires human gate. Persona "Avery"
handles patient intake duties:
- Schedules appointments based on provider availability
- Verifies insurance coverage and benefits (mocked payer)
- Sends pre-visit prep instructions to patients
- Flags risk factors (allergies, contraindications) for
  clinician review
- Updates patient records (always behind human approval)

High-value demo because (a) every clinic has intake
overhead, (b) demonstrates that almost-all-gated workflows
still work cleanly under the executor trust model, (c) the
single auto-approved action (schedule_appointment) shows
the platform isn't all-or-nothing.

## What lands

11 NEW files (no NEW crate — re-use existing executors):

- `verticals/healthcare-intake/vertical.toml` — manifest
  (persona=Avery, allowed_executors=[gcal, email, human_approval])
- `verticals/healthcare-intake/persona.toml` — voice
  (warm, reassuring, avoids medical claims)
- `verticals/healthcare-intake/system_prompt.md` — intake
  duties, PHI handling, escalation rules
- `verticals/healthcare-intake/memory_schema.toml` —
  patient_id, appointment_history, insurance_status,
  risk_flags (NO clinical data; pointers only)
- `verticals/healthcare-intake/eval/seeds.jsonl` — 10
  scenarios (new patient / returning / insurance gap /
  drug allergy / urgent triage)
- `verticals/healthcare-intake/inventory/sample_patients.json`
  — 5 fake patient records (synthetic — no real PHI)
- `verticals/healthcare-intake/skills/triage_rubric.md` —
  red-flag pattern guidance for clinician escalation
- `verticals/healthcare-intake/workspace/auto_approval.toml`
  — only schedule_appointment auto; everything else gated
- `verticals/healthcare-intake/actions/` — 5 action defs:
  - `schedule_appointment.toml` (auto)
  - `verify_insurance.toml` (NotifyChannel.Human)
  - `send_prep_instructions.toml` (NotifyChannel.Email)
  - `flag_risk_factor.toml` (NotifyChannel.Human)
  - `update_patient_record.toml` (NotifyChannel.Human)
- `bin/demo_healthcare_intake.sh` — end-to-end demo run
- `docs/verticals/healthcare-intake.md` — operator doc

## Acceptance criteria
- [ ] Vertical bootstraps via Genesis from charter
- [ ] Demo script runs end-to-end (mocked payer/EMR OK)
- [ ] All 5 actions exercise
- [ ] verify_insurance + flag_risk_factor + update_record
      correctly route to human approval (3 gates fire)
- [ ] No PHI is logged at INFO level (audit-grep test in
      vertical_smoke.rs)
- [ ] E2E smoke passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Total LOC ≤ 700 across the 11 files

## Branch + PR
- Branch: `claude-vertical-healthcare-intake`
- PR title: `verticals: Healthcare intake demo vertical (claude-vertical-healthcare-intake)`

## Notes
- All sample data is SYNTHETIC. No real PHI in repo.
- send_prep_instructions uses Email NotifyChannel (not
  Human) because it's templated, low-risk patient comms;
  approval policy can flip to Human for stricter operators
- schedule_appointment is the single auto path — proves the
  pattern that high-volume low-risk slots still benefit from
  automation even in heavily-gated verticals
- This vertical does NOT integrate any real EMR/payer
  systems. V3 candidate: Epic/Cerner connectors.
