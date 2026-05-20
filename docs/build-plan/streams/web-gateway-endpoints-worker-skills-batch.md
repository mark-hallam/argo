# web-gateway-endpoints-worker-skills-batch — implement 5 missing worker-skills endpoints

**Owner**: claude (or codex).
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-worker-skills-batch`.
**Priority**: P1 — wave-3 API audit "single highest-priority gateway endpoint that's missing" is exactly `GET /api/v1/workers/<id>/skills`. The design-locked worker-skills tab is 100% mock today.
**Sizing**: ~500-900 LOC (5 handlers + lookup logic + tests).

## Why this brief exists

From the wave-3 API-wiring audit:

> All 5 worker-skills endpoints (GET/POST/DELETE
> /api/v1/workers/<id>/skills*) — the Active/Library/Learned tab
> is 100% mock.
>
> **Single highest-priority missing endpoint:**
> `GET /api/v1/workers/<id>/skills`. It closes the worker-detail
> page's biggest mock surface and unblocks the "AI worker learns
> and gets promoted" narrative locked in the 2026-05-19 worker-
> detail design.

This is also a narrative unblocker — the wave-1 mission audit's #2
pillar ("learn from your feedback") demands a visible skills tab
on worker pages.

## Endpoints to implement

1. `GET /api/v1/workers/<id>/skills` — list active + library + learned skills for a worker. Envelope `{ active: [...], library: [...], learned: [...], total_counts }`.
2. `POST /api/v1/workers/<id>/skills` — add a skill from the library to active. Body `{ skill_id, trust_grade_min }`.
3. `DELETE /api/v1/workers/<id>/skills/<skill_id>` — remove an active skill.
4. `GET /api/v1/workers/<id>/skills/<skill_id>/history` — when this worker learned/earned/used this skill.
5. `POST /api/v1/workers/<id>/skills/<skill_id>/promote` — promote a learned skill into active (trust ratchet UX).

## Backing storage

`worker_skill` table already exists (per `mvp-worker-skills-table`
#1272). May need additive column: `state` (active/library/learned/promoted)
or join to a sibling enum table.

## Steps

1. Check `crates/argo-store/src/worker_skills.rs` (or wherever the table lives) for current API.
2. Decide: add `state` column via additive migration OR derive state from existing fields (trust_grade + last_used + provenance).
3. Add gateway handlers under `crates/argo-cli/src/handlers/gateway/routes_worker_skills.rs`.
4. Each handler validates that `<id>` belongs to the caller's tenant (cross-tenant guard).
5. Wire into routes-dispatch.
6. Tests: CRUD + state-transition + cross-tenant negative + promote-trust-up.

## may-touch

- `crates/argo-store/src/worker_skills.rs` (additive)
- Optional migration if state column needed
- New: `crates/argo-cli/src/handlers/gateway/routes_worker_skills.rs`
- New: tests file
- `crates/argo-cli/src/handlers/gateway/routes.rs` (dispatch wire)
- `web/lib/api/worker-skills.ts` (only if envelope shape changes)

## must-NOT-touch

- `argo-skill` crate substrate (the skill-library lookup) — this brief only wires the gateway facade.
- Trust-grade computation (separate concern).
- Web-side mock fallback (leave for graceful degradation).

## Acceptance criteria

- [ ] All 5 endpoints return 200 with realistic data; cross-tenant returns 404.
- [ ] Promote endpoint moves a learned skill into active and writes an audit row (so the trust ratchet is visible in /audit).
- [ ] `cargo clippy -p argo-store -p argo-skill -p argo-cli --all-targets -- -D warnings` clean.
- [ ] At least 8 unit tests + 1 integration smoke.
- [ ] `web/app/(app)/workers/[workerId]/skills/page.tsx` shows live data instead of mock.

## References

- Audit: `docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md`
- Operator memory: `project_worker_detail_design_locked.md`
- Prior brief: #1272 `mvp-worker-skills-table`
