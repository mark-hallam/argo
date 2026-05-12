# Mission load-bearing wire — Fundraising-Coordinator vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/fundraising-coordinator/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-fundraising-coordinator-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. fundraising-coordinator ships with 4 skills (draft_followup / research_investor / schedule_intro_call / track_pipeline_status). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`prep_pitch_deck_review.skill.md`** — given the current pitch deck + a target investor's known thesis/portfolio, produce a 5-bullet "what to emphasize" + 3-bullet "what to soften / cut for this audience" brief. Cites investor evidence per recommendation. Internal-output (no investor-facing artifact).
2. **`flag_term-sheet-redlines.skill.md`** — given a term sheet draft + operator's preferred terms (from `memory.fundraising_targets`), redline disagreements: valuation, liquidation preference, option-pool size, board seats, anti-dilution. Output structured JSON + Markdown summary. **Always approval-gated** (legal commitment instrument).

## Files you may touch

- `verticals/fundraising-coordinator/skills/prep_pitch_deck_review.skill.md` (NEW, ≤ 90 LOC)
- `verticals/fundraising-coordinator/skills/flag_term_sheet_redlines.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `prep_pitch_deck_review` cites every recommendation back to investor evidence
- [ ] `flag_term_sheet_redlines` explicitly approval-gated; covers the 5 named dimensions
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-fundraising-coordinator-skills`
- PR title: `verticals: 2 skills for fundraising-coordinator (mission-vertical-fundraising-coordinator-skills)`

## Notes

- `flag_term_sheet_redlines` is the highest-stakes skill in the vertical. Frame outputs as "the operator decides; this is research". Never propose accepting a term — only flag the gap.
- Don't make legal claims. Document this constraint in `# Hard rules` per skill.
