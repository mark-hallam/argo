# V2.3 — Product Manager demo vertical

**Owner**: claude. **Depends on**: G-genesis (#790),
claude-vertical-finance-ar (#937),
C-gcal + claude-document-creation-executor merged.
Vertical demo #12.

## Scope

Product Manager demo vertical — proves the platform handles
internal knowledge-worker flows where most outputs are
documents and the auto-approval ratio is high (PMs are
trusted writers; only roadmap commits gate). Persona
"Riley" handles PM duties:
- Triages inbound user feedback into themes
- Drafts PRDs from feature requests
- Schedules cross-functional reviews
- Updates the public roadmap (gated — external commitment)
- Summarizes user research interviews

High-value demo because (a) PM work is a great fit for
agentic doc generation, (b) the meta-quality of "an AI PM
for an AI startup" plays well in demos, (c) shows
auto-approval ratio scaling up when output is internal.

## What lands

11 NEW files (no NEW crate — re-use existing executors):

- `verticals/product-manager/vertical.toml` — manifest
  (persona=Riley, allowed_executors=[gcal,
  document_creation, human_approval])
- `verticals/product-manager/persona.toml` — voice
  (curious, structured, opinionated but humble)
- `verticals/product-manager/system_prompt.md` — duties,
  prioritization frameworks, scope limits
- `verticals/product-manager/memory_schema.toml` —
  feedback_themes, prd_drafts, roadmap_state,
  research_synthesis_log
- `verticals/product-manager/eval/seeds.jsonl` — 10
  scenarios (single feature ask / theme synthesis /
  competitive response PRD / roadmap re-prioritization /
  research read-out)
- `verticals/product-manager/inventory/sample_feedback.json`
  — 20 fake support tickets / interview snippets / NPS
  comments (more rows than other verticals — synthesis
  needs a corpus)
- `verticals/product-manager/skills/prd_template.md` — the
  standard PRD structure the persona uses
- `verticals/product-manager/workspace/auto_approval.toml`
  — triage_feedback + schedule_review +
  summarize_user_research auto; PRD + roadmap gated
- `verticals/product-manager/actions/` — 5 action defs:
  - `triage_feedback.toml` (auto)
  - `draft_prd.toml` (NotifyChannel.DocumentCreation)
  - `schedule_review.toml` (auto)
  - `update_roadmap.toml` (NotifyChannel.Human)
  - `summarize_user_research.toml` (auto)
- `bin/demo_product_manager.sh` — end-to-end demo run
- `docs/verticals/product-manager.md` — operator doc

## Acceptance criteria
- [ ] Vertical bootstraps via Genesis from charter
- [ ] Demo script runs end-to-end
- [ ] All 5 actions exercise
- [ ] triage_feedback emits structured theme clusters
- [ ] draft_prd produces a markdown PRD following the
      template
- [ ] update_roadmap correctly gates to human (only
      gate in the vertical — auto-approval ratio is 3/5)
- [ ] summarize_user_research auto-runs and produces a
      synthesis doc
- [ ] E2E smoke passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Total LOC ≤ 750 across the 11 files (+50 vs other
      verticals to accommodate 20-row sample_feedback.json)

## Branch + PR
- Branch: `claude-vertical-product-manager`
- PR title: `verticals: Product manager demo vertical (claude-vertical-product-manager)`

## Notes
- HIGHEST auto-approval ratio of the 10 demos (3/5) —
  intentional: PM outputs are usually internal drafts that
  humans review in next-step workflows
- update_roadmap is the only gate because it's the one
  external-commitment action — public roadmap changes
  affect customers
- prd_template.md is operator-customizable; treat as a
  starting point, not a fixed shape
- 20 sample feedback rows (vs 5-10 elsewhere) — synthesis
  demos need enough data to actually cluster
