# Mission load-bearing wire — 25+ starter-library skills (vertical-agnostic)

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 5 #164. Today every vertical's skills are vertical-specific. Operators picking up SKYCOT want a starter library of vertical-AGNOSTIC skills (summarize-document, draft-email, schedule-meeting, etc.) that compose into custom workflows without needing to author from scratch.

## Scope

NEW `verticals/_starter-library/skills/` directory (the underscore prefix marks it as non-vertical, library-tier). 25 skills covering the universal operator-helper surface:

**Communication** (5): `summarize_long_doc` / `draft_polite_decline` / `draft_meeting_agenda` / `compose_status_update` / `paraphrase_for_audience`
**Scheduling** (3): `find_calendar_overlap` / `propose_meeting_options` / `convert_timezone`
**Data** (4): `extract_structured_data` / `validate_against_schema` / `compare_two_documents` / `tabulate_loose_data`
**Decision support** (4): `weigh_pros_cons` / `summarize_stakeholder_views` / `flag_decision_blockers` / `rank_options_by_criteria`
**Research** (3): `cite_sources_for_claim` / `find_credible_counterpoint` / `summarize_thread_for_skim`
**Numeric** (3): `sanity_check_arithmetic` / `estimate_with_bounds` / `convert_units`
**Process** (3): `propose_runbook_steps` / `flag_missing_step` / `draft_post_mortem_template`

Each skill ~40-70 LOC of markdown frontmatter + behavior + examples. Total: ~1500 LOC of skill-file content.

This is NOT in scope: registering the starter library as a "vertical" (it's a library accessed by other verticals via `extends:`); cross-skill orchestration; per-tenant customization of starter skills.

## Files you may touch

- `verticals/_starter-library/` (NEW directory tree)
  - `vertical.toml` (NEW, ≤ 40 LOC) — minimal: marks this as a library tier, NOT a deployable vertical
  - `skills/<skill_name>.skill.md` × 25 (NEW, each ≤ 80 LOC)
  - `README.md` (NEW, ≤ 200 LOC) — index + usage from other verticals

## Files you must NOT touch

- Existing per-vertical skills (these aren't replacements; they're library)
- The skill-loader code (assumed to discover `verticals/*/skills/` paths)

## Acceptance criteria

- [ ] 25 skill files across 7 categories
- [ ] Each file has frontmatter (name / version / description / provenance: `{ source: vertical/_starter-library, hash: sha256-pending }`)
- [ ] Each has sections (When / Inputs / Output / Examples / Approval posture)
- [ ] README documents the library + per-vertical inclusion pattern
- [ ] No file > 100 LOC; total directory ≤ 1700 LOC markdown
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-starter-skill-library-25`
- PR title: `verticals: 25-skill starter library (mission-starter-skill-library-25)`

## Notes

- The library lives under `verticals/_starter-library/` (underscore prefix) to signal "not a deployable vertical, library-tier". Other verticals reference these by name from their prompts; the skill-loader treats them no differently.
- Bias toward vertical-AGNOSTIC tasks. If a skill could equally apply to a salesperson, a doctor, a lawyer, and an event coordinator — it belongs here. Vertical-specific skills stay in their vertical.
- Per-skill examples: use placeholders, not domain-specific fixtures (`<input topic>` not "marketing newsletter draft").
