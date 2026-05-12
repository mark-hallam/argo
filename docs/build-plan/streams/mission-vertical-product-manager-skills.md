# Mission load-bearing wire — Product Manager vertical: 5+ skills

**Owner**: claude. **Depends on**: existing `verticals/product-manager/` charter stub.
**Authored against**: origin/main `e2441425` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-product-manager-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to product-manager. Today the vertical ships with 1 skill (`prd_template.skill.md`). MISSION claim 7 expects parity-with-gtm-engineer. This brief brings product-manager to 6 skills.

## Scope

5 NEW skill files under `verticals/product-manager/skills/`:

1. **`triage_feedback.skill.md`** — given a batch of inbound user feedback (Intercom / Zendesk / email), cluster into themes (≤8), bucket by surface area + severity, identify duplicate signal, output JSON: `{themes: [{name, count, severity, sample_quotes, surface_area}]}`. Autonomous per charter (internal-only output).
2. **`draft_prd.skill.md`** — given a feature request + linked themes from `triage_feedback`, draft a PRD using the existing `prd_template`: problem statement, success criteria, scope/out-of-scope, risks, sequencing. Output Markdown. Internal-draft per charter — approval-gated only when routed to engineering for commitment.
3. **`schedule_review.skill.md`** — propose cross-functional review meeting times (Eng/Design/PM/CS) factoring panel availability, time-zone, prep-time-buffer. Output JSON. Autonomous per charter (internal scheduling).
4. **`summarize_user_research.skill.md`** — given raw interview notes / survey responses, output a 5-bullet executive summary + supporting evidence map + open questions. Cites every claim back to source. Autonomous per charter.
5. **`update_public_roadmap.skill.md`** — propose roadmap updates (add / move / reprioritize). **Approval-gated per charter** ("only roadmap commits and document_creation route through human approval"). Output: diff envelope showing before/after + reasoning + dependency check.

Each skill ~50-90 LOC. Total: ~350-450 LOC.

This is NOT in scope: live Intercom/Zendesk integration; auto-publishing to public roadmap; Linear/Jira write; OKR setting.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/product-manager/skills/triage_feedback.skill.md` (NEW, ≤ 80 LOC)
- `verticals/product-manager/skills/draft_prd.skill.md` (NEW, ≤ 90 LOC)
- `verticals/product-manager/skills/schedule_review.skill.md` (NEW, ≤ 70 LOC)
- `verticals/product-manager/skills/summarize_user_research.skill.md` (NEW, ≤ 80 LOC)
- `verticals/product-manager/skills/update_public_roadmap.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- `verticals/product-manager/skills/prd_template.skill.md` — existing skill stable
- `verticals/product-manager/vertical.toml`, prompts — locked
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 5 new `.skill.md` files
- [ ] Each has frontmatter + sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`, `# Approval posture`
- [ ] `update_public_roadmap` marked `requires_approval: true` per charter; other 4 marked autonomous
- [ ] `triage_feedback` caps themes at 8 (operator attention budget)
- [ ] `summarize_user_research` cites every claim back to source (no fabrication)
- [ ] `draft_prd` is shape-compatible with the existing `prd_template` (so the two compose without translation)
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-product-manager-skills`
- PR title: `verticals: 5 skills for product-manager (mission-vertical-product-manager-skills)`

## Notes for the implementer

- Read the existing `prd_template.skill.md` BEFORE writing `draft_prd` — match the template's section names and order. `draft_prd` is the "fill in the template" skill; `prd_template` is the structural skeleton.
- `triage_feedback`'s 8-theme cap is operator-meaningful: more themes mean less actionable signal. If the input has >8 clusters, the skill should output the top 8 by signal-weight and emit `truncated: true`.
- `update_public_roadmap` is the only approval-gated skill — every output frame should make the diff visible (before → after) so the operator can approve atomically.
- The product-manager persona's `# Hard rules` should reinforce: NEVER commit to public roadmap without approval; NEVER fabricate user quotes (cite back to source or refuse).
