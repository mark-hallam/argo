# web-workflow-publish-button-real-wire

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-publish-button-real-wire`.
**Priority**: **P1 — wave-15 buyer-trial blocker #6 (phase 2 of 3).**

## Summary

Phase 2 of 3 split from `web-workflow-save-publish-real-wire` (superseded). Wire the **Publish** button to `PUT /api/v1/workflows/<vid>/<name>/head` so the user can promote a saved draft to head. Confirm dialog before publish; audit row emitted; viewer page redirected.

Pre-condition: **`web-workflow-save-button-real-wire` MERGED** (reuses the API client + hook + Builder toolbar plumbing).

## Acceptance

- [ ] `publishHead({vertical_id, name, version})` exported from `web/lib/api/workflows-save-publish.ts` (extend the file from phase 1).
- [ ] `use-workflow-save-publish` hook gains `publishHead` callback + `isPublishing` + `lastError`.
- [ ] Publish button on the builder (BuilderToolbar post-#1951): disabled while `isPublishing` OR while there are unsaved changes (must Save first).
- [ ] Click → confirm dialog "Publish v${loadedVersion} to head?" (canonical AlertDialog component).
- [ ] On confirm → `publishHead(...)` → toast "Published v${head_version} to head" → redirect to viewer page `/workflows/<vid>/<name>` (or whatever the canonical viewer route is).
- [ ] Network/5xx errors surface as red toast with retry.
- [ ] Audit row emitted by gateway (already does this per #1860; verify in test).
- [ ] Playwright E2E: save a draft → click Publish → confirm → assert viewer page shows new head version. New spec `web/tests/e2e/workflow-publish-real.spec.ts`.
- [ ] No regression on phase 1's Save flow.
- [ ] `pnpm typecheck` + `pnpm lint` + `pnpm test --run` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.

## May touch

- `web/lib/api/workflows-save-publish.ts` (extend)
- `web/lib/hooks/use-workflow-save-publish.ts` (extend)
- `web/app/(app)/workflows/[verticalId]/[name]/build/BuilderToolbar.tsx` (Publish handler)
- `web/components/skycot/workflows/PublishConfirmDialog.tsx` (new, optional — could be inline AlertDialog)
- `web/tests/e2e/workflow-publish-real.spec.ts` (new)

## Must NOT touch

- The Save endpoint (Phase 1 settled)
- The conflict-banner UX or `if_version` 409 path (Phase 3 scope)
- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — `PUT /head` already exists per #1860
- `crates/argo-store/src/workflow_definition.rs`

## Sizing

200–300 LOC.

## Notes

- Pre-flight: `grep -n 'PUT /workflows' crates/argo-cli/src/handlers/gateway/routes_workflows.rs` to confirm endpoint exists and its contract.
- Pre-flight: read the AlertDialog component sibling (look at how #1949 or other recent PRs use the confirm-dialog pattern).
- This brief is the user-visible payoff of the save+publish flow — buyer can take a workflow from draft to live in two clicks.
