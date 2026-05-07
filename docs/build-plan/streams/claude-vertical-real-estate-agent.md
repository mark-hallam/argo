# V2.3 — Real Estate Agent demo vertical

**Owner**: claude. **Depends on**: G-genesis (#790),
claude-vertical-finance-ar (#937),
C-gcal + C-gmail merged. Vertical demo #8.

## Scope

Real Estate Agent Assistant demo vertical — proves the
platform handles client-facing sales flows with mixed
trust tiers. Persona "Jamie" handles agent duties:
- Schedules property viewings (auto — calendar)
- Sends listing packets to qualified leads (gated email)
- Drafts offer documents based on agent input
- Flags inspection findings for agent review
- Follows up on warm leads after viewings (auto — templated)

High-value demo because (a) RE is a $200B+ commission
industry with massive admin overhead, (b) shows the
"executive assistant for a single licensed professional"
pattern, (c) demonstrates the document_creation executor
producing offer letters.

## What lands

11 NEW files (no NEW crate — re-use existing executors):

- `verticals/real-estate-agent/vertical.toml` — manifest
  (persona=Jamie, allowed_executors=[gcal, email,
  document_creation, human_approval])
- `verticals/real-estate-agent/persona.toml` — voice
  (enthusiastic, professional, never makes binding
  representations)
- `verticals/real-estate-agent/system_prompt.md` — duties,
  fair-housing rules, escalation triggers
- `verticals/real-estate-agent/memory_schema.toml` —
  lead_id, viewing_history, property_interests,
  offer_drafts, inspection_notes
- `verticals/real-estate-agent/eval/seeds.jsonl` — 10
  scenarios (new lead / repeat buyer / multi-offer / failed
  inspection / cold lead nurture)
- `verticals/real-estate-agent/inventory/sample_listings.json`
  — 8 fake listings (mix of price points + types)
- `verticals/real-estate-agent/skills/lead_qualification.md`
  — questions to ask, red-flag patterns
- `verticals/real-estate-agent/workspace/auto_approval.toml`
  — schedule_viewing + follow_up_lead auto; rest gated
- `verticals/real-estate-agent/actions/` — 5 action defs:
  - `schedule_viewing.toml` (auto)
  - `send_listing_packet.toml` (NotifyChannel.Email)
  - `draft_offer.toml` (NotifyChannel.DocumentCreation)
  - `flag_inspection_finding.toml` (NotifyChannel.Human)
  - `follow_up_lead.toml` (auto)
- `bin/demo_real_estate.sh` — end-to-end demo run
- `docs/verticals/real-estate-agent.md` — operator doc

## Acceptance criteria
- [ ] Vertical bootstraps via Genesis from charter
- [ ] Demo script runs end-to-end (mocked MLS OK)
- [ ] All 5 actions exercise
- [ ] draft_offer produces a markdown offer letter
- [ ] schedule_viewing + follow_up_lead run without prompts
- [ ] send_listing_packet + draft_offer + flag_inspection
      route to appropriate gates (3 different NotifyChannels)
- [ ] E2E smoke passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Total LOC ≤ 700 across the 11 files

## Branch + PR
- Branch: `claude-vertical-real-estate-agent`
- PR title: `verticals: Real estate agent demo vertical (claude-vertical-real-estate-agent)`

## Notes
- system_prompt MUST include fair-housing language: persona
  refuses to discriminate by protected classes
- Sample listings synthetic; do not include real addresses
- follow_up_lead uses templated copy with merge fields —
  templates live in inventory/follow_up_templates.md (one of
  the 11 files); auto-approval safe because no novel content
- 8 sample listings (vs 10 invoices in finance-ar) — RE
  needs richer per-listing data, fewer total
