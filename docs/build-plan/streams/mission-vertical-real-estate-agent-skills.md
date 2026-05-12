# Mission load-bearing wire — Real-Estate-Agent vertical: 5+ skills

**Owner**: claude. **Depends on**: existing `verticals/real-estate-agent/` charter stub.
**Authored against**: origin/main `e2441425` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-real-estate-agent-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to real-estate-agent. Today the vertical ships with 1 skill (`lead_qualification.md`). MISSION claim 7 expects parity-with-gtm-engineer. This brief brings real-estate-agent to 6 skills.

## Scope

5 NEW skill files under `verticals/real-estate-agent/skills/`:

1. **`schedule_viewing.skill.md`** — given a qualified lead + property, propose 3 viewing time options factoring agent availability, property access (lockbox / on-site agent / by appointment), commute time, weekday vs weekend preference. Output JSON. Autonomous per charter.
2. **`send_listing_packet.skill.md`** — given a qualified lead, draft a listing packet (property facts + neighborhood comps + recent sales + school district + tax history). Output Markdown + structured metadata. Approval-gated per charter ("client-facing").
3. **`draft_offer.skill.md`** — given a buyer's interest + property + market comps, draft an offer document: purchase price, contingencies (inspection / financing / appraisal), earnest money, close date, requested concessions. Output structured JSON mapping to standard offer form fields. **Always approval-gated** — financial commitment instrument.
4. **`flag_inspection_finding.skill.md`** — given an inspection report, categorize findings by severity (cosmetic / functional / safety / structural / environmental) + estimated remediation cost band. Output JSON. Flags safety + structural + environmental findings as "requires immediate operator review" regardless of remediation cost. Approval-gated.
5. **`follow_up_lead.skill.md`** — given a lead + last-contact timestamp + lead-state (browsing / qualified / under-contract / lost), draft a templated follow-up message matching the state. Output Markdown. Autonomous per charter; never auto-sends to leads (the runner enforces send-gating; the skill just produces the draft).

Each skill ~50-90 LOC. Total: ~350-450 LOC.

This is NOT in scope: live MLS integration; auto-sending client-facing communications; e-signature integration (DocuSign / Dotloop); real comps API (operator-provided comps for v1).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/real-estate-agent/skills/schedule_viewing.skill.md` (NEW, ≤ 80 LOC)
- `verticals/real-estate-agent/skills/send_listing_packet.skill.md` (NEW, ≤ 90 LOC)
- `verticals/real-estate-agent/skills/draft_offer.skill.md` (NEW, ≤ 90 LOC)
- `verticals/real-estate-agent/skills/flag_inspection_finding.skill.md` (NEW, ≤ 90 LOC)
- `verticals/real-estate-agent/skills/follow_up_lead.skill.md` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- `verticals/real-estate-agent/skills/lead_qualification.md` — existing skill stable
- `verticals/real-estate-agent/vertical.toml`, prompts — locked
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 5 new `.skill.md` files
- [ ] Each has frontmatter + sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`, `# Approval posture`
- [ ] `schedule_viewing` + `follow_up_lead` marked autonomous; `send_listing_packet` + `draft_offer` + `flag_inspection_finding` marked `requires_approval: true`
- [ ] `draft_offer` is the most safety-critical — output explicitly cites the financial-commitment framing
- [ ] `flag_inspection_finding` 5-severity vocabulary documented (cosmetic / functional / safety / structural / environmental); safety/structural/environmental always require operator review regardless of cost
- [ ] No skill auto-sends — listing packet, offer doc, and follow-up message are all drafts
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-real-estate-agent-skills`
- PR title: `verticals: 5 skills for real-estate-agent (mission-vertical-real-estate-agent-skills)`

## Notes for the implementer

- Real-estate is a fiduciary-duty profession in most jurisdictions. The persona's `# Hard rules` should reinforce: never represent both sides without disclosure; never make legal claims (refer to attorney); never quote a specific list-to-sale ratio as a prediction.
- `draft_offer` is the highest-stakes skill. Every field has a financial consequence. The output JSON should map clearly to standard offer form fields so the operator can review line-by-line.
- `flag_inspection_finding`'s 5-category vocabulary is durable — keep the order (cosmetic / functional / safety / structural / environmental) in every example so downstream consumers can rely on it.
- `follow_up_lead` autonomous-draft-but-never-auto-send framing is the same charter approach as marketing-intern: the SKILL never sends, the runner does. The skill should NOT include "send" logic.
- For `send_listing_packet`, prefer Markdown body + a JSON metadata sidecar (property_address, listing_id, comp_count, generated_at) so the runner can audit which listings went to which leads.
