# Mission load-bearing wire — `lead-research` vertical

**Owner**: claude. **Depends on**: existing vertical pattern (verified at `verticals/marketplace-seller-browser/`); existing MCP connectors (zoominfo/apollo/clearbit are TODO; vertical works in dry-run without them).
**Authored against**: origin/main `805deb35` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-lead-research`.

> SOURCE: MISSION_BACKLOG.md Section 11 #150 — "Lead-research vertical." Adds a vertical for prospect research: given a target company or ICP, enrich with key contacts, recent news, tech stack, and produce an outbound research brief.

## Scope

NEW `verticals/lead-research/` (content-only) mirroring `verticals/marketplace-seller-browser/`:

1. `vertical.toml`:
   - `[vertical].id = "lead-research"`, version `0.1.0`
   - `[role]` persona = sharp SDR-supporting researcher who writes 1-pagers
   - `[tools].allowed` = `["web_search", "web_extract", "file_read"]`
   - `[mcp_servers]` = `{ zoominfo = {...}, clearbit = {...}, similarweb = {...} }` (all optional — vertical works in dry-run)
   - `[escalation].human_approval_required` = `["send_outreach_email"]`

2. `prompts/system.md` (≤ 200 LOC) — covers: ICP definition, sources to consult, structured output format (firmographic / contacts / news / tech stack / talking points), confidence scoring.

3. `prompts/persona.md` (≤ 100 LOC).

4. `skills/research_company.md` — given company name+domain, produce structured 1-pager.

5. `skills/identify_decision_makers.md` — given company, return top 5 likely buying-committee contacts with titles + LinkedIn URLs (when web_extract permits).

6. `skills/draft_research_brief.md` — combine signals into a sales-ready brief.

7. `memory/schema.toml` — namespaces: `accounts`, `recent_research`, `icp_definitions`.

8. `eval/seed.jsonl` (≥ 7 fixtures): 3 clear-research-tasks, 2 ambiguous-icp, 2 escalation-needed.

9. `auto_approval.toml` — approves all research skills; gates outreach.

10. `inventory.json` — small ICP examples for v1 demo.

This is NOT in scope: shipping zoominfo/apollo/clearbit MCP connectors (separate briefs); CRM write-back (separate); shipping the outreach-sender vertical (separate `sales-sdr` already exists for that).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/lead-research/vertical.toml` (NEW)
- `verticals/lead-research/prompts/system.md` (NEW, ≤ 200 LOC)
- `verticals/lead-research/prompts/persona.md` (NEW, ≤ 100 LOC)
- `verticals/lead-research/skills/*.md` (3 NEW skill files, ≤ 80 LOC each)
- `verticals/lead-research/memory/schema.toml` (NEW, ≤ 40 LOC)
- `verticals/lead-research/eval/seed.jsonl` (NEW, ≥ 7 fixtures)
- `verticals/lead-research/auto_approval.toml` (NEW, ≤ 40 LOC)
- `verticals/lead-research/inventory.json` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Any other `verticals/<name>/` directory
- Any `crates/**` source
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `argo vertical list` includes `lead-research`
- [ ] `argo vertical lint lead-research` reports no errors
- [ ] `argo vertical show lead-research --json` returns valid envelope
- [ ] All 3 skill files have correct frontmatter
- [ ] eval/seed.jsonl has ≥ 7 fixtures
- [ ] auto_approval.toml gates outreach skills
- [ ] No code changes
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-lead-research`
- PR title: `vertical: lead-research (mission-vertical-lead-research)`

## Notes for the implementer

- The research-brief output format is the most important deliverable — get the structure right (firmographic / contacts / news / tech stack / talking points + confidence) before the prompts.
- For eval/seed.jsonl: pick realistic company names (avoid real people, use fictional reps if naming any).
- The MCP connectors are intentionally optional — `argo vertical lint` should pass even without them installed.
