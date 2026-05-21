# web-workflow-concurrent-edit-conflict-banner

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-concurrent-edit-conflict-banner`.
**Priority**: **P2 — wave-15 buyer-trial blocker #6 (phase 3 of 3).**

## Summary

Phase 3 of 3 split from `web-workflow-save-publish-real-wire` (superseded). Optimistic concurrency: the SPA's `saveDraft` includes `if_version: <currently-loaded>`. The gateway rejects with 409 + current version + current head TOML when the user's view is stale. SPA renders a "Someone else just edited — refresh?" banner.

Pre-condition: **`web-workflow-save-button-real-wire` MERGED** (reuses the API client + hook).

## Acceptance

- [ ] `saveDraft(...)` in `web/lib/api/workflows-save-publish.ts` accepts optional `if_version: number` and returns either `SaveDraftResult` (success) or `SaveDraftConflict` (with `current_version` + `current_head_definition_toml`).
- [ ] Gateway side (`crates/argo-cli/src/handlers/gateway/routes_workflows.rs`): when `if_version` is present and stale, return `409 Conflict` body `{ kind: "conflict", current_version, current_head_definition_toml }`. If `if_version` matches, proceed and bump version. Verify with #1860's contract first — may already be partially implemented.
- [ ] Gateway tests (extend `routes_workflows_tests.rs`):
  - `save_draft_with_no_if_version_succeeds_and_bumps_version`
  - `save_draft_with_matching_if_version_succeeds`
  - `save_draft_with_stale_if_version_returns_409_with_current_head`
- [ ] `web/components/skycot/workflows/ConcurrentEditBanner.tsx` (new, ~80 LOC) — renders when `saveDraft` returns `kind: 'conflict'`. Two CTAs: "Refresh (loses my changes)" and "Show diff vs current head" (links to future diff page; for v1 just opens current head in a new tab).
- [ ] BuilderToolbar wires the banner: on conflict response, renders the banner + retains the user's dirty state (don't auto-discard their changes).
- [ ] Playwright E2E: User A saves → loadedVersion=2. User B (separate session, same workflow) saves → 409 → banner appears.
- [ ] No regression on phase 1's Save happy path.
- [ ] `pnpm typecheck` + `pnpm lint` + `pnpm test --run` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.

## May touch

- `web/lib/api/workflows-save-publish.ts` (extend with `if_version` + `SaveDraftConflict` union type)
- `web/lib/hooks/use-workflow-save-publish.ts` (surface the conflict)
- `web/app/(app)/workflows/[verticalId]/[name]/build/BuilderToolbar.tsx` (handle conflict response)
- `web/components/skycot/workflows/ConcurrentEditBanner.tsx` (new)
- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` (if_version handling — verify if already in place)
- `crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs` (3 new tests)
- `web/tests/e2e/workflow-concurrent-edit.spec.ts` (new)

## Must NOT touch

- The Save happy-path or Publish flow (phases 1 + 2 settled)
- `crates/argo-store/src/workflow_definition.rs` (#1846 owns storage)
- `crates/argo-store/migrations/*.sql` (no migration; version column already exists per #1846)

## Sizing

200–300 LOC.

## Notes

- Pre-flight: `grep -n 'if_version\|409' crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — if #1860 already wired this, the scope shrinks to UI-only.
- The "Show diff" CTA can be a v1 stub (link to viewer of current head); a real diff UI is a v1.x follow-up.
- This brief is the procurement-grade leverage — concurrent-edit safety is what enterprise buyers ask about.
