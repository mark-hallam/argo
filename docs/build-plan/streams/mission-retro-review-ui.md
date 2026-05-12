# Mission load-bearing wire — Retro-review UI

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 6 #209. After a worker completes a batch of actions (or a long-running duty wraps), the operator wants a retrospective view: what went well, what surprised, what would change for next time. Today: scroll through audit history. Add a structured retro surface.

## Scope

NEW `/retro` route + worker-retro dialog:

1. **Backend** `POST /api/v1/workers/<id>/retros` creates a retro record over a time-window (default last 7 days). Records aggregated metrics (actions_total, actions_approved, actions_refused, escalations, learned_skills_added, error_count_by_category) + a 5-section operator-prompt template.
2. **Frontend** `WorkerRetroDialog` — multi-step form (Surface metrics → What went well → What surprised → What to change → Action items). Each section operator-fillable; saves draft.
3. **Backend** `PATCH /api/v1/workers/<id>/retros/<retro_id>` saves draft; `POST .../finalize` locks it.
4. **`/retro` index route** lists historical retros across all workers with searchable themes.
5. **Audit row** per retro creation + finalization (kinds: `retro.created`, `retro.finalized`).

This is NOT in scope: cross-worker retro aggregation (one worker per retro in v1); retro templates per vertical (one canonical template); retro-feeds-skill-learning (separate brief — feedback-loop wiring).

## Files you may touch

- `crates/argo-store/migrations/72__retro.sql` (NEW, ≤ 60 LOC) — `retro` table (id, worker_id, window_start, window_end, draft_json, finalized_at, sections_json)
- `crates/argo-store/src/retro.rs` (NEW, ≤ 320 LOC) + tests
- `crates/argo-cli/src/handlers/gateway/routes_retros.rs` (NEW, ≤ 400 LOC) + tests
- `web/components/skycot/worker/WorkerRetroDialog.tsx` (NEW, ≤ 380 LOC)
- `web/app/(app)/retro/page.tsx` (NEW, ≤ 320 LOC) — index
- `web/lib/api/retros.ts` + `use-retros.ts` (NEW, ≤ 200 LOC each)
- `docs/skycot/RUNBOOK.md` — "Retros" subsection

## Files you must NOT touch

- The audit chain (retros consume audit data; don't mutate it)

## Acceptance criteria

- [ ] Migration 72 applies cleanly
- [ ] Retro auto-aggregates metrics over time window
- [ ] 5-section operator template renders + saves draft
- [ ] Finalize locks (no more edits)
- [ ] Index route searchable by theme
- [ ] 6 backend tests + 8 frontend tests; clippy + pnpm test clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-retro-review-ui`
- PR title: `retros: worker-retrospective UI (mission-retro-review-ui)`

## Notes

- The 5-section template (Surface metrics / What went well / Surprises / What to change / Action items) is the load-bearing structure — operators write better retros against a template than blank text.
- Auto-aggregated metrics give the retro a factual anchor; the operator-filled sections give it narrative shape.
- Migration slot 72 follows the header-pinned slots 63-70 + slot 71 from approval-bundle brief.
