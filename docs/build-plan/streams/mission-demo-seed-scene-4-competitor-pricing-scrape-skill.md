# mission-demo-seed-scene-4-competitor-pricing-scrape-skill

**Owner**: claude.
**Authored against**: origin/main `bec35dd45` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-seed-scene-4-competitor-pricing-scrape-skill`.
**Depends on**: `mission-demo-seed-crate-scaffold` (#2272 IN_REVIEW) — uses the same seed CLI infrastructure.

🔴 **P0 DEMO-GATE — Scene 4 search beat unrecordable without this specific skill.**

`v1-mission-mvp-runbook.md` Scene 4 (line ~213): "Operator searches **competitor pricing scrape**. Three results." The current marketplace catalog (whether mock fixture or seeded backend) does NOT include a skill with that name. Scene 4's talk track breaks at the search step.

Per the POC/investor demo positioning doc (`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`), this IS a blocker — the camera literally captures the search input and result list.

## Scope

Extend the demo-seed CLI / library to include:

- 1 × hero skill: **"Competitor pricing scrape"** — a learned skill that scrapes a competitor's public price page via headless browser + computer-use macro, returns a normalized price ladder. Trust grade `LearnedReviewed` or `PublishedCommunity`. Author = one of the canon roster workers (Mariah is the most thematically fitting per Scene 1 narration about marketplace pricing).
- 2 × sibling skills the search "**competitor pricing scrape**" returns as results 2 and 3 (per runbook "Three results"):
  - **"Competitor feature audit"** — scrapes competitor's public marketing site for feature claims; normalizes to comparable structure.
  - **"Repricing rules-engine bulk apply"** — applies a price ladder to N marketplace listings via existing MCP marketplace connector.

The seed CLI's skill-catalog branch must insert these 3 rows into `skill_ir` with real `tenant_id` + real ULID skill IDs + valid manifests. The marketplace search index (whatever powers `GET /api/v1/skills?q=`) must return them ranked 1/2/3 for the query string "competitor pricing scrape".

## Acceptance criteria

- [ ] After running `argo demo seed --tenant <demo>`, `curl 'http://localhost:8088/api/v1/skills?q=competitor+pricing+scrape'` returns exactly 3 results with the hero result at index 0.
- [ ] Each result has a valid `skill_id` ULID, a `name`, a `description`, an `author_worker_id` (one of the canon roster), and a `trust_grade` in `{LearnedReviewed, PublishedCommunity}`.
- [ ] The hero skill renders correctly in the SPA's `/skills` marketplace page (verify with a curl-fetch + grep for the skill name in the HTML).
- [ ] Cross-tenant invisibility: a different tenant's marketplace query returns 0 results for the same query.
- [ ] PR body includes a screenshot OR `curl | jq` output showing the search query against the seeded data.

## May-touch

- `crates/argo-demo-seed/src/**` (the seed library — extend with a skill-catalog branch)
- `crates/argo-cli/src/handlers/demo_seed.rs` (wire the new branch into the canonical seed flow)
- `crates/argo-skill/src/sample_skills.rs` or similar IF the existing sample-skill module is the SOURCE OF TRUTH for skill manifests; if so, ADD the 3 skill manifests there (read-only consumed by the seed CLI).
- This brief file (status updates)

## Out of scope

- Changing the marketplace search algorithm (use existing `LayeredLookup` or whatever the catalog query uses)
- Adding real production scraping logic — these are demo skills, the action payload can be the stub-marker pattern from `feedback_no_theater_in_demo`
- Re-authoring the runbook (Scene 4 text stays as-is; this brief makes the data match the script)

## Effort

~150-250 LOC + 3 skill manifests; ~60-90 min.
