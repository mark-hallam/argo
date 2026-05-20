# web-workflow-rollback-feature — ship the workflow rollback feature

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-rollback-feature`.
**Priority**: P1 — wave-5 close-the-loop found "One-tap rollback" was a broken promise. Wave-5 fix softened landing copy to "Rollback shipping next" — this brief delivers the feature so we can promote the claim back.
**Sizing**: ~500-800 LOC across argo-store (rollback ops) + gateway (endpoint) + web (UI + confirmation modal) + tests.

## Why this brief exists

From the wave-5 close-the-loop audit (`docs/skycot/audit/close-the-loop/2026-05-20-wave-5-close-the-loop.md`):

> **"One-tap rollback"** — MISSING entirely. DB substrate exists in
> `argo-store/src/workflow_definition.rs` ("upcoming") but no API,
> no UI, no route. Promised on both /landing and /pricing.

The DB substrate exists; the feature is undelivered. Once this ships,
the marketing copy ("Rollback shipping next") can revert to "One-tap rollback."

## Goals

1. **DB op**: `WorkflowStore::rollback_to(workflow_id, version)` — creates a new version that's an exact copy of the named older version (additive, never destructive).
2. **Gateway endpoint**: `POST /api/v1/workflows/<id>/rollback` — body `{ target_version: number }`. Returns the new version number.
3. **Web UI**: on `/workflows/[v]/[name]` viewer, every prior version row has a "Rollback to v<N>" button. Click → confirmation modal showing diff vs current → confirm → API call → toast + reload to viewer at new version.
4. **Audit row**: every rollback writes `kind: "workflow_rolled_back"` to the chain (so the action shows in /audit).
5. **A11y**: confirmation modal has focus trap + Esc close + Enter confirm.

## Steps

1. Read `crates/argo-store/src/workflow_definition.rs` for the "upcoming" comment + existing versioning shape.
2. Implement `WorkflowStore::rollback_to` (additive — produces a new version pointing at older version's content).
3. Add gateway endpoint + cross-tenant guard.
4. Build `<RollbackConfirmModal>` showing the diff (reuse if a diff component exists, else add a simple side-by-side).
5. Wire button + modal into the viewer page.
6. Add audit-row emission on rollback success.
7. Tests: rollback creates new version; diff matches; cross-tenant 404; audit row written.

## may-touch

- `crates/argo-store/src/workflow_definition.rs` (the rollback op)
- New: `crates/argo-cli/src/handlers/gateway/routes_workflows_rollback.rs`
- `crates/argo-cli/src/handlers/gateway/routes.rs`
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (viewer)
- New: `web/components/skycot/workflows/RollbackConfirmModal.tsx`
- New: `web/lib/api/workflow-rollback.ts`
- Tests

## must-NOT-touch

- Workflow version-history schema (rollback is additive only — no schema change)
- The builder page (rollback is a viewer-page feature)

## Acceptance criteria

- [ ] `POST /api/v1/workflows/<id>/rollback` returns 200 + new version number.
- [ ] Web viewer: "Rollback to v<N>" button on every prior-version row.
- [ ] Confirmation modal shows the diff between current and target version.
- [ ] After rollback: new version visible at top of version history; current pointer moved.
- [ ] Audit row visible in /audit with `kind: "workflow_rolled_back"`.
- [ ] Cross-tenant returns 404.
- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Once this ships: revert `/landing` copy from "Rollback shipping next" back to "One-tap rollback".

## References

- Audit: `docs/skycot/audit/close-the-loop/2026-05-20-wave-5-close-the-loop.md`
- Promise source: `web/app/landing/page.tsx:46` (the softened "Rollback shipping next" line — restore to "One-tap rollback" in the merge commit)
