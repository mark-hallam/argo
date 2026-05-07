# V2.3 — Tax Preparation demo vertical

**Owner**: claude. **Depends on**: G-genesis (#790),
claude-vertical-finance-ar (#937),
claude-document-creation-executor merged. Vertical demo #10.

## Scope

Tax Prep Assistant demo vertical — proves the platform
handles seasonal high-stakes financial workflows where the
final filing action is always behind a human gate. Persona
"Dakota" handles tax preparation duties:
- Reviews uploaded W-2s and extracts line items
- Flags potential deductions for client review
- Drafts tax return forms (1040 + schedules) for review
- Schedules client review meetings
- Files returns with the IRS (always behind human approval)

High-value demo because (a) tax prep is the canonical
"high-volume seasonal knowledge work" use case, (b) the
file_return human gate maps cleanly onto the e-file
authorization (8879), (c) shows the platform handling
structured form data alongside narrative client comms.

## What lands

11 NEW files (no NEW crate — re-use existing executors):

- `verticals/tax-prep/vertical.toml` — manifest
  (persona=Dakota, allowed_executors=[gcal,
  document_creation, human_approval])
- `verticals/tax-prep/persona.toml` — voice
  (precise, conservative, never gives tax advice without
  CPA sign-off)
- `verticals/tax-prep/system_prompt.md` — duties, deduction
  rules of thumb, scope limits
- `verticals/tax-prep/memory_schema.toml` —
  client_id, return_year, w2_data, deduction_candidates,
  draft_status, review_log
- `verticals/tax-prep/eval/seeds.jsonl` — 10 scenarios
  (W-2 only / mixed 1099 / itemized vs standard / disputed
  deduction / late-season filing)
- `verticals/tax-prep/inventory/sample_w2s.json` — 5 fake
  W-2 forms (synthetic SSNs e.g. 000-00-0000)
- `verticals/tax-prep/skills/deduction_eligibility.md` —
  common-deduction rubric; explicit "consult CPA" triggers
- `verticals/tax-prep/workspace/auto_approval.toml`
  — review_w2 + schedule_review auto; rest gated
- `verticals/tax-prep/actions/` — 5 action defs:
  - `review_w2.toml` (auto)
  - `flag_deduction_eligibility.toml` (NotifyChannel.Human)
  - `draft_return.toml` (NotifyChannel.DocumentCreation)
  - `schedule_review.toml` (auto)
  - `file_return.toml` (NotifyChannel.Human)
- `bin/demo_tax_prep.sh` — end-to-end demo run
- `docs/verticals/tax-prep.md` — operator doc

## Acceptance criteria
- [ ] Vertical bootstraps via Genesis from charter
- [ ] Demo script runs end-to-end (mocked IRS e-file OK)
- [ ] All 5 actions exercise
- [ ] file_return ALWAYS routes to human approval
- [ ] draft_return produces a markdown 1040 draft
- [ ] No real SSNs in any sample data (regex assertion in
      vertical_smoke.rs)
- [ ] E2E smoke passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Total LOC ≤ 700 across the 11 files

## Branch + PR
- Branch: `claude-vertical-tax-prep`
- PR title: `verticals: Tax prep demo vertical (claude-vertical-tax-prep)`

## Notes
- All sample SSNs MUST be 000-00-0000 or similar invalid
  patterns — vertical_smoke.rs asserts this
- system_prompt MUST disclaim: "I can prepare drafts but a
  licensed preparer must sign and file"
- file_return is mocked via stub executor (no real IRS
  integration in V2)
- Deduction rubric covers ~15 common deductions; the long
  tail is explicitly out of scope and triggers escalation
- 5 sample W-2s sufficient — each is data-rich
