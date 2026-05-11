# Mission load-bearing wire — AI-receptionist vertical flesh-out

**Owner**: claude. **Depends on**: existing stub `verticals/ai-receptionist/` (vertical.toml + prompts/{system,persona}.md only); `verticals/recruiter-screener/` (#1340) + `verticals/executive-assistant/` (#1336) as canonical full-pattern templates.
**Authored against**: origin/main `d20fabc3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-ai-receptionist-flesh-out`.

> SOURCE: MISSION_BACKLOG.md Section 4 Claim 11 #132 ("Charter seed for ai-receptionist"). **Tier A (vertical).** Front-desk worker that triages incoming calls/messages, schedules meetings, and routes communications to the right team member. The current stub is just vertical.toml + system/persona prompts; this brief extends it to the canonical 11-file pattern with 4 skills + memory + ≥9 eval fixtures + auto_approval + inventory so it can be claimed by operators and exercised by the swarm. Drafts-only — never sends, schedules, or commits without operator approval.

## Scope

EXTEND content-only `verticals/ai-receptionist/` (add 7 files + 1 eval expansion, ≤ 500 LOC delta — pattern budget: 4 skills × ≤ 65, schema ≤ 35, auto_approval ≤ 25, inventory ≤ 55, eval/seed.jsonl ≥ 9 fixtures):

- KEEP existing `vertical.toml` + `prompts/system.md` + `prompts/persona.md` (do not rewrite unless adding the 4 skill action declarations to `[actions]` block)
- 4 NEW skills (drafts-only):
  - `triage_inbound_call` — caller-context + transcript-snippet in; intent-classification (booking / question / complaint / spam) + suggested-routing-target + escalation-Y/N + draft-callback-message out
  - `propose_meeting_slot` — caller + requested-attendees + topic in; 3 candidate slots from calendar-availability cache + conflict-summary + draft confirmation-message-to-caller out (NEVER books — proposes only)
  - `route_message_to_owner` — message + sender + roster-cache in; recommended-owner (name + role + handle) + urgency-classification + suggested-response-template out
  - `draft_after_hours_response` — message + business-hours-policy in; on-brand acknowledgement + next-available-time + escalation-path-if-urgent out
- NEW `memory/schema.toml` — `roster` (name + role + handle + topics-they-own), `business_hours` (per-weekday open/close + holiday overrides), `routing_policy` (intent → owner map), `caller_history` (last-7-day caller-id → previous-interaction-summary; PII-soft — never echo full name + reason in same draft without operator approval)
- NEW `auto_approval.toml` — wake list includes only the 4 triage/propose/route/draft skills; `book_meeting`, `send_reply`, `transfer_call`, `share_caller_data_externally` intentionally absent + gated through `[escalation].human_approval_required`
- NEW `inventory.json` — sample `roster` (4 synthetic teammates), `business_hours` (Mon-Fri 09:00-17:00 + 1 holiday), `routing_policy` (4 intent→owner entries), `caller_history` (3 synthetic — all anonymized initials only)
- EXPAND `eval/` to ≥ 9 fixtures: 1 triage-booking-intent, 1 triage-spam-detection, 1 triage-complaint-route-to-manager, 1 propose-meeting-w-conflict, 1 route-message-w-clear-owner, 1 route-message-w-ambiguous-owner-escalate, 1 after-hours-non-urgent, 3 escalation (caller-asks-for-customer-data-disclosure / urgency-without-on-call / proposed-meeting-conflicts-w-CEO-meeting)

NO Rust. NO migrations. NO crate changes. Pattern is identical to existing verticals — re-use shapes 1:1.

This is NOT in scope: actual telephony connector wiring (Twilio / Vonage — separate MCP brief); calendar-booking wiring (separate — vertical proposes slots only); voice transcription model selection (the vertical accepts pre-transcribed text); spam-detection model training (heuristic-only classification in the prompt); on-call rotation logic (the vertical reads roster + business-hours from memory but does not maintain them).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/ai-receptionist/skills/triage_inbound_call.md` (NEW, ≤ 65 LOC)
- `verticals/ai-receptionist/skills/propose_meeting_slot.md` (NEW, ≤ 65 LOC)
- `verticals/ai-receptionist/skills/route_message_to_owner.md` (NEW, ≤ 65 LOC)
- `verticals/ai-receptionist/skills/draft_after_hours_response.md` (NEW, ≤ 65 LOC)
- `verticals/ai-receptionist/memory/schema.toml` (NEW, ≤ 35 LOC)
- `verticals/ai-receptionist/auto_approval.toml` (NEW, ≤ 25 LOC)
- `verticals/ai-receptionist/inventory.json` (NEW, ≤ 55 LOC)
- `verticals/ai-receptionist/eval/seed.jsonl` (NEW or EXPAND existing, ≥ 9 fixtures total)
- `verticals/ai-receptionist/vertical.toml` (EXTEND only — add 4 `[actions.<skill>]` blocks + `[memory] schema = "memory/schema.toml"` if not already present; do NOT rewrite)

## Files you must NOT touch

- Any `crates/**` Rust source
- `crates/argo-store/migrations/**`
- Any other `verticals/<other>/**` directory
- `verticals/ai-receptionist/prompts/{system,persona}.md` (the stub charter prompts stand)

## Acceptance criteria

- [ ] `argo vertical list` continues to show `ai-receptionist <version>` (version may bump 0.0.1 → 0.1.0)
- [ ] `argo vertical lint ai-receptionist` exits 0 errors (baseline warns acceptable)
- [ ] `argo vertical show ai-receptionist --json` returns valid envelope (id / name / version / persona / memory_entries / eval_seed_count ≥ 9)
- [ ] All 4 NEW skill files have YAML frontmatter (`name` + `description` + `inputs` + `outputs`)
- [ ] `eval/seed.jsonl` has ≥ 9 fixtures, ≥ 3 of which are escalation cases
- [ ] `auto_approval.toml` wake list contains only the 4 NEW skills; `book_meeting` / `send_reply` / `transfer_call` / `share_caller_data_externally` gated through `[escalation].human_approval_required`
- [ ] No Rust changes; no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-ai-receptionist-flesh-out`
- PR title: `vertical: ai-receptionist flesh-out (4 skills + memory + ≥9 eval + auto_approval + inventory) (mission-vertical-ai-receptionist-flesh-out)`

## Notes for the implementer

- The stub already has `vertical.toml` + `prompts/system.md` + `prompts/persona.md`. Do NOT rewrite the prompts — they carry the charter framing. Only EXTEND `vertical.toml` with 4 new `[actions.<skill>]` blocks + `[memory] schema` reference.
- Treat `verticals/recruiter-screener/` (#1340) and `verticals/executive-assistant/` (#1336) as 1:1 templates for the 4-skills / memory / auto_approval / inventory shape.
- The drafts-only pattern is load-bearing: every skill emits a DRAFT (call summary / meeting proposal / response template / after-hours acknowledgement) — none send, book, or transfer autonomously. Operator confirmation is mandatory.
- `caller_history` memory should be PII-soft: synthesize anonymized initials only in eval fixtures. Never echo full name + interaction-reason together in a single draft without explicit operator approval — eval fixture #9 tests this.
- The escalation list (`book_meeting` / `send_reply` / `transfer_call` / `share_caller_data_externally`) maps to the operator-touching surface: anything that commits state externally or discloses caller info gates through `[escalation].human_approval_required`.
- DO NOT add a "predict caller intent without transcript" feature. Voice-only intent classification without text is out of scope — the vertical accepts pre-transcribed input.
