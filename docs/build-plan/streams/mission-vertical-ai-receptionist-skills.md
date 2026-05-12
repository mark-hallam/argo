# Mission load-bearing wire — AI Receptionist vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: existing `verticals/ai-receptionist/` charter stub (4 existing skills).
**Authored against**: origin/main `b825b1d5` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-ai-receptionist-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to ai-receptionist. Today the vertical ships with 4 skills (draft_after_hours_response / propose_meeting_slot / route_message_to_owner / triage_inbound_call). This brief brings it to 6.

## Scope

2 NEW skill files under `verticals/ai-receptionist/skills/`:

1. **`verify_caller_identity.skill.md`** — given an inbound call + caller-claimed identity, validate against the operator's known-contacts list (`memory.contact_book[]`) using fuzzy name + phone match. Output JSON: `{matched_contact, confidence, verification_signals: [phone_match, name_match, prior_call_count], action: greet_by_name|request_clarification|escalate_to_human}`. Confidence < 0.7 routes to clarification. Autonomous.
2. **`schedule_callback.skill.md`** — when the operator is unavailable + the caller needs a follow-up, capture preferred callback window (next-available / specific-window / urgent) + operator's calendar availability + reason. Output JSON envelope. Confirms back to caller in plain language. Approval-gated only for after-hours scheduling (operator's quiet-hours setting); otherwise autonomous.

Each skill ~50-80 LOC. Total: ~120-160 LOC.

This is NOT in scope: live calendar write (sibling brief); SMS/voice transcription engine (operator-provided); CRM integration; visitor-recognition vision.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/ai-receptionist/skills/verify_caller_identity.skill.md` (NEW, ≤ 80 LOC)
- `verticals/ai-receptionist/skills/schedule_callback.skill.md` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Existing 4 skill files
- `verticals/ai-receptionist/vertical.toml`, prompts — locked
- `verticals/receptionist/**` — sibling with narrower (conversation-only) scope

## Acceptance criteria

- [ ] 2 new `.skill.md` files with frontmatter + sections (When to use / Inputs / Output shape / Examples / Approval posture)
- [ ] `verify_caller_identity` 0.7-confidence threshold + 3-action vocab (greet_by_name / request_clarification / escalate_to_human)
- [ ] `schedule_callback` respects quiet-hours operator setting (gate after-hours scheduling)
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-ai-receptionist-skills`
- PR title: `verticals: 2 skills for ai-receptionist (mission-vertical-ai-receptionist-skills)`

## Notes for the implementer

- `verify_caller_identity` should NOT make claims about a caller's identity beyond the verification signals — output is a routing recommendation, not an authoritative identification. The operator decides whether to treat a fuzzy match as definitive.
- `schedule_callback`'s quiet-hours interaction is the safety belt — never schedule outside the operator's stated availability without explicit approval-gating.
- Both skills should be tone-matched to the existing `draft_after_hours_response` skill — Avery-persona is the family voice.
