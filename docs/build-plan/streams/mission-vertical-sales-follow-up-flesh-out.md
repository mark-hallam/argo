# Mission load-bearing wire — sales-follow-up vertical flesh-out

**Owner**: claude. **Depends on**: existing stub `verticals/sales-follow-up/` (vertical.toml + prompts/{system,persona}.md + eval/followup_core.jsonl); `verticals/recruiter-screener/` (#1340) + `verticals/lead-research/` as canonical full-pattern templates.
**Authored against**: origin/main `d20fabc3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-sales-follow-up-flesh-out`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #133 ("Charter seed for sales-follow-up"). **Tier A (vertical).** Warm-lead follow-up: research recent signals, draft personalised outreach, propose follow-up cadence. The current stub has the charter prompts + a thin eval; this brief extends it to the canonical 11-file pattern with 4 skills + memory + ≥9 eval fixtures + auto_approval + inventory. Drafts-only — never sends outreach, schedules calls, or writes to the CRM without operator approval. Anti-spam compliance load-bearing (CAN-SPAM Act + GDPR).

## Scope

EXTEND content-only `verticals/sales-follow-up/` (add 7 files + 1 eval expansion, ≤ 500 LOC delta — pattern budget: 4 skills × ≤ 65, schema ≤ 35, auto_approval ≤ 25, inventory ≤ 55, eval/seed.jsonl ≥ 9 fixtures):

- KEEP existing `vertical.toml` + `prompts/system.md` + `prompts/persona.md` (do not rewrite; extend `vertical.toml` only with `[actions.<skill>]` blocks + `[memory] schema` reference)
- 4 NEW skills (drafts-only):
  - `research_lead_signals` — lead-context (company + role + last-touch) in; recent-signals-summary (hiring posts / funding announcements / product-launches) + suggested-relevance-hook + skip-Y/N (if no fresh signal in 30d) out
  - `draft_followup_message` — lead-context + signal in; first-touch draft + alternate-tone variants (warm / professional / brief) + suggested-subject-line + CAN-SPAM-unsubscribe-line-included-Y/N out
  - `propose_cadence` — lead-stage + last-touch + engagement-history in; recommended-next-touch (day + channel) + escalation-after-N-no-response + dormancy-threshold-recommendation out
  - `summarize_conversation_thread` — message-thread in; objection-themes + buying-signals + recommended-next-action + tag-suggestions-for-CRM out (operator commits to CRM; vertical never writes)
- NEW `memory/schema.toml` — `lead_index` (lead-id → company/role/stage/last-touch), `cadence_policy` (per-stage day-offset map), `compliance_register` (per-jurisdiction: CAN-SPAM-applicability / GDPR-applicability / opt-out-registry), `messaging_voice` (per-tenant voice / tone-config / forbidden-phrases)
- NEW `auto_approval.toml` — wake list includes only the 4 research/draft/propose/summarize skills; `send_email`, `update_crm`, `schedule_call`, `add_to_sequence`, `export_lead_list` intentionally absent + gated through `[escalation].human_approval_required`
- NEW `inventory.json` — sample `lead_index` (3 synthetic leads), `cadence_policy` (3 stage→day-offset entries), `compliance_register` (US-CAN-SPAM + EU-GDPR + UK-PECR), `messaging_voice` (1 sample voice profile)
- EXPAND `eval/seed.jsonl` to ≥ 9 fixtures: 1 research-lead-w-fresh-signal, 1 research-lead-w-no-fresh-signal-skip, 1 draft-followup-warm-tone, 1 draft-followup-w-CAN-SPAM-block, 1 propose-cadence-active-engagement, 1 propose-cadence-dormancy-escalation, 1 summarize-thread-w-buying-signal, 3 escalation (lead-in-GDPR-opt-out-list / lead-explicitly-asked-not-to-contact / proposed-message-contains-forbidden-phrase)

NO Rust. NO migrations. NO crate changes. Pattern is identical to existing verticals — re-use shapes 1:1.

This is NOT in scope: actual CRM write-through (separate MCP-connector briefs — HubSpot, Salesforce, Pipedrive already covered or queued); email-sending wiring (vertical drafts; argo-channels + connectors send); sequence-orchestration UI (separate); lead-scoring model training (vertical surfaces signals; downstream tool scores); auto-personalisation from social profiles (the vertical accepts pre-fetched signals only — no live LinkedIn scraping).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/sales-follow-up/skills/research_lead_signals.md` (NEW, ≤ 65 LOC)
- `verticals/sales-follow-up/skills/draft_followup_message.md` (NEW, ≤ 65 LOC)
- `verticals/sales-follow-up/skills/propose_cadence.md` (NEW, ≤ 65 LOC)
- `verticals/sales-follow-up/skills/summarize_conversation_thread.md` (NEW, ≤ 65 LOC)
- `verticals/sales-follow-up/memory/schema.toml` (NEW, ≤ 35 LOC)
- `verticals/sales-follow-up/auto_approval.toml` (NEW, ≤ 25 LOC)
- `verticals/sales-follow-up/inventory.json` (NEW, ≤ 55 LOC)
- `verticals/sales-follow-up/eval/seed.jsonl` (NEW or EXPAND alongside existing `followup_core.jsonl`, ≥ 9 fixtures total)
- `verticals/sales-follow-up/vertical.toml` (EXTEND only — add 4 `[actions.<skill>]` blocks + `[memory] schema = "memory/schema.toml"` reference; do NOT rewrite)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory
- `verticals/sales-follow-up/prompts/{system,persona}.md` (the stub charter prompts stand)

## Acceptance criteria

- [ ] `argo vertical list` continues to show `sales-follow-up <version>` (version may bump 0.0.1 → 0.1.0)
- [ ] `argo vertical lint sales-follow-up` exits 0 errors (baseline warns acceptable)
- [ ] `argo vertical show sales-follow-up --json` returns valid envelope (id / name / version / persona / memory_entries / eval_seed_count ≥ 9)
- [ ] All 4 NEW skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] System (or one new skill) names CAN-SPAM Act + GDPR explicitly; unsubscribe-line inclusion is named in `draft_followup_message` skill output
- [ ] `eval/seed.jsonl` has ≥ 9 fixtures, ≥ 3 of which are escalation cases (GDPR-opt-out / explicit-do-not-contact / forbidden-phrase)
- [ ] `auto_approval.toml` wake list contains only the 4 NEW skills; send / update_crm / schedule / add_to_sequence / export gated through `[escalation].human_approval_required`
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-sales-follow-up-flesh-out`
- PR title: `vertical: sales-follow-up flesh-out (4 skills + memory + ≥9 eval + auto_approval + inventory) (mission-vertical-sales-follow-up-flesh-out)`

## Notes for the implementer

- Treat `verticals/recruiter-screener/` (#1340) and `verticals/lead-research/` as 1:1 templates for the 4-skills / memory / auto_approval / inventory shape.
- Drafts-only is load-bearing: every skill emits a DRAFT (signal-summary / message-draft / cadence-recommendation / thread-summary) — none send or write to CRM autonomously.
- CAN-SPAM (US) + GDPR (EU) + PECR (UK) compliance: the `compliance_register` memory entry surfaces which regime applies per-lead; eval fixtures #4 + escalation #1 exercise unsubscribe-line and GDPR-opt-out blocking.
- `messaging_voice` config gates against `forbidden_phrases` (e.g. spammy "limited time," "act now," "guaranteed") — eval escalation #3 exercises this.
- DO NOT add lead-scoring or predictive-engagement-modelling features. The vertical surfaces signals from `lead_index`; predictive scoring is a separate brief (and ethically requires explicit opt-in).
- Skip-flagging on "no fresh signal in 30d" prevents spammy nurture; eval fixture #2 tests this. Operator can override but the vertical recommends skip.
- The persona prompt (already present) carries the consultative tone — preserve it. The 4 skills inherit the tone; do not add a separate tone-override per skill.
