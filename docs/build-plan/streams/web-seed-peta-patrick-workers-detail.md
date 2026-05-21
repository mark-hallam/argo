# web-seed-peta-patrick-workers-detail

**Authored against**: `origin/main` 349b6acc on 2026-05-21.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-seed-peta-patrick-workers-detail`.

## Summary

Single-file follow-up to #1935. Peta + Patrick PM workers now exist in `sample-team` / `sample-audit` / `sample-cohorts` / `sample-worker-{overview,memory,skills,activity}`, but `sample-workers-detail.ts` was not in #1935's may-touch list. Result: `/workers/peta` and `/workers/patrick` hero falls through to "Worker not found" even though every tab below it renders correctly. This brief fills that one-file gap.

## Acceptance

- [ ] `web/lib/sample-workers-detail.ts` gains `PETA` and `PATRICK` entries with the same shape as existing entries (id, name, role, vertical, level, headline, traits, "great at" / "escalate when" copy lifted from operator memory `project_pm_cohorts_peta_patrick.md`).
- [ ] `/workers/peta` and `/workers/patrick` return HTTP 200 with hero rendered (not the not-found path).
- [ ] Detail copy is consistent with the worker-overview + cohorts copy that landed in #1935 (same headline, same trait set).
- [ ] No other file touched. Pure additive seed.
- [ ] Pre-existing 5-7 boundary-flake failures declared in PR body per HARD RULE 2.

## May touch

- `web/lib/sample-workers-detail.ts` (additive only).

## Must NOT touch

- Any other seed file (avoid re-litigating #1935's persona-spine + temporal landings).
- `web/components/skycot/cohorts/*` (locked D3 port).

## Notes

- Reference shape: read an existing entry (e.g. Sam or Mariah) and mirror it. Peta = Project Coordinator L2, Linear + GitHub stack; Patrick = Status Reporter L3, weekly-rollup specialist.
- Trait language: lift directly from `project_pm_cohorts_peta_patrick.md` memory (operator-locked voice).
- Authored in response to declared follow-up gap in #1935 PR body.
