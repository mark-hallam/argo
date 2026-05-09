# Vertical: recruiter

V2.3 demo vertical (Stream `claude-vertical-recruiter`). Demo #9 —
proves the platform handles the candidate-pipeline funnel with
screening logic, EEOC-compliance guardrails, and a never-AI-extends-
the-offer gate. See also `finance-ar`, `healthcare-intake`,
`real-estate-agent`, `legal-paralegal`.

## Persona

Morgan — technical recruiter for **Skycot Hiring** (synthetic demo
team). Warm, transparent, EEOC-compliant. Morgan applies a pinned
rubric to every resume and routes hiring decisions to the recruiter.

## Action surface

- `screen_candidate` — autonomous-safe. Structured rubric card.
- `send_screening_questions` — **human-gated**. gmail MCP. Body
  drawn from `inventory/question_bank.md`.
- `schedule_interview` — autonomous-safe. gcal MCP. Title carries
  candidate_id only; body never carries rubric score or forbidden
  signals.
- `submit_offer` — **human-gated, ALWAYS**. AI never extends the
  offer itself, regardless of pre-approval.
- `flag_red_flag` — **human-gated**. Routing-only; recruiter decides.

## Memory schema

`candidate_history`, `screening_status`, `interview_loop_state`,
`offer_history`, `red_flag_log`, `screening_emails`. All pointers
— ATS refs, never resume text. Per-criterion rubric scores live in
the audit log, not memory.

## EEOC compliance

The screening rubric (`skills/screening_rubric.md`) is the only
authority for scoring. The forbidden-signals list (age, school
prestige, gender, race, marital status, etc.) is enforced two ways:

1. The system prompt names every forbidden signal explicitly and
   forbids scoring on any of them. "Culture fit" is in the persona's
   forbidden-phrasing list.
2. `crates/argo-cli/tests/vertical_recruiter_assets.rs` asserts the
   prompt contains the EEOC anchor phrase, the rubric forbids each
   signal, and the persona's `avoid_phrasing` includes "culture fit".

## Demo

```sh
./scripts/demo_recruiter.sh
```

Copies the vertical into a tempdir, runs `argo vertical doctor`,
EEOC-greps the doctor output for scoring-language leaks ("score on
age", "weight by school", etc.). Exits non-zero on any leak. No API
keys required.

## Approval gates

Three of five actions are gated. `screen_candidate` and
`schedule_interview` are autonomous-safe.
`workspace/auto_approval.toml` caps daily auto-spend at $1.00 +
$0.05/action. `submit_offer` is gated regardless of operator
override.

The demo's thesis: a platform position ("AI never extends the offer
itself") encoded as a non-overridable gate is the contract that
lets recruiters trust the rest of the funnel.

## Bootstraps via Genesis

Closest match in `crates/argo-genesis/src/archetype.rs` is
`SchedulingCoordinator`. A dedicated `TechnicalRecruiter` archetype
is a v3 candidate. Classify with
`argo genesis classify --charter "..."`.

## Sensible deviations from brief

- `bin/demo_recruiter.sh` → `scripts/demo_recruiter.sh` (matches
  finance-ar / healthcare-intake / real-estate / legal-paralegal /
  sales-sdr / marketing-intern / customer-support convention).
- `vertical_smoke.rs` → `crates/argo-cli/tests/vertical_recruiter_assets.rs`
  (matches `vertical_<id>_assets.rs` per-vertical convention).
