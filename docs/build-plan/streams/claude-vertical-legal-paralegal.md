# V2.3 — Legal Paralegal demo vertical

**Owner**: claude. **Depends on**: G-genesis (#790),
claude-vertical-finance-ar (#937),
claude-document-creation-executor merged. Vertical demo #6.

## Scope

Legal Paralegal demo vertical — proves the platform claim
"any role" extends into regulated knowledge work. Persona
"Quinn" handles routine paralegal duties under attorney
supervision:
- Reviews incoming contracts (NDAs, MSAs, SOWs) for standard
  clauses
- Flags concerning clauses (liability, indemnity, IP) for
  attorney review
- Drafts amendment language for negotiated changes
- Files documents with court systems (mocked)
- Generates plain-language summaries for clients

High-value demo because (a) legal work is high-margin, (b)
the human-approval gates demonstrate the platform's
trust-tiering (auto vs. NotifyChannel.Human), (c) shows
document_creation executor in a non-finance context.

## What lands

11 NEW files (no NEW crate — re-use existing executors):

- `verticals/legal-paralegal/vertical.toml` — manifest
  (persona=Quinn, allowed_executors=[email, document_creation,
  human_approval])
- `verticals/legal-paralegal/persona.toml` — voice
  (precise, formal, avoids legal advice; defers to attorney)
- `verticals/legal-paralegal/system_prompt.md` — duty
  description, escalation rules, scope limits
- `verticals/legal-paralegal/memory_schema.toml` —
  matter_id, contract_metadata, clause_history,
  attorney_review_log
- `verticals/legal-paralegal/eval/seeds.jsonl` — 10
  paralegal scenarios (clean NDA / risky MSA / routine
  SOW / disputed IP clause / urgent court filing)
- `verticals/legal-paralegal/inventory/sample_contracts.json`
  — 5 fake contracts (NDA, MSA, SOW, employment, lease)
- `verticals/legal-paralegal/skills/clause_review.md` —
  skill markdown for clause-pattern recognition
- `verticals/legal-paralegal/workspace/auto_approval.toml`
  — review_contract + generate_summary auto; rest gated
- `verticals/legal-paralegal/actions/` — 5 action defs:
  - `review_contract.toml` (auto)
  - `flag_clause_concern.toml` (NotifyChannel.Human)
  - `draft_amendment.toml` (NotifyChannel.DocumentCreation)
  - `file_with_court.toml` (NotifyChannel.Human)
  - `generate_summary.toml` (auto)
- `bin/demo_legal_paralegal.sh` — end-to-end demo run
- `docs/verticals/legal-paralegal.md` — operator doc

## Acceptance criteria
- [ ] Vertical bootstraps via Genesis from charter
- [ ] Demo script runs end-to-end (mocked court filing OK)
- [ ] All 5 actions exercise (review / flag / draft / file /
      summarize)
- [ ] flag_clause_concern + file_with_court correctly route
      to human approval
- [ ] draft_amendment produces a markdown amendment doc
- [ ] E2E smoke passes in `crates/argo-e2e/tests/vertical_smoke.rs`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Total LOC ≤ 700 across the 11 files (HARD RULE 6: each
      file ≤ 500)

## Branch + PR
- Branch: `claude-vertical-legal-paralegal`
- PR title: `verticals: Legal paralegal demo vertical (claude-vertical-legal-paralegal)`

## Notes
- Persona MUST decline to give legal advice; system_prompt
  enforces "I can summarize but not advise — attorney
  required."
- file_with_court is mocked via stub executor (no real
  court API integration in V2)
- 10 eval seeds (vs 20 in finance-ar) is intentional —
  legal scenarios are harder to enumerate; quality > count
