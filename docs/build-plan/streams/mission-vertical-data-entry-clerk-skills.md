# Mission load-bearing wire — Data-Entry-Clerk vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/data-entry-clerk/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-data-entry-clerk-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. data-entry-clerk ships with 4 skills (extract_table / normalize_address / parse_invoice / validate_against_schema). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`deduplicate_records.skill.md`** — given a batch of records + a match key (or fuzzy keys), flag duplicates with confidence (high/medium/low) + merge recommendation (auto / operator-review / leave). Output JSON. Auto-merge only at confidence ≥ 0.95; everything else routes to operator.
2. **`reconcile_against_source.skill.md`** — given an internal record + a source document (PDF/scan/email), diff field-by-field and emit a reconciliation report with disagreement count + suggested resolution per field. Output Markdown + JSON. Approval-gated for any record mutation.

## Files you may touch

- `verticals/data-entry-clerk/skills/deduplicate_records.skill.md` (NEW, ≤ 90 LOC)
- `verticals/data-entry-clerk/skills/reconcile_against_source.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `deduplicate_records` 3-confidence vocab + 0.95 auto-merge threshold explicit
- [ ] `reconcile_against_source` field-level diff format + approval-gated
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-data-entry-clerk-skills`
- PR title: `verticals: 2 skills for data-entry-clerk (mission-vertical-data-entry-clerk-skills)`

## Notes

- 0.95 auto-merge threshold is operator-tunable in production but the skill defaults are load-bearing — too low → silent data corruption.
- `reconcile_against_source` should produce a side-by-side table when output is Markdown; operators eyeball diffs faster than they read prose.
