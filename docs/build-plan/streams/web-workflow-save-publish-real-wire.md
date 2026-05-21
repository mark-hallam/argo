# web-workflow-save-publish-real-wire — wire the workflow-builder Save+Publish buttons to the merged gateway endpoints

**Owner**: claude.
**Authored against**: origin/main `2fa4baf1` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-save-publish-real-wire`.
**Priority**: **P0 — wave-15 buyer-trial blocker #6.** Save+publish buttons currently fire `sonner` toasts but POST nothing. Backend exists (`POST /workflows/{name}` from #1846, `PUT /workflows/{name}/head` from #1860); the SPA just isn't wired to them.

## Why this brief exists

Read `docs/skycot/audit/save-publish/` + wave-15 final verdict §15. Audit summary:

> "Save + publish is a toast — button posts nothing, no Next.js API route exists, no concurrent-edit protection at any layer. Score: **1.0/10**. P0 BLOCKER."

The chain of failure:

1. Workflow builder has a "Save as draft" button and a "Publish" button.
2. Both buttons today call `toast.success("Saved as draft (preview build)")` or similar — no `fetch()`, no `POST`, no persistence.
3. The user thinks their work saved. They reload. It's gone.
4. The gateway endpoints exist (`POST /workflows/{name}` for create + `PUT /workflows/{name}/head` for publish) — landed via `mission-pm-workflow-edit-api` (#1860) and `mission-pm-workflow-persistence-schema` (#1846).
5. The SPA just never got wired to them.

This brief lands four things together:

1. **Real save** — Save-as-draft posts the current canvas state to `POST /api/v1/workflows/{vertical}/{name}` and shows a "Saved (v3)" toast on success.
2. **Real publish** — Publish posts to `PUT /api/v1/workflows/{vertical}/{name}/head` to flip the published version and emits an audit row.
3. **Optimistic concurrent-edit protection** — `if_version: <currently-loaded>` field in the POST body; gateway rejects with 409 + current version if the user's view is stale. SPA shows a "Someone else just edited — refresh?" banner.
4. **Real error surfacing** — network errors / 5xx surface as red toast with retry; 409 conflicts surface as the banner; 400 schema errors surface field-level.

## Approach (concrete)

### Phase 0 — Find the actual save+publish callsites

```bash
# Where the buttons are today.
rg -ln 'Save as draft|"Publish"|handleSaveDraft|handlePublish|onSave|onPublish' web/app/\(app\)/workflows/ 2>&1
# Expected: build/page.tsx, viewer/page.tsx, or a sibling component.

# Where the toast lies live.
rg -ln 'toast\.success.*[Ss]aved|toast\.success.*[Pp]ublish|preview build' web/app/\(app\)/workflows/ 2>&1
```

The wave-15 audit located these in `build/page.tsx` and the viewer pages. Verify before authoring.

### Phase 1 — API client

1. **NEW** `web/lib/api/workflows-save-publish.ts` (~150 LOC) — thin wrappers around the gateway endpoints:

   ```ts
   import { gatewayFetch } from './client'
   import type { WorkflowDefinition } from './workflows'

   export interface SaveDraftInput {
     vertical_id: string
     name: string
     definition_toml: string          // canonical TOML representation of the canvas
     if_version?: number              // optimistic-concurrency token
     edit_reason?: string             // optional operator note
   }

   export interface SaveDraftResult {
     workflow_id: string
     version: number
     audit_row_id: string
   }

   export interface SaveDraftConflict {
     kind: 'conflict'
     current_version: number
     current_head_definition_toml: string
   }

   export async function saveDraft(input: SaveDraftInput):
     Promise<SaveDraftResult | SaveDraftConflict> { /* POST /api/v1/workflows/<vid>/<name> */ }

   export async function publishHead(input: {
     vertical_id: string
     name: string
     version: number                  // version to promote to head
   }): Promise<{ workflow_id: string; head_version: number; audit_row_id: string }> {
     /* PUT /api/v1/workflows/<vid>/<name>/head */
   }
   ```

   **DEMO-PATH PRECONDITION**: must NOT have a sample-data fallback in `isStrictProduction()` mode (wave-15 acceptance). In prod-strict, the 404/5xx propagates as an error toast.

2. **NEW** `web/lib/hooks/use-workflow-save-publish.ts` (~120 LOC) — hooks returning `mutateAsync`-backed callbacks plus `isSaving` / `isPublishing` booleans + last-error state.

### Phase 2 — Canvas → TOML serializer

3. **MODIFY** (or **NEW** if not present) `web/app/(app)/workflows/[verticalId]/[name]/build/step-encoder.ts` (already exists from #1889 http+code-nodes brief) — extend if needed so EVERY node kind round-trips canvas-state ↔ TOML. The save flow takes the current canvas state, runs it through `encodeStepsToToml(steps)`, and POSTs that string.

4. **NEW** `web/app/(app)/workflows/[verticalId]/[name]/build/__tests__/step-encoder-roundtrip.test.ts` (~120 LOC) — round-trip test: encode a workflow with every node kind to TOML, decode back, assert structural equality. Catches encoder bugs that would corrupt saves.

### Phase 3 — UI wire-up

5. **MODIFY** `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — replace the `toast.success(...)` stubs with real `saveDraft(...)` / `publishHead(...)` calls:

   - **Save button**: disabled while `isSaving` is true. On click → `saveDraft({ ..., if_version: loadedVersion })`. On success → `toast.success(`Saved (v${result.version})`)` + update local `loadedVersion`. On `kind: 'conflict'` → render the conflict banner (see step 6). On error → red toast with retry.
   - **Publish button**: disabled while `isPublishing` or while there are unsaved changes (force a save first). On click → confirm dialog "Publish v${loadedVersion} to head?" → `publishHead(...)` → success toast + redirect to viewer.

6. **NEW** `web/components/skycot/workflows/ConcurrentEditBanner.tsx` (~80 LOC) — the conflict banner. Shows when `saveDraft` returns `kind: 'conflict'`. Two CTAs: "Refresh (loses my changes)" and "Show diff vs current head" (links to the future diff page).

### Phase 4 — Gateway audit + idempotency

7. **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — confirm the existing `POST /workflows/<vid>/<name>` handler accepts the optional `if_version` query param or body field. If not, extend it: when `if_version` is present and ≠ current head version, return `409 Conflict` with body `{ current_version, current_head_definition_toml }`. Mirror the existing `mission-pm-workflow-edit-api` (#1860) implementation — DO NOT regress that brief's contract.

8. **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs` — extend the existing tests with 3 new cases:
   - `save_draft_with_no_if_version_succeeds_and_bumps_version`.
   - `save_draft_with_matching_if_version_succeeds`.
   - `save_draft_with_stale_if_version_returns_409_with_current_head`.

### Phase 5 — E2E test

9. **NEW** `web/tests/e2e/workflow-save-publish-real.spec.ts` (~150 LOC) — Playwright test that:
   - Opens /workflows/<vid>/<name>/build.
   - Drags a node.
   - Clicks Save.
   - Asserts the toast says "Saved (v...)" with a real version number.
   - Reloads the page.
   - Asserts the dragged node is still there (proves the save persisted).
   - Clicks Publish.
   - Asserts the viewer page shows the published version.

   This test is the load-bearing "not theater" proof — if it passes, the gap is closed.

## Files you may touch (verified to exist on `2fa4baf1`)

- **NEW** `web/lib/api/workflows-save-publish.ts`.
- **NEW** `web/lib/hooks/use-workflow-save-publish.ts`.
- **NEW** `web/components/skycot/workflows/ConcurrentEditBanner.tsx`.
- **NEW** `web/app/(app)/workflows/[verticalId]/[name]/build/__tests__/step-encoder-roundtrip.test.ts`.
- **NEW** `web/tests/e2e/workflow-save-publish-real.spec.ts`.
- **MODIFY** `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — replace toast stubs with real API calls. Be aware: this file is already >1000 LOC and pending the cluster-splitter brief #1888. Do NOT add to its LOC; consider extracting the save/publish handlers into a sibling `save-publish-handlers.ts`.
- **MODIFY** `web/app/(app)/workflows/[verticalId]/[name]/build/step-encoder.ts` (extend if needed).
- **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — `if_version` handling. **Verify with the brief author**: it may already be in place from #1860.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs` — 3 new tests.

## Files you must NOT touch

- `crates/argo-store/src/workflow_definition.rs` — the storage layer is owned by #1846. This brief uses the existing CRUD; do not extend the table.
- Any other workflow-related crate or web page outside the build page + the new files listed.
- `crates/argo-store/migrations/*.sql` — no migration required.

## Pre-flight verification

```bash
# Confirm the backend endpoints exist.
grep -n 'POST /workflows\|PUT /workflows' crates/argo-cli/src/handlers/gateway/routes_workflows.rs

# Confirm the brief deps merged (#1846, #1860).
git log --oneline origin/main | grep -E 'mission-pm-workflow-(persistence-schema|edit-api)' | head -3

# Confirm the toast stub callsites the wave-15 audit found.
rg -ln '"Saved as draft (preview build)"\|"All 5 steps passed"\|"Live wire ships in PR #1882"' web/app/

# Confirm step-encoder.ts exists from #1889.
ls web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/step-encoder.ts

# Confirm Playwright is wired.
ls web/tests/e2e/
```

## Acceptance criteria

- [ ] Save button POSTs to the gateway and the response version number appears in the toast.
- [ ] After Save, page reload preserves all canvas changes (the proof-of-persistence test).
- [ ] Publish button promotes head version; viewer page shows the new head.
- [ ] Stale `if_version` returns 409 and the conflict banner renders.
- [ ] Network/5xx errors surface as red toasts with a retry path.
- [ ] No more `toast.success("Saved as draft (preview build)")` strings in `web/app/(app)/workflows/`.
- [ ] **E2E test passes** — `web/tests/e2e/workflow-save-publish-real.spec.ts` is the load-bearing "not theater" gate.
- [ ] `pnpm typecheck` + `pnpm lint` + `pnpm test --run` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `bash scripts/conductor-pre-merge-gate.sh` PASS.
- [ ] Every modified file < 500 LOC. (The save/publish handlers extracted to sibling file keep `build/page.tsx` from growing.)
- [ ] No `unwrap` / `expect` outside `#[cfg(test)]`.
- [ ] No new mock fallback in `isStrictProduction()` mode.

## Branch + PR

- Branch: `claude-web-workflow-save-publish-real-wire`
- PR title: `v1 — web/workflows — real save+publish wire to merged gateway endpoints (web-workflow-save-publish-real-wire)`

## Gotchas

- **Don't grow build/page.tsx**: it's already >1000 LOC and pending the cluster-splitter (#1888). Extract save/publish handlers into a sibling file (`save-publish-handlers.ts`) to add functionality without adding LOC. Per HARD RULE 6.
- **`if_version` semantics**: the gateway may already accept this from #1860 (check `routes_workflows.rs` for any existing handling). If yes, skip the gateway-side modification — your job is purely SPA wire-up + the 409 banner.
- **Optimistic concurrency edge case**: when 2 users open the same workflow, both have `if_version: 3`. User A saves → server now has v4. User B saves → server returns 409 with `current_version: 4`. SPA shows banner. If User B refreshes, their canvas is wiped (loses their changes). The banner's "Show diff" CTA exists so they can copy-paste their changes manually before refresh.
- **Cluster collision risk**: this brief touches `build/page.tsx` which is also touched by the 9-PR workflow-builder cluster. **Wait for the cluster to land** (per the operator-locked sequencing via #1888 splitter) before claiming this brief. If claimed during cluster-flux, the rebase pain will be significant. Suggest claim AFTER #1888 splitter merges.

## References

- Audit: `docs/skycot/audit/save-publish/` + wave-15 final verdict §15.
- Sibling code:
  - `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — the merged POST + PUT handlers.
  - `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — the toast-stub callsites.
  - `web/lib/api/notifications.ts` (from #1906) — the canonical "no mock in prod-strict" pattern.
- Pattern reference: `mission-pm-workflow-edit-api` (#1860) for the gateway's contract.
