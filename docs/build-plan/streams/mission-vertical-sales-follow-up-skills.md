# Mission load-bearing wire — Sales-Follow-Up vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/sales-follow-up/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-sales-follow-up-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. sales-follow-up ships with 4 skills (draft_followup_message / propose_cadence / research_lead_signals / summarize_conversation_thread). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`detect_disengagement.skill.md`** — given a lead's engagement history (last_touch, opens/clicks pattern, response gaps), identify disengagement risk + recommend action (continue-cadence / pause-30d / switch-channel / drop-from-cadence). Output JSON: `{ engagement_score: 0-100, trend: improving|stable|declining|cold, days_since_last_response, recommended_action, supporting_evidence }`. Conservative on drop-recommendation (only at engagement < 20 AND >60 days no-response).
2. **`reframe_objection.skill.md`** — given an objection captured in the conversation thread (price / timing / authority / fit / no-budget), draft a reframe attempt addressing the underlying concern without arguing the surface objection. Output Markdown response + structured `{ underlying_concern_inferred, reframe_approach, fallback_to_AE: bool }`. Approval-gated before send.

## Files you may touch

- `verticals/sales-follow-up/skills/detect_disengagement.skill.md` (NEW, ≤ 90 LOC)
- `verticals/sales-follow-up/skills/reframe_objection.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts; `verticals/sales-sdr/` (sibling).

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `detect_disengagement` 4-action vocab + 4-trend vocab + drop threshold explicit (score<20 AND >60d)
- [ ] `reframe_objection` 5-objection-class vocab (price/timing/authority/fit/no-budget) + AE-escalation fallback
- [ ] Both reference data from `memory` (cadence_policy / lead_index) — no fabrication
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-sales-follow-up-skills`
- PR title: `verticals: 2 skills for sales-follow-up (mission-vertical-sales-follow-up-skills)`

## Notes

- `detect_disengagement`'s conservative drop-threshold is operator-respect — premature drops cost pipeline. The two-gate rule (low engagement AND >60d) is the load-bearing pattern.
- `reframe_objection` should NEVER argue the surface objection (e.g. "actually we are cheaper than X"). The reframe addresses the underlying concern ("is this priority right now"). Document this in `# Hard rules`.
