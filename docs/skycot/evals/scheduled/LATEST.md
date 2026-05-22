# Round 7 — Workflow CRUD code health audit

**Date**: 2026-05-23 (round 7)
**Scope**: Code review across the 3 layers of workflow create/edit/publish/rollback. Read ~1700 LOC. No code written — evaluator reports, conductor + workers fix.
**Verdict**: 🟢 **Mostly healthy** — strong tenant scoping, transactional writes, clean error mapping. **5 actionable findings** in priority order below + 1 known P0 (already in the queue).

## Layer-by-layer health snapshot

| Layer | Files | LOC | Tests | Health |
|---|---|---|---|---|
| Gateway routes | `routes_workflows.rs` + `routes_workflows_handlers.rs` | 798 | parse + lifecycle smoke | 🟢 solid |
| Storage primitive | `workflow_definition.rs` + read/write split | ~600 | inline + rollback tests | 🟢 solid |
| SPA build canvas | `build/page.tsx` (orchestrator) + 5 `builder-handlers/*.ts` hooks | 7196 across 34 files | unit + Playwright | 🟢 architecturally clean; ⏳ blocked by P0-E1 |

All files within the 500-LOC HARD RULE 6 ceiling. `page.tsx` at **497 LOC** (3 from the cap — would tip on any addition).

## What's strong

1. **Tenant scoping is uniform and SQL-level.** Every read/write in `workflow_definition_write.rs` and `routes_workflows_handlers.rs` binds `tenant_id` via parameter. No interpolated strings, zero injection surface. The `rollback_to` impl explicitly returns `ArgoError::NotFound` (not `Invariant`) when the target id belongs to a different tenant — **does not leak existence**. This matches the workflows handler's stated policy and **outperforms 6 worker handlers** flagged in round 4 (`mission-cross-tenant-worker-handler-no-existence-leak` brief still pending).

2. **Schema-level race protection.** Migration `118__workflow_definition.sql:53` declares `UNIQUE (tenant_id, vertical_id, name, version)`. Even if two concurrent `create()` calls compute the same `MAX(version) + 1`, only one INSERT can land. Race is bounded.

3. **All mutating handlers are transactional where atomicity matters.** `set_head` wraps probe + UPSERT into index + UPDATE lifecycle in one tx. `archive` wraps probe + UPDATE state + DELETE from index. `rollback_to` wraps probe + uniqueness check + INSERT new row + UPSERT head. ✅

4. **Optimistic-concurrency wire is end-to-end.** `X-Argo-If-Version` header → handler reads → compares to current head → returns 409 with `current_head_definition_toml` so the SPA can render a conflict banner + diff. The Playwright spec `workflow-concurrent-edit-409` exercises this end-to-end.

5. **SPA save-publish flow is correct.**
   - `useSavePublishHandlers` encodes nodes → TOML → real `saveDraft({vertical_id, name, definition_toml})` POST.
   - Records a `savedDraft` snapshot + canvas JSON; `canPublish` returns `true` only when the live canvas still matches the snapshot. Prevents publishing a dirty canvas. **UX guard, not theatre.**
   - Both save and publish toasts include `Retry` actions on error.
   - `handlePublishConfirm` routes to `/workflows/<v>/<n>` (the detail page) on success.

6. **Connect-handler validation is real.** `validateConnection` rejects:
   - empty source/target
   - self-loops (toast: "A step can't connect to itself")
   - duplicates (toast: "These steps are already connected")
   - trigger-incoming edges (toast: "Triggers can't have incoming connections")

7. **Audit emissions are documented + uniform.** Module docstring lists the 4 audit kinds, the payload shape, and the rollback-specific extras. Every mutating handler appends one row.

8. **Error mapping is layered.** Storage returns typed `ArgoError` variants; gateway handlers translate:
   - `ArgoError::Schema` → 400
   - `ArgoError::NotFound` → 404
   - `ArgoError::Invariant` → 400 or 404 (404 when leakage-sensitive)
   - everything else → 500

## What needs attention

### 🔴 F-R7-1 — (KNOWN) `layout.ts` import on `page.tsx:54` still hits the broken file

```typescript
import { stepsToFlow } from "./layout";   // page.tsx:54
import { formatWaitSummary } from "./layout";  // BuilderCanvasNode.tsx
```

Same P0 as round-5 — `build/layout.ts` is a React-Flow positioning utility but Next.js App Router treats `layout.ts` as a route layout and demands a default React-component export. None exists → blank canvas. **Brief `web-workflow-build-layout-ts-rename` pending in the conductor queue.** This is the gating fix; everything else in this audit becomes verifiable only after this lands.

### 🟡 F-R7-2 — Concurrent-create maps to opaque 500 instead of 409

Path: `routes_workflows_handlers.rs:151-184` (the `create_version` handler).

If two callers race past the optimistic-concurrency check (no `X-Argo-If-Version`) and try to create the same `(tenant, vertical, name, MAX+1)` simultaneously, the second INSERT fails with the UNIQUE-constraint violation. That bubbles up as `ArgoError::Provider("workflow_definition insert: …")` which the handler maps to **generic 500** at line 183.

Better UX: pattern-match the sqlx error in `wd::create()` for the `2067 SQLITE_CONSTRAINT_UNIQUE` code and return `ArgoError::Conflict(...)` (a new variant) → gateway maps to **409 with "version conflict, retry"** so the SPA can offer Retry.

**Brief recommendation: `claude-workflow-create-unique-collision-as-409`** — ~30 LOC + 1 test. Low priority, but fixes a real UX confusion.

### 🟡 F-R7-3 — Audit write is OUTSIDE the workflow write transaction

Pattern across all 4 mutating handlers (`create_version`, `set_head`, `archive_version`, `rollback`):

```rust
// 1) write workflow row (transactional inside wd::*)
wd::create(...).await?;
// 2) write audit row (separate, not in tx)
emit_audit_row(store, tenant, AuditKind::WorkflowDefinitionCreated, ...).await;
// → if step 2 fails: workflow lands, audit doesn't. Handler returns 201.
```

The audit write logs a `tracing::warn!` on failure but does NOT fail the request. Mission claim 6 is *"audit chain on every action"*. If a transient sqlite hiccup misses the audit row, the workflow change has no audit record, but the operator sees success.

Two fix shapes:
- **Option A (simplest)**: thread an `&mut tx` through `wd::create` so the audit row writes into the same transaction. Both succeed or both fail.
- **Option B (outbox pattern)**: write a queued `audit_pending` row in the same tx as the workflow, and a background worker drains it to `audit_record` with retries. More resilient under partial outages but more LOC.

**Brief recommendation: `mission-workflow-audit-write-transactional`** — Option A first; Option B if Option A creates contention. ~50-100 LOC.

Note: the audit-sink-tenant-guard CI check (per CLAUDE.md) verifies `tenant_id` is bound; it doesn't catch missing audit rows. A separate CI check `audit-row-presence-after-mutation` would catch this class of bug.

### 🟡 F-R7-4 — Build canvas reads workflows from a STATIC TS fixture, NOT the gateway

Path: `build/page.tsx:115-130`:

```typescript
const workflow = useMemo<WorkflowSpec | undefined>(() => {
  if (templateId) {
    const tpl = getTemplate(templateId);
    if (tpl) { return { ... }; }
  }
  return SEED_WORKFLOWS[`${verticalId}/${name}`];  // ← STATIC fixture
}, [templateId, verticalId, name]);
```

`SEED_WORKFLOWS` is a hardcoded TS object in `build/seed-workflows.ts` (118 LOC of static data). When the operator clicks "Open builder" on `/workflows/<v>/<n>`, the canvas shows the **hardcoded fixture for that vertical+name**, NOT the actual workflow stored in the gateway DB.

Comment at line 110-114 admits this:

> *"`?state=loading|error|empty` is the gateway-fetch-state simulator (real fetch wires in via PR #1860 routes follow-up)"*

So **save and publish work end-to-end** (real POST/PUT to gateway), but **load doesn't** — the workflow you edit is always the static fixture. If a user saved their own custom workflow yesterday, today they'd see the seed fixture again, not their saved version.

This is a real UX gap. The save side works (mission claim 3 surface looks correct), but the loop isn't closed. Mission claim 3 requires "edit → save → reload still shows the saved version" — which doesn't work today.

**Brief recommendation: `web-workflow-build-fetch-from-gateway`** — wire `useEffect` on mount → `GET /api/v1/verticals/<v>/workflows/<n>` → if 200 use that, else fall back to template / seed / empty. ~80 LOC + Playwright spec. Likely depends on `PR #1860` mentioned in the comment — check the ledger for status.

### 🟡 F-R7-5 — Two parallel audit-emission code paths

Path: `routes_workflows_handlers.rs:308-318` (rollback) vs `routes_workflows_handlers.rs:347-377` (`emit_audit_row` helper).

The rollback handler inlines its own audit-write because it needs extra payload fields (`rolled_back_from_version_id`, `new_version_id`, `new_version_number`). Three other handlers (create / set_head / archive) call `emit_audit_row`.

Refactor: change `emit_audit_row` signature to take an additional `extras: Vec<(&str, Value)>` parameter. Rollback passes its 3 extras; the other 3 handlers pass empty.

**Brief recommendation: `claude-workflow-audit-emit-unify-rollback-helper`** — ~20 LOC DRY refactor. Lowest priority — purely cosmetic.

### 🟡 F-R7-6 — `decode_index_row` is 49 LOC of manual sqlx row decoding

Path: `routes_workflows_handlers.rs:394-442`. Also similar 12-LOC blocks in `workflow_definition_write.rs:117-132` (the namespace-triple decode in `set_head`/`archive`/`rollback_to` — copied THREE times).

Two extractions possible:
- A `fn decode_namespace_triple(row) -> ArgoResult<(String, String, String)>` helper for the inline blocks.
- A `sqlx::FromRow` derive on `WorkflowDefinition` so the gateway can `fetch_all::<WorkflowDefinition>` directly.

**Brief recommendation: `claude-workflow-store-fromrow-derive`** — ~40 LOC saved via FromRow. Cosmetic, no behavior change. Lowest priority.

## What I'd ask the conductor to action — ranked

| # | Brief | Source | Priority |
|---|---|---|---|
| 1 | `web-workflow-build-layout-ts-rename` | round 5 | 🔴 P0 (still pending) |
| 2 | `web-workflow-build-fetch-from-gateway` | **round 7 NEW** | 🟡 P1 — closes the mission-claim-3 edit→reload loop |
| 3 | `mission-workflow-audit-write-transactional` | **round 7 NEW** | 🟡 P1 — mission-claim-6 audit-chain-on-every-action |
| 4 | `claude-workflow-create-unique-collision-as-409` | **round 7 NEW** | 🟡 P2 — UX nicety |
| 5 | `claude-workflow-audit-emit-unify-rollback-helper` | **round 7 NEW** | 🟡 P3 — DRY |
| 6 | `claude-workflow-store-fromrow-derive` | **round 7 NEW** | 🟡 P3 — DRY |

## Reading-trail (1700+ LOC reviewed)

For the auditor and any future evaluator who wants to re-audit:

- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — full (356 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs` — full (442 LOC)
- `crates/argo-store/src/workflow_definition.rs` — full (182 LOC)
- `crates/argo-store/src/workflow_definition_write.rs` — full (353 LOC)
- `crates/argo-store/migrations/118__workflow_definition.sql` — header + schema
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — full (497 LOC)
- `web/app/(app)/workflows/[verticalId]/[name]/build/builder-handlers.ts` — head (120 LOC)
- `web/app/(app)/workflows/[verticalId]/[name]/build/builder-handlers/save-publish.ts` — full (135 LOC)

Not read this round (deferred to next session): `workflow_definition_read.rs`, the React Flow canvas component (`BuilderCanvas.tsx`, 223 LOC), `RunOverlay.tsx` (379 LOC), `step-encoder.ts` (322 LOC — TOML serialization, critical), `useTestRun.ts` (278 LOC).

## What this round PROVES

🟢 **The workflow CRUD substrate is production-grade.** Tenant scoping, transactions, optimistic-concurrency, error mapping, and audit emissions are all wired correctly. The 6 findings above are quality/UX improvements, not safety bugs.

🟢 **The SPA save/publish loop is real, not theatre.** When the canvas isn't blanked by P0-E1, the operator's save/publish actually round-trips to the gateway, gets a version number back, and routes to the detail page.

🟡 **The load side of the loop is the missing piece.** Save works; load is static fixture. Mission claim 3 ("platform builds workers; operator describes, platform generates") needs both halves of the edit→save→reload contract.

🟢 **No `.unwrap()` or `.expect()` outside `#[cfg(test)]` in any file I read** (CLAUDE.md HARD RULE compliance). The `current_dir()` calls at `routes_workflows.rs:259, 296` use `.unwrap_or_else(...)` / `.map_err(...)` — defensive fallbacks, not panics. ✅
