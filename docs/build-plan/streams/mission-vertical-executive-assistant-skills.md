# Mission load-bearing wire — Executive-Assistant vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: existing `verticals/executive-assistant/` charter stub (4 existing skills).
**Authored against**: origin/main `b825b1d5` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-executive-assistant-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to executive-assistant. Today the vertical ships with 4 skills (draft_meeting_brief / find_calendar_slot / summarize_thread / triage_inbox). This brief brings it to 6.

## Scope

2 NEW skill files under `verticals/executive-assistant/skills/`:

1. **`plan_travel_logistics.skill.md`** — given a trip request (destination + date range + meeting purpose), draft an itinerary: flights (3 options with timing + cost trade-offs), hotel (3 options matching exec's preferences from memory), ground transport, key meetings on the trip + their addresses. Output structured JSON + Markdown summary. **Approval-gated** per charter — travel commits spend.
2. **`prepare_meeting_brief.skill.md`** — given a meeting on the exec's calendar + attendee list, prep a 1-page brief: attendee bios (from `memory.contact_history[]`), prior interactions, current relationship state, suggested talking points, open action items from prior meetings, agenda + objectives. Output Markdown. Internal draft per charter — autonomous.

Each skill ~50-90 LOC. Total: ~150-180 LOC.

This is NOT in scope: live gmail/gcal/slack write (charter says "drafts only — any external comms ... routes through human approval"); booking flights/hotels (charter excludes commits without approval); travel-policy enforcement (operator-provided rules only).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/executive-assistant/skills/plan_travel_logistics.skill.md` (NEW, ≤ 90 LOC)
- `verticals/executive-assistant/skills/prepare_meeting_brief.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files
- `verticals/executive-assistant/vertical.toml`, prompts — locked

## Acceptance criteria

- [ ] 2 new `.skill.md` files with frontmatter + sections
- [ ] `plan_travel_logistics` outputs 3 flight + 3 hotel options with explicit trade-offs (price / timing / preference-match)
- [ ] `plan_travel_logistics` marked `requires_approval: true` (financial commitment)
- [ ] `prepare_meeting_brief` cites every attendee fact back to `memory.contact_history[]` or marks `unknown / first-meeting`
- [ ] `prepare_meeting_brief` 1-page ceiling (~300 word output cap) — execs don't read longer
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-executive-assistant-skills`
- PR title: `verticals: 2 skills for executive-assistant (mission-vertical-executive-assistant-skills)`

## Notes for the implementer

- Read `verticals/executive-assistant/persona.md` BEFORE writing — the exec-assistant persona is discretion-first. Skills should reinforce: never share exec's calendar or contacts beyond what was explicitly requested; never make commitments on the exec's behalf.
- `plan_travel_logistics`: the operator's exec usually has stored preferences (window vs aisle, airline loyalty, hotel chain). Cite the preference source per recommendation. If preference is unknown, mark `assumption: generic-best-practice`.
- `prepare_meeting_brief`'s ~300-word ceiling is operator-respect: execs glance, they don't read. Use bullets, not paragraphs.
