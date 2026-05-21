# web-workflow-save-button-real-wire

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-save-button-real-wire`.
**Priority**: **P0 — wave-15 buyer-trial blocker #6 (phase 1 of 3).**

## Summary

Phase 1 of 3 split from `web-workflow-save-publish-real-wire` (superseded). Wire the **Save as Draft** button on the workflow builder to `POST /api/v1/workflows/<vid>/<name>`. Persistence works end-to-end: user clicks Save → server stores the draft → reload preserves changes.

Publish wiring and concurrent-edit conflict UX are deferred to phase 2 (`web-workflow-publish-button-real-wire`) and phase 3 (`web-workflow-concurrent-edit-conflict-banner`).

## Acceptance

- [ ] `web/lib/api/workflows-save-publish.ts` (new, ~80 LOC) — thin wrapper `saveDraft({vertical_id, name, definition_toml, edit_reason?})` over the existing gateway endpoint.
- [ ] `web/lib/hooks/use-workflow-save-publish.ts` (new, ~60 LOC) — exposes `saveDraft` callback + `isSaving` + `lastError` state.
- [ ] Save button on the builder page (`build/page.tsx` or its toolbar subcomponent post-#1951 split) calls `saveDraft(...)` on click. Toast shows "Saved (v${result.version})" on success.
- [ ] Network/5xx/parse errors surface as red toast with a retry CTA.
- [ ] Page reload after Save preserves the canvas state (proof-of-persistence).
- [ ] Step-encoder round-trip test: encode canvas with every node kind → TOML → decode → assert structural equality. New file `web/app/(app)/workflows/[verticalId]/[name]/build/__tests__/step-encoder-roundtrip.test.ts`.
- [ ] Playwright E2E: open builder → drag a node → Save → reload → assert node still present. New file `web/tests/e2e/workflow-save-real.spec.ts`.
- [ ] No mock fallback in `isStrictProduction()` mode — 404/5xx propagates as error toast.
- [ ] `pnpm typecheck` + `pnpm lint` + `pnpm test --run` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.

## May touch

- `web/lib/api/workflows-save-publish.ts` (new)
- `web/lib/hooks/use-workflow-save-publish.ts` (new)
- `web/app/(app)/workflows/[verticalId]/[name]/build/BuilderToolbar.tsx` (post-#1951 — Save button lives there now)
- `web/app/(app)/workflows/[verticalId]/[name]/build/__tests__/step-encoder-roundtrip.test.ts` (new)
- `web/tests/e2e/workflow-save-real.spec.ts` (new)
- `web/app/(app)/workflows/[verticalId]/[name]/build/step-encoder.ts` (extend if needed)

## Must NOT touch

- The Publish button or `PUT /head` endpoint (Phase 2 scope)
- The conflict-banner UX or `if_version` 409 path (Phase 3 scope)
- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — endpoint already exists from #1846/#1860; this brief is wire-up only
- `crates/argo-store/src/workflow_definition.rs` (#1846 owns storage)
- `crates/argo-store/migrations/*.sql` (no migration)

## Sizing

300–400 LOC. Smaller-bite phase 1.

## Notes

- Pre-flight: `grep -n 'POST /workflows' crates/argo-cli/src/handlers/gateway/routes_workflows.rs` to confirm endpoint shape.
- Pre-flight: `cat web/app/(app)/workflows/[verticalId]/[name]/build/BuilderToolbar.tsx` (post-#1951) to see the current Save handler.
- Pre-condition: #1951 (builder-page-split) MERGED ✅ — BuilderToolbar is the natural mount point.
- This is the load-bearing "not theater" gate for the wave-15 buyer-trial save flow.
