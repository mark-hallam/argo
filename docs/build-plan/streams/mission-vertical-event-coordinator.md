# Mission load-bearing wire — Event-coordinator vertical

**Owner**: claude. **Depends on**: existing vertical-template scaffold (`verticals/executive-assistant/` MERGED #1336 as canonical content-only-vertical pattern).
**Authored against**: origin/main `281508ec` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-event-coordinator`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #160 ("Event-coordinator vertical"). **Tier A (vertical).** Internal-event / corporate-offsite / conference coordination — venue + catering + attendee logistics + vendor management. Drafts only — all spend (venue holds, catering deposits, vendor contracts, transportation bookings) routes through human approval. The budget-ceiling pattern is load-bearing.

## Scope

NEW content-only `verticals/event-coordinator/` (11 files, ≤ 500 LOC total — pattern budget: system ≤ 120, persona ≤ 35, skills 4×≤60, schema ≤ 35, auto_approval ≤ 25, inventory ≤ 55, vertical.toml ≤ 55, eval/seed.jsonl ≥ 8 fixtures):

- `vertical.toml` — id=`event-coordinator`, version=`0.1.0`, optional gcal/airbnb/eventbrite MCP, `[escalation].human_approval_required = ["book_venue", "sign_contract", "place_catering_order", "send_attendee_email_blast", "issue_refund"]`
- `prompts/system.md` — budget-ceiling load-bearing (every recommendation cites total + remaining), vendor-qualification rule (must be in `approved_vendors` memory or escalate), 3-bid rule for spend ≥ $5k mirroring procurement-assistant, dietary-restriction-completeness check (catering recommendations must enumerate gluten/dairy/nut/halal/kosher/vegan coverage), accessibility-requirements check (ADA compliance for venue + materials), escalation triggers (international-travel-component / VIP-attendee-named / political-event-context / spend-over-budget / vendor-not-in-approved-list)
- `prompts/persona.md` — Reese, event-coordinator; methodical w/ logistics, plain-spoken w/ stakeholders, never speculates on attendee preferences (asks instead)
- 4 skills:
  - `propose_venue` — date + size + budget + region in; ranked venue options w/ pros/cons/total-cost out
  - `draft_attendee_comms` — event-stage in; pre-event / pre-deadline / on-day / post-event communications drafts w/ replacement-slot tokens out
  - `compare_catering_bids` — catering bid set in; per-bid dietary-coverage matrix + total-cost-per-head + comparison-recommendation out
  - `build_runbook` — event spec in; per-hour runbook w/ owner column + dependency arrows + risk-callouts out
- `memory/schema.toml` — `approved_vendors` (vendor_id / category / tier / contact / last_review_date), `budget_lines` (line_id / period / ceiling / used / remaining), `attendee_preferences_history` (per-attendee historical accommodations), `runbook_templates` (per-event-shape skeletons)
- `eval/seed.jsonl` — 8+ fixtures: 1 venue-happy-path, 1 catering-compare-w-2-clear-winner, 1 catering-compare-w-dietary-gap (escalate), 1 attendee-comms-draft, 1 runbook-build, 3 escalation (international-travel / VIP-attendee / spend-over-budget)
- `auto_approval.toml` — wake list includes the 4 propose/draft/compare/build skills only; book/sign/place/send/refund intentionally absent
- `inventory.json` — sample `approved_vendors` (5 entries across catering / venue / AV / transportation / printing); sample `budget_lines` (3 entries); sample `runbook_templates` (1 conference + 1 offsite skeleton)

NO Rust. NO migrations. NO crate changes. Pattern is identical to executive-assistant / procurement-assistant / research-analyst — re-use shapes 1:1.

This is NOT in scope: actual Eventbrite / Airbnb / vendor-portal integration (separate MCP-connector briefs); calendar invites generation (handled by executive-assistant for attendee-side scheduling); per-attendee follow-up automation (separate Tier B brief); virtual-event-platform integration (Zoom / Teams meeting links are referenced but the actual scheduling is downstream); seat assignment / banquet layout optimization (manual still).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/event-coordinator/vertical.toml` (NEW, ≤ 55 LOC)
- `verticals/event-coordinator/prompts/system.md` (NEW, ≤ 120 LOC)
- `verticals/event-coordinator/prompts/persona.md` (NEW, ≤ 35 LOC)
- `verticals/event-coordinator/skills/propose_venue.md` (NEW, ≤ 60 LOC)
- `verticals/event-coordinator/skills/draft_attendee_comms.md` (NEW, ≤ 60 LOC)
- `verticals/event-coordinator/skills/compare_catering_bids.md` (NEW, ≤ 60 LOC)
- `verticals/event-coordinator/skills/build_runbook.md` (NEW, ≤ 60 LOC)
- `verticals/event-coordinator/memory/schema.toml` (NEW, ≤ 35 LOC)
- `verticals/event-coordinator/eval/seed.jsonl` (NEW, ≥ 8 fixtures)
- `verticals/event-coordinator/auto_approval.toml` (NEW, ≤ 25 LOC)
- `verticals/event-coordinator/inventory.json` (NEW, ≤ 55 LOC)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory

## Acceptance criteria

- [ ] `argo vertical list` includes `event-coordinator 0.1.0`
- [ ] `argo vertical lint event-coordinator` exits 0 errors (warns of the executive-assistant / procurement-assistant baseline shape are acceptable)
- [ ] `argo vertical show event-coordinator --json` returns valid envelope (id / name / version / persona / memory_entries / eval_seed_count ≥ 8 / vertical_toml)
- [ ] All 4 skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] `eval/seed.jsonl` has ≥ 8 fixtures, ≥ 3 of which are escalation cases (international-travel / VIP-attendee / spend-over-budget)
- [ ] `auto_approval.toml` wake list contains only the 4 propose/draft/compare/build skills; book/sign/place/send/refund gated through `[escalation].human_approval_required`
- [ ] System prompt names budget-ceiling, 3-bid-rule, dietary-coverage check, and ADA-accessibility explicitly
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-event-coordinator`
- PR title: `vertical: event-coordinator (mission-vertical-event-coordinator)`

## Notes for the implementer

- Treat the just-merged `verticals/executive-assistant/` (#1336), `verticals/procurement-assistant/` (queued), and `verticals/research-analyst/` (#1337) as 1:1 templates — same file shape, same YAML frontmatter, same `[escalation]` block style. The budget-ceiling pattern is shared with procurement-assistant.
- Dietary-restriction completeness check is the load-bearing differentiator — every catering recommendation MUST cover gluten / dairy / nut / halal / kosher / vegan or escalate. This catches the most common event-failure mode (one attendee can't eat anything provided).
- ADA accessibility is also load-bearing — venue recommendations must surface ADA compliance status (or "unknown — verify with venue" if not in memory). Eval fixture should check this.
- The 3-bid rule mirrors procurement-assistant — same dollar threshold ($5k+) means 3 quotes required. Document this cross-vertical consistency in the system prompt.
- VIP-attendee escalation: if any named attendee in the inventory's `attendee_preferences_history` has a `vip_tier` field, the entire event escalates to human-review regardless of other factors. Eval fixture should test this.
- International-travel escalation: any travel component crossing a country border triggers escalation (visa / customs / per-diem rules vary too much for the LLM to handle).
- `build_runbook` output is the most operator-visible deliverable — it should be markdown with explicit columns (time / item / owner / dependency / risk) and the eval fixture should check the column shape.
- Sample inventory should NOT have any real PII — use synthetic vendor names (`Acme Catering`, `Lighthouse AV`, etc.).
