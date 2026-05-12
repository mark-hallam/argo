# Mission load-bearing wire — Sales SDR vertical: 5+ skills

**Owner**: claude. **Depends on**: existing `verticals/sales-sdr/` charter stub.
**Authored against**: origin/main `e2fdf0a3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-sales-sdr-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to sales-sdr. Today the vertical ships with one skill (`draft_outreach.md`). MISSION claim 7 expects parity-with-gtm-engineer. This brief brings sales-sdr to 6 skills. Sibling-style of `verticals/sales-follow-up/` (4 existing skills) — sales-sdr's flow is upstream (prospecting + first-touch) where sales-follow-up is downstream (re-engagement).

## Scope

5 NEW skill files under `verticals/sales-sdr/skills/`. Skills cover the dominant SDR flow not already covered by `draft_outreach`:

1. **`qualify_lead.skill.md`** — score a single lead against the operator-provided ICP. Output JSON: `{qualification: high|medium|low|disqualify, signals_match: [...], signals_missing: [...], reasoning, recommended_next_step}`. Disqualify-with-reason is preferred over forcing a low-confidence "low" — operators see disqualified leads less often.
2. **`research_prospect.skill.md`** — deep dive on a single prospect (company + key contact). Reads from `memory.icp` + `memory.account_research[]` if present, else generates from web search. Output sections: company facts (with sources), buying signals (with confidence), likely pain (`verified|inferred`), suggested angle. Mirror `verticals/gtm-engineer/skills/research_company.skill.md` shape.
3. **`book_meeting.skill.md`** — given prospect interest signal + operator-available time blocks, propose 3 meeting times + draft a calendar-hold message. Output JSON: `{proposed_times: [...], hold_message: "...", reschedule_etiquette: "..."}`. Approval-gated. Never auto-sends.
4. **`handle_objection.skill.md`** — given an inbound objection (price, timing, fit, authority, need — the BANT-derived 5), draft a measured response. Output JSON: `{objection_class, response_draft, escalation_threshold: when-to-loop-AE-in}`. Always includes "escalate to AE" as a fallback path.
5. **`update_lead_state.skill.md`** — persist lead state transitions: new → researched → contacted → engaged → qualified → meeting-set → handed-off → disqualified. Writes to `memory.lead_pipeline[]`. Includes a `last_touched_at` timestamp so re-engagement skills (sales-follow-up) can pick up the trail.

Each skill file ~40-80 LOC. Total: ~350-450 LOC.

This is NOT in scope: changing `vertical.toml`; modifying prompts; live CRM writes (separate brief); auto-sending outreach (charter mandates approval-gating); per-skill Rust tests.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/sales-sdr/skills/qualify_lead.skill.md` (NEW, ≤ 90 LOC)
- `verticals/sales-sdr/skills/research_prospect.skill.md` (NEW, ≤ 90 LOC)
- `verticals/sales-sdr/skills/book_meeting.skill.md` (NEW, ≤ 80 LOC)
- `verticals/sales-sdr/skills/handle_objection.skill.md` (NEW, ≤ 80 LOC)
- `verticals/sales-sdr/skills/update_lead_state.skill.md` (NEW, ≤ 70 LOC)

## Files you must NOT touch

- `verticals/sales-sdr/skills/draft_outreach.md` — existing skill stable
- `verticals/sales-sdr/vertical.toml` — schema is fine
- `verticals/sales-sdr/prompts/**` — persona stable
- `verticals/sales-follow-up/**` — sibling vertical, different scope; don't drift its skills

## Acceptance criteria

- [ ] 5 new `.skill.md` files
- [ ] Each file has frontmatter: `name`, `version: "0.1.0"`, `description`, `provenance: { source: vertical/sales-sdr, hash: sha256-pending }`
- [ ] Each skill has sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`
- [ ] `qualify_lead` includes the disqualify-with-reason path explicitly
- [ ] `research_prospect` mirrors gtm-engineer's `research_company.skill.md` JSON structure where they overlap (so the two skills compose without translation)
- [ ] `book_meeting` outputs are clearly marked `requires_approval: true`
- [ ] `handle_objection` always includes the AE-escalation fallback
- [ ] `update_lead_state` documents the 8-state vocabulary
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-sales-sdr-skills`
- PR title: `verticals: 5 skills for sales-sdr (mission-vertical-sales-sdr-skills)`

## Notes for the implementer

- The SDR persona is high-volume, high-rejection — the skills should be brief and decisive. Avoid wordy "let me consider both sides" framing in `# Examples`.
- `research_prospect` overlaps with gtm-engineer's `research_company` — that's intentional. The shapes match so a future skill-router can call whichever is available. Don't try to be clever and import-by-reference; ship the skill self-contained, just keep the JSON shape compatible.
- The BANT objection classes (Budget / Authority / Need / Timing — plus Fit as a 5th SDR-specific class) are the canonical objection vocabulary. Use these exact names.
- `update_lead_state`'s 8-state vocabulary is shared with `sales-follow-up`'s `propose_cadence` skill — don't drift the names. If you find a state name there that disagrees, prefer the existing skill's name and document it in your skill's `# Memory schema notes`.
- Charter mandates "every outbound message is approval-gated" — `draft_outreach`, `book_meeting`, and `handle_objection` outputs all need `requires_approval: true` marker. `qualify_lead`, `research_prospect`, and `update_lead_state` are internal and don't need the marker.
- Demo readiness: a working SDR demo runs `research_prospect` → `qualify_lead` → (if qualified) `draft_outreach` → (approval) → `book_meeting` → `update_lead_state(meeting-set)`. The new skills + the existing skill cover that chain.
