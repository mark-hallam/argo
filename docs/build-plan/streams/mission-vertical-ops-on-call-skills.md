# Mission load-bearing wire — Ops-On-Call vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/ops-on-call/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-ops-on-call-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. ops-on-call ships with 4 skills (draft_incident_update / escalate_to_human / find_runbook / triage_page). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`coordinate_war_room.skill.md`** — given an active incident + the response roster (operator-provided), output a 5-section coordination plan: (a) current state, (b) active investigation tracks (named owner per track), (c) blocked items + who can unblock, (d) external comms status, (e) next 15-min checkpoint. Output Markdown. Autonomous (internal coordination doc).
2. **`draft_post_incident_summary.skill.md`** — given the incident timeline (entries from the audit chain + operator notes), draft a 5-section post-incident summary: (a) impact (users / duration / data), (b) timeline, (c) root cause, (d) what worked, (e) action items (owner + due date per item). Approval-gated before external-facing publication.

## Files you may touch

- `verticals/ops-on-call/skills/coordinate_war_room.skill.md` (NEW, ≤ 90 LOC)
- `verticals/ops-on-call/skills/draft_post_incident_summary.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `coordinate_war_room` 5-section structure documented; 15-min checkpoint rhythm explicit
- [ ] `draft_post_incident_summary` 5-section structure + approval-gated for external publication
- [ ] Neither skill names root-causes without operator-provided evidence (no fabrication)
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-ops-on-call-skills`
- PR title: `verticals: 2 skills for ops-on-call (mission-vertical-ops-on-call-skills)`

## Notes

- Match the existing tone — ops-on-call persona is calm-under-pressure, structure-over-prose. Skills should reinforce.
- The 15-min checkpoint rhythm in `coordinate_war_room` is the load-bearing detail — long-cycle status updates are how incidents extend. Bias short.
- Don't speculate on root cause in `draft_post_incident_summary` — if the operator hasn't named one, the section says "under investigation" with a named owner.
