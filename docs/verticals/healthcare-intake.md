# Vertical: healthcare-intake

V2.3 demo vertical (Stream `claude-vertical-healthcare-intake`).
Demo #7 — the heaviest gate profile in the catalogue, with 4 of 5
actions running through the human approval gate. See also
`finance-ar` (#937), `customer-support`, `sales-sdr`,
`marketing-intern`, `marketplace-seller`.

## Persona

Avery — front-desk intake coordinator for **Birchwood Family
Practice** (synthetic demo clinic). Warm, reassuring, plain-language.
Avery is not a clinician and never weighs symptoms; the job is to
route every clinical decision to the right human while handling the
schedule + paperwork + payer plumbing.

## Action surface

- `schedule_appointment` — autonomous-safe. Books a calendar slot via
  gcal MCP. Event title carries the patient_id only; never a diagnosis.
- `verify_insurance` — **human-gated**. Mocked payer lookup. Operator
  approves coverage / copay / deductible before they land in memory.
- `send_prep_instructions` — **human-gated**. Patient-facing email via
  gmail MCP. Stricter operators can flip notify_channel to Human.
- `flag_risk_factor` — **human-gated**. Routing-only — hands an
  allergy / contraindication / red-flag case to the on-call clinician.
- `update_patient_record` — **human-gated**. Non-clinical corrections
  only. Clinical-edit requests refuse and escalate.

## Memory schema

`patient_history`, `appointment_history`, `insurance_status`,
`risk_flags`, `prep_sends`, `record_updates`. All pointers — EMR
refs, never free-text clinical notes. See `memory_schema.toml`.

## PHI handling

Every patient identifier is treated as PHI. The OS log boundary is
INFO — anything that would expose PHI lands at DEBUG or below. The
demo script's PHI grep is the operational floor; production deploys
layer real audit sinks (`crates/argo-audit`) on top. All sample data
is **synthetic** — masked member ids, `pt####@example.com` emails,
555-010-#### fictional phones. No real PHI in repo.

## Demo

```sh
./scripts/demo_healthcare_intake.sh
```

The demo copies the vertical into a tempdir workspace, runs
`argo vertical doctor`, and PHI-greps the doctor scan output. The
script exits non-zero if any synthetic-PHI string surfaces at INFO
level. No external API keys required.

## Approval gates

Four of five actions are gated. `schedule_appointment` is the single
autonomous-safe action. `workspace/auto_approval.toml` caps daily
auto-spend at $1.00 total + $0.05/action.

The demo's thesis: a **heavily-gated workflow still works cleanly**
under the executor trust model. The human gate is the contract that
keeps PHI out of any autonomously-triggered path.

## Bootstraps via Genesis

No dedicated `HealthcareIntake` archetype yet — the closest match in
`crates/argo-genesis/src/archetype.rs` is `CalendarCoordinator` (slot
booking is the dominant action). A dedicated archetype is a v3
candidate. Classify with `argo genesis classify --charter "..."`.

## Deferred follow-up (out of scope)

- Real payer integrations (837 / 270 / 271). Mocked fixture today.
- Real EMR connectors (Epic, Cerner, Athenahealth). EMR ref string
  only; write path not wired.
- HIPAA-compliant audit sink. PHI grep is the floor.
- Dedicated `HealthcareIntake` Genesis archetype.
