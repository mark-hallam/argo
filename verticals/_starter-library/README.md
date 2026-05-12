# Starter Skill Library

Vertical-agnostic skills for the SKYCOT platform. 25 skills covering
the universal operator-helper surface — communication, scheduling,
data wrangling, decision support, research, numeric checking, and
process drafting.

This directory is **library tier**, not a deployable vertical. The
leading underscore on `_starter-library/` is the social convention
that signals this; the skill-loader treats these files identically
to any per-vertical `skills/` directory.

## Why it exists

Every vertical today (receptionist, gtm-engineer, sales-follow-up,
etc.) ships its own per-vertical skills, hand-written. Operators
picking up SKYCOT want a starter library of skills they can compose
into custom workflows without authoring each from scratch. The
starter library is that baseline.

## What's here

| Category | Count | Skills |
|---|---|---|
| Communication | 5 | `summarize_long_doc`, `draft_polite_decline`, `draft_meeting_agenda`, `compose_status_update`, `paraphrase_for_audience` |
| Scheduling | 3 | `find_calendar_overlap`, `propose_meeting_options`, `convert_timezone` |
| Data | 4 | `extract_structured_data`, `validate_against_schema`, `compare_two_documents`, `tabulate_loose_data` |
| Decision support | 4 | `weigh_pros_cons`, `summarize_stakeholder_views`, `flag_decision_blockers`, `rank_options_by_criteria` |
| Research | 3 | `cite_sources_for_claim`, `find_credible_counterpoint`, `summarize_thread_for_skim` |
| Numeric | 3 | `sanity_check_arithmetic`, `estimate_with_bounds`, `convert_units` |
| Process | 3 | `propose_runbook_steps`, `flag_missing_step`, `draft_post_mortem_template` |

Each skill file: `name`, `version`, `description`, `provenance`
frontmatter, plus sections for When-to-use, Inputs, Output,
Examples, Approval posture, and Notes.

## How a vertical uses these

A vertical references starter skills by name from its prompts —
the skill-loader discovers the files identically to any other
`verticals/*/skills/` directory. The mental model is:

1. The vertical's `vertical.toml` declares its own `skills/`
   bundle for vertical-SPECIFIC skills.
2. The vertical's system prompt or workflow can reference
   library skills by name (e.g. "use `summarize_long_doc` when
   the user pastes a memo").
3. The runtime resolves the reference against the union of the
   vertical's own bundle and `_starter-library/skills/`.

If a vertical needs a tweaked version of a starter skill (more
strict, different output shape, domain-specific examples), it
copies the file into its own `skills/` directory and edits there
— the per-vertical copy wins. The library is the floor, not the
ceiling.

## Design constraints

- **Vertical-agnostic**: if a skill could equally apply to a
  salesperson, a doctor, a lawyer, and an event coordinator, it
  belongs here. Anything narrower goes in the relevant vertical.
- **Placeholder examples**: no domain-specific fixtures. `<input
  topic>` not "marketing newsletter draft." This keeps the library
  re-usable across verticals without implying which vertical owns
  it.
- **Read-only by default**: most starter skills are diagnostic /
  drafting / synthesis. None auto-send, auto-execute, or mutate
  external state. Vertical-specific skills handle action-taking
  through the worker-v3 action layer; the library stays neutral.
- **Approval posture explicit**: every skill states whether it's
  pure computation, drafts-only, or diagnostic. No skill in this
  library is action-emitting.

## Adding to the library

A skill belongs here if **all** of these are true:

1. It would be useful to operators in ≥4 of the existing verticals
   without modification.
2. It has no domain-specific vocabulary in its body.
3. It's a single, focused capability — not an orchestration.
4. It does not require domain-specific tools (a starter skill
   that needs `<vertical-specific MCP>` belongs in that vertical).

If any of those are false, the skill belongs in the relevant
vertical's `skills/` directory.

## File budget

The library directory targets ≤ 1700 LOC of markdown total. No
single skill file exceeds 100 LOC. Adding a skill that pushes past
the budget should trigger a content review — likely the new skill
or an existing one is over-scoped.

## Versioning

Each skill carries its own `version` in frontmatter, starting at
`0.1.0`. The directory itself is versioned via `vertical.toml`.
Breaking changes to a skill's I/O shape require a major-version
bump and a migration note in the PR body.
