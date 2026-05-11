# Mission load-bearing wire — `research-analyst` vertical

**Owner**: claude. **Depends on**: existing vertical pattern (verified at `verticals/marketplace-seller-browser/`).
**Authored against**: origin/main `d10658d5` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-research-analyst`.

> SOURCE: MISSION_BACKLOG.md Section 11 #149 — "Research-analyst vertical." Deep-dive deskwork: sectoral research, competitor scans, regulatory updates, citation-grade memos. Outputs are markdown reports with source links + confidence calls. Lower-touch than lead-research (which is sales-supporting).

## Scope

NEW `verticals/research-analyst/` (content-only) mirroring `verticals/marketplace-seller-browser/`:

1. `vertical.toml`:
   - `[vertical].id = "research-analyst"`, version `0.1.0`
   - `[role]` persona = experienced sector analyst, slightly skeptical, sources every claim
   - `[tools].allowed` = `["web_search", "web_extract", "file_read"]`
   - `[mcp_servers]` = `{ similarweb = {...} }` (optional)
   - `[escalation].human_approval_required` = `["publish_report"]`

2. `prompts/system.md` (≤ 200 LOC) — covers: source hierarchy (primary > tier-1 secondary > tier-2 > industry blogs), claim-citation requirement, when to escalate (regulated industry without licensed source, MNPI risk, contradicting authoritative source).

3. `prompts/persona.md` (≤ 100 LOC).

4. `skills/sector_overview.md` — given sector + scope, produce 1-pager with market size + 5 players + 3 trends.

5. `skills/competitor_scan.md` — given target company, produce 5-competitor matrix (pricing / segments / strengths / weaknesses).

6. `skills/regulatory_brief.md` — given jurisdiction + topic, summarize current regs + recent changes + 3 horizon items.

7. `skills/source_credibility_check.md` — given a URL, rate the source on tier-1/2/3 + confidence + caveats.

8. `memory/schema.toml` — namespaces: `prior_reports`, `trusted_sources`, `sector_taxonomies`, `flagged_claims`.

9. `eval/seed.jsonl` (≥ 7 fixtures): 3 clear-research, 2 source-credibility, 2 escalation.

10. `auto_approval.toml` — read + draft all auto. Publish requires human.

11. `inventory.json` — sample sector taxonomies for v1 demo.

This is NOT in scope: financial modeling (deferred to a numerical-analyst vertical); academic-paper-summarization (separate); paywall scraping (sourcing rules forbid).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/research-analyst/vertical.toml` (NEW)
- `verticals/research-analyst/prompts/system.md` (NEW, ≤ 200 LOC)
- `verticals/research-analyst/prompts/persona.md` (NEW, ≤ 100 LOC)
- `verticals/research-analyst/skills/*.md` (4 NEW skill files, ≤ 80 LOC each)
- `verticals/research-analyst/memory/schema.toml` (NEW, ≤ 40 LOC)
- `verticals/research-analyst/eval/seed.jsonl` (NEW, ≥ 7 fixtures)
- `verticals/research-analyst/auto_approval.toml` (NEW, ≤ 40 LOC)
- `verticals/research-analyst/inventory.json` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Any other `verticals/<name>/` directory
- Any `crates/**` source
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `argo vertical list` includes `research-analyst`
- [ ] `argo vertical lint research-analyst` reports no errors
- [ ] `argo vertical show research-analyst --json` returns valid envelope
- [ ] All 4 skill files have correct frontmatter
- [ ] eval/seed.jsonl has ≥ 7 fixtures
- [ ] auto_approval.toml gates publish behind human
- [ ] No code changes
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-research-analyst`
- PR title: `vertical: research-analyst (mission-vertical-research-analyst)`

## Notes for the implementer

- The "claim must have citation" rule is the most important content — every skill prompt should mandate inline citations or note the unsourced claim explicitly.
- For eval/seed.jsonl: avoid real, named companies. Use anonymized patterns ("a 50-person SaaS in X sector").
- The MNPI (material non-public information) escalation criterion is load-bearing for regulated industries — document it explicitly.
