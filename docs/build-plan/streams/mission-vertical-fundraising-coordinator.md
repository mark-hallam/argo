# Mission load-bearing wire — `fundraising-coordinator` vertical

**Owner**: claude. **Depends on**: existing vertical pattern.
**Authored against**: origin/main `6960eb12` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-fundraising-coordinator`.

> SOURCE: MISSION_BACKLOG.md Section 11 #156 — "Fundraising-coordinator vertical." Founder-facing assistant for VC raise: investor pipeline tracking, intro requests, materials prep, follow-up cadence. Treats every external interaction as high-stakes — escalates aggressively.

## Scope

NEW `verticals/fundraising-coordinator/` (content-only):

1. `vertical.toml`:
   - `[vertical].id = "fundraising-coordinator"`, version `0.1.0`
   - `[role]` persona = experienced fundraising chief-of-staff
   - `[tools].allowed` = `["web_search", "web_extract", "file_read"]`
   - `[mcp_servers]` = `{ gmail = {...}, gcal = {...}, hubspot = {...} }` (optional)
   - `[escalation].human_approval_required` = `["send_email_external", "share_deck", "respond_to_term_sheet", "negotiate_anything"]`
   - `[escalation].escalate_after_failures = 1`

2. `prompts/system.md` (≤ 200 LOC) — covers: pipeline stages (intro → first-call → followup → diligence → term-sheet), founder voice-matching for outbound, NEVER auto-share deck without explicit founder approval per share, MNPI/confidentiality preservation, NEVER negotiate (always escalate even on low-stakes asks).

3. `prompts/persona.md` (≤ 100 LOC).

4. `skills/research_investor.md` — given investor name + firm, produce 1-pager: thesis fit + portfolio overlap + check size range + intro paths.

5. `skills/draft_followup.md` — given last interaction context, produce 1-paragraph follow-up + 1 clear ask.

6. `skills/schedule_intro_call.md` — propose 3 slots given investor + founder calendar constraints; phrase in founder voice.

7. `skills/track_pipeline_status.md` — given pipeline data, surface stalled investors (>14d no contact) + at-risk threads + next-best-action.

8. `memory/schema.toml` — namespaces: `investor_pipeline`, `founder_voice_samples`, `thesis_alignments`, `outreach_log`.

9. `eval/seed.jsonl` (≥ 7 fixtures): 3 clear-research, 2 followup-draft, 2 should-escalate (term sheet / negotiation request).

10. `auto_approval.toml` — research + draft + status auto. ALL external comms + deck-share + term-sheet response gated.

11. `inventory.json` — sample thesis_alignments for v1 demo.

This is NOT in scope: actual deck rewriting (separate brief); cap-table modeling; legal-review of term sheets (always escalates per system prompt); managing the data-room directly.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/fundraising-coordinator/vertical.toml` (NEW)
- `verticals/fundraising-coordinator/prompts/system.md` (NEW, ≤ 200 LOC)
- `verticals/fundraising-coordinator/prompts/persona.md` (NEW, ≤ 100 LOC)
- `verticals/fundraising-coordinator/skills/*.md` (4 NEW skill files, ≤ 80 LOC each)
- `verticals/fundraising-coordinator/memory/schema.toml` (NEW, ≤ 40 LOC)
- `verticals/fundraising-coordinator/eval/seed.jsonl` (NEW, ≥ 7 fixtures)
- `verticals/fundraising-coordinator/auto_approval.toml` (NEW, ≤ 40 LOC)
- `verticals/fundraising-coordinator/inventory.json` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Any other `verticals/<name>/` directory
- Any `crates/**` source
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `argo vertical list` includes `fundraising-coordinator`
- [ ] `argo vertical lint fundraising-coordinator` reports no errors
- [ ] All 4 skill files have correct frontmatter
- [ ] eval/seed.jsonl has ≥ 7 fixtures including ≥ 2 escalation cases
- [ ] auto_approval.toml gates ALL external comms + negotiation behind human
- [ ] `escalate_after_failures = 1` (conservative)
- [ ] No code changes
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-fundraising-coordinator`
- PR title: `vertical: fundraising-coordinator (mission-vertical-fundraising-coordinator)`

## Notes for the implementer

- The "NEVER negotiate" + "NEVER auto-share deck" rules in system.md are load-bearing. Be explicit and unambiguous.
- For eval/seed.jsonl: use fictional investors ("Acme Capital, $5-10M Series A, B2B SaaS focus"). No real firms.
- Founder-voice-matching is the harder content — give 2-3 example "voice samples" the prompt can pattern-match against.
- The MNPI line: if a fundraising-coordinator hears strategic info from one investor that affects another's pitch, that's MNPI territory — always escalate.
