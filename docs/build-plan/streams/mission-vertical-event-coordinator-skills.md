# Mission load-bearing wire — Event-Coordinator vertical: 1+ skill to reach 6

**Owner**: claude. **Depends on**: `verticals/event-coordinator/` (5 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-event-coordinator-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. event-coordinator ships with 5 skills (build_runbook / compare_catering_bids / draft_attendee_comms / propose_venue / run_of_show). Brief brings to 6.

## Scope

1 NEW skill file:

1. **`track_rsvps.skill.md`** — given an attendee list + RSVP responses (operator-pasted), output JSON envelope: `{ confirmed, declined, pending, no_response, dietary_constraints[], accessibility_requests[], plus_ones_count }` + a Markdown summary suitable for a 24h-before-event status email. Reasoning: explicit cap on confirmed count if venue capacity is provided; flag over-capacity as `requires_immediate_operator_review`.

## Files you may touch

- `verticals/event-coordinator/skills/track_rsvps.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 5 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 1 new `.skill.md` file with full frontmatter + sections
- [ ] Output JSON includes all 7 listed fields
- [ ] Venue-capacity overflow flags `requires_immediate_operator_review`
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-event-coordinator-skills`
- PR title: `verticals: 1 skill for event-coordinator (mission-vertical-event-coordinator-skills)`

## Notes

- The 7-field output is the canonical RSVP envelope — downstream `run_of_show` and `build_runbook` skills can consume it without re-parsing.
- Capacity overflow is the load-bearing safety signal. Don't merge an "approve" decision on an event that's over-capacity without operator review.
