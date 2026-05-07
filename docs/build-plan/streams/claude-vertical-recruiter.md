# V2.3 — Technical Recruiter demo vertical

**Owner**: claude. **Depends on**: G-genesis (#790),
claude-vertical-finance-ar (#937),
C-gcal + C-gmail merged. Vertical demo #9.

## Scope

Technical Recruiter demo vertical — proves the platform
handles candidate pipeline ops with screening logic and
human gates on hiring decisions. Persona "Morgan" handles
sourcing and screening:
- Screens inbound candidate resumes against job criteria
- Sends screening question emails to qualified candidates
- Schedules phone-screen and onsite interviews
- Submits offers (always behind human approval)
- Flags red-flag patterns (employment gaps, fraud signals)
  for recruiter review

High-value demo because (a) recruiting is a high-volume
funnel where automation pays back fast, (b) demonstrates
the platform reasoning over unstructured text (resumes),
(c) the offer-submission gate is a clean example of
"never let the AI make the hiring decision."

## What lands

11 NEW files (no NEW crate — re-use existing executors):

- `verticals/recruiter/vertical.toml` — manifest
  (persona=Morgan, allowed_executors=[gcal, email,
  human_approval])
- `verticals/recruiter/persona.toml` — voice
  (warm, transparent, EEOC-compliant)
- `verticals/recruiter/system_prompt.md` — sourcing duties,
  bias-mitigation rules, scope limits
- `verticals/recruiter/memory_schema.toml` —
  candidate_id, screening_status, interview_loop_state,
  offer_history, red_flag_log
- `verticals/recruiter/eval/seeds.jsonl` — 10 scenarios
  (strong fit / borderline / over-qualified / red flag /
  ghosted candidate)
- `verticals/recruiter/inventory/sample_candidates.json`
  — 10 fake candidate profiles (mix of seniority + skills)
- `verticals/recruiter/skills/screening_rubric.md` —
  job-criteria scoring guide; explicit list of FORBIDDEN
  signals (age, school prestige, etc.)
- `verticals/recruiter/workspace/auto_approval.toml`
  — screen_candidate + schedule_interview auto; rest gated
- `verticals/recruiter/actions/` — 5 action defs:
  - `screen_candidate.toml` (auto)
  - `send_screening_questions.toml` (NotifyChannel.Email)
  - `schedule_interview.toml` (auto)
  - `submit_offer.toml` (NotifyChannel.Human)
  - `flag_red_flag.toml` (NotifyChannel.Human)
- `bin/demo_recruiter.sh` — end-to-end demo run
- `docs/verticals/recruiter.md` — operator doc

## Acceptance criteria
- [ ] Vertical bootstraps via Genesis from charter
- [ ] Demo script runs end-to-end (mocked ATS OK)
- [ ] All 5 actions exercise
- [ ] submit_offer + flag_red_flag correctly route to human
      approval (NEVER auto)
- [ ] screen_candidate emits structured rubric output
- [ ] EEOC-compliance test: screening output does NOT
      reference forbidden signals (assertion in
      vertical_smoke.rs)
- [ ] E2E smoke passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Total LOC ≤ 700 across the 11 files

## Branch + PR
- Branch: `claude-vertical-recruiter`
- PR title: `verticals: Technical recruiter demo vertical (claude-vertical-recruiter)`

## Notes
- screening_rubric.md is the most important skill file —
  it's where bias mitigation lives and where reviewers will
  scrutinize most. Treat it as PRD-grade copy.
- submit_offer is intentionally human-gated even when offer
  parameters are pre-approved by the hiring manager — the
  platform position is "AI never extends the offer itself"
- 10 sample candidates is the right number for a screening
  demo (vs 5 contracts for legal, where each is heavier)
- send_screening_questions uses Email channel; the actual
  questions are pulled from inventory/question_bank.md
