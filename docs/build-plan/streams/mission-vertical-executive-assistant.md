# Mission load-bearing wire — `executive-assistant` vertical

**Owner**: claude. **Depends on**: existing vertical pattern (verified at `verticals/marketplace-seller-browser/` + 18 other shipped verticals).
**Authored against**: origin/main `d10658d5` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-executive-assistant`.

> SOURCE: MISSION_BACKLOG.md Section 11 #145 — "Executive-assistant vertical." Email triage + calendar coordination + meeting prep + travel logistics for a single principal. Auto-approval gates anything spending money or external comms.

## Scope

NEW `verticals/executive-assistant/` (content-only) mirroring `verticals/marketplace-seller-browser/`:

1. `vertical.toml`:
   - `[vertical].id = "executive-assistant"`, version `0.1.0`
   - `[role]` persona = discreet senior EA with 10+ years C-level support
   - `[tools].allowed` = `["web_search", "web_extract", "file_read"]`
   - `[mcp_servers]` = `{ gmail = {...}, gcal = {...}, slack = {...} }` (all optional — vertical works in dry-run)
   - `[escalation].human_approval_required` = `["send_email_external", "book_travel", "schedule_meeting_with_external"]`
   - `[escalation].escalate_after_failures = 2`

2. `prompts/system.md` (≤ 200 LOC) — covers: principal's daily routine, prioritization heuristics (drop-ins / scheduled / fires), tone-matching for outbound, how to summarize attachments, when to NEVER auto-respond (legal, financial, family-emergency).

3. `prompts/persona.md` (≤ 100 LOC) — first-person.

4. `skills/triage_inbox.md` — bucket emails into action/info/spam/respond-with-template.

5. `skills/draft_meeting_brief.md` — given an invite, produce a 1-pager with attendees + relevant prior threads + agenda.

6. `skills/find_calendar_slot.md` — propose 3 slots given attendee constraints + principal's prefs.

7. `skills/summarize_thread.md` — long email thread → 5-bullet summary + decision points.

8. `memory/schema.toml` — namespaces: `principal_prefs`, `frequent_contacts`, `travel_aliases`, `drafted_replies`.

9. `eval/seed.jsonl` (≥ 7 fixtures): 3 clear-triage, 2 meeting-brief, 2 escalation-needed.

10. `auto_approval.toml` — auto-approves: triage, calendar reads, draft generation. Requires-approval: send, book, schedule-with-external.

11. `inventory.json` — example principal prefs for v1 demo.

This is NOT in scope: voice/phone integration; shared-team-EA pattern (single-principal v1); legal-review automation; expense reconciliation (that's bookkeeping-assistant's territory).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/executive-assistant/vertical.toml` (NEW)
- `verticals/executive-assistant/prompts/system.md` (NEW, ≤ 200 LOC)
- `verticals/executive-assistant/prompts/persona.md` (NEW, ≤ 100 LOC)
- `verticals/executive-assistant/skills/*.md` (4 NEW skill files, ≤ 80 LOC each)
- `verticals/executive-assistant/memory/schema.toml` (NEW, ≤ 40 LOC)
- `verticals/executive-assistant/eval/seed.jsonl` (NEW, ≥ 7 fixtures)
- `verticals/executive-assistant/auto_approval.toml` (NEW, ≤ 40 LOC)
- `verticals/executive-assistant/inventory.json` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Any other `verticals/<name>/` directory
- Any `crates/**` source
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `argo vertical list` includes `executive-assistant`
- [ ] `argo vertical lint executive-assistant` reports no errors
- [ ] `argo vertical show executive-assistant --json` returns valid envelope
- [ ] All 4 skill files have correct markdown frontmatter
- [ ] eval/seed.jsonl has ≥ 7 fixtures, each with `input` + `expected`
- [ ] auto_approval.toml gates external-comms + spend behind human
- [ ] No code changes
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-executive-assistant`
- PR title: `vertical: executive-assistant (mission-vertical-executive-assistant)`

## Notes for the implementer

- Use `verticals/marketplace-seller-browser/` as the structural template.
- The "never auto-respond" list in system.md is load-bearing — be explicit about legal, financial, family-emergency. Operators will read this.
- For eval/seed.jsonl: avoid real names. Use generic "ACME CEO" patterns.
- The optional MCP connectors (gmail/gcal/slack) match existing shipped connectors in the workspace — vertical lints clean even when they're not installed.
