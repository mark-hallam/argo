# Mission load-bearing wire — Lead-Research vertical: 3+ skills to reach 6

**Owner**: claude. **Depends on**: existing `verticals/lead-research/` charter stub.
**Authored against**: origin/main `df7b9dfe` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-lead-research-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137, scoped to lead-research. Today the vertical ships with 3 skills (draft_research_brief / identify_decision_makers / research_company). MISSION claim 7 expects parity-with-gtm-engineer. This brief brings lead-research to 6.

## Scope

3 NEW skill files under `verticals/lead-research/skills/`:

1. **`detect_buying_signals.skill.md`** — given a company + recent news / web-extract output, identify buying signals: hiring posts (proxy for direction), funding events, exec moves, product launches, technology migrations, regulatory pressure. Output JSON: `[{signal, signal_type, evidence, recency, confidence}]`. Cites every signal with source URL + date. Confidence one of: high (verified primary source) / medium (inferred from credible secondary) / low (single-source rumor).
2. **`score_lead_fit.skill.md`** — given an ICP definition + a researched company, score fit against ICP dimensions (segment / size / tech-stack / pain-signal / geography / vertical). Output JSON: `{overall_fit: 0-100, dimension_breakdown, disqualifiers: [...], recommended_priority: high|medium|low|disqualify}`. Disqualifier path is preferred over forcing low — keeps the operator's pipeline clean.
3. **`rank_priority_list.skill.md`** — given N researched companies, produce a ranked priority list factoring fit_score, recency of buying signals, sales-cycle-length-estimate, and operator-provided business-context weights. Output Markdown table + JSON. Up to 50 companies; emits `truncated: true` if input exceeds.

Each skill ~50-80 LOC. Total: ~200-250 LOC.

This is NOT in scope: live CRM enrichment integration (uses MCP connectors when present); auto-outreach (sales-sdr handles that with approval gating); contact-info-scraping (PII concerns — operator-provided or via approved enrichment connector only).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/lead-research/skills/detect_buying_signals.skill.md` (NEW, ≤ 90 LOC)
- `verticals/lead-research/skills/score_lead_fit.skill.md` (NEW, ≤ 90 LOC)
- `verticals/lead-research/skills/rank_priority_list.skill.md` (NEW, ≤ 80 LOC)

## Files you must NOT touch

- Existing skill files `draft_research_brief.md` / `identify_decision_makers.md` / `research_company.md`
- `verticals/lead-research/vertical.toml`, prompts — locked
- Any other vertical's skill directory

## Acceptance criteria

- [ ] 3 new `.skill.md` files
- [ ] Each has frontmatter + sections: `# When to use`, `# Inputs you expect`, `# Output shape`, `# Examples`
- [ ] `detect_buying_signals` outputs deterministic 3-confidence vocabulary (high / medium / low) with explicit source per signal
- [ ] `score_lead_fit` includes the disqualifier path explicitly (preferred over forcing low score)
- [ ] `rank_priority_list` caps at 50 + truncated flag
- [ ] No fabricated signals — every claim cited or marked inferred (charter mandate)
- [ ] No outreach skills (sibling vertical `sales-sdr` handles that)
- [ ] No file > 100 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-lead-research-skills`
- PR title: `verticals: 3 skills for lead-research (mission-vertical-lead-research-skills)`

## Notes for the implementer

- Match the source-citation tone of `research_company.md` (the existing skill) — verified-vs-inferred is the durable framing.
- `detect_buying_signals` is the highest-leverage new skill. Salespeople prize timing — the recency field should be explicit (within 7d / 30d / 90d / older) and high-confidence signals should win even if dimension fit is weaker.
- `score_lead_fit` and gtm-engineer's existing `research_company` overlap on dimensions but score differently — gtm-engineer is for active campaigns, lead-research is for top-of-funnel ICP screening. Document the difference in `# When to use` so workers pick the right one.
- The 6-dimension fit score (segment/size/tech-stack/pain/geo/vertical) is the canonical vocabulary. Don't drift it across the skill set.
- `rank_priority_list` is operator-facing — operators will eyeball the top 10. The Markdown table format is load-bearing for human review; the JSON sidecar is for downstream systems.
