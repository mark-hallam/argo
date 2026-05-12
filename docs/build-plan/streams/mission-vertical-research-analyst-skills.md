# Mission load-bearing wire — Research-Analyst vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/research-analyst/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-research-analyst-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. research-analyst ships with 4 skills (competitor_scan / regulatory_brief / sector_overview / source_credibility_check). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`synthesize_signal_cluster.skill.md`** — given a batch of related observations (news items, filings, social signals), cluster into 3-5 themes + label each with confidence (high/medium/low) + cite evidence per theme. Output JSON. Caps themes at 5; emits `truncated: true` if input exceeds. Conservative on theme-naming (favors descriptive over evocative).
2. **`flag_thesis_disconfirming_evidence.skill.md`** — given an operator's stated thesis + recent evidence, identify the strongest disconfirming signals (≥3 if present). Output JSON: `{ disconfirmers: [{signal, source, recency, why_it_disconfirms, severity: weak|moderate|strong}], recommended_action: hold_thesis | revisit_in_30d | reconsider_now }`. Bias toward surfacing disconfirmers, not validating the thesis.

## Files you may touch

- `verticals/research-analyst/skills/synthesize_signal_cluster.skill.md` (NEW, ≤ 90 LOC)
- `verticals/research-analyst/skills/flag_thesis_disconfirming_evidence.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `synthesize_signal_cluster` 5-theme cap + 3-confidence vocab
- [ ] `flag_thesis_disconfirming_evidence` 3-severity vocab + ≥3-disconfirmer target + bias-toward-disconfirmation framing
- [ ] Both skills cite every claim back to source (no fabrication)
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-research-analyst-skills`
- PR title: `verticals: 2 skills for research-analyst (mission-vertical-research-analyst-skills)`

## Notes

- `flag_thesis_disconfirming_evidence` is the explicit antidote to confirmation bias. Document in `# Hard rules`: "this skill's job is to make the case against the thesis. Validation comes from elsewhere."
- The 5-theme cap on signal clustering is operator-respect — humans can't hold more than ~5 themes simultaneously. Truncation is honest, not lossy.
