# Wave-14 audit — Save + Publish + Versions wiring (workflow builder)

**Date**: 2026-05-21
**Surface**: `/workflows/<vertical>/<name>/build` (visual builder)
**Audit scope**: backend gateway → web SPA → operator UX for save, publish, version increment, audit emission, concurrent-edit handling.

## 1. Snapshot — what fires today

The "Save + publish" button in
`web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:708-714`
is a neutral toast. It calls **no API**, increments **no version**, emits
**no audit row**, and the canvas state is **discarded on page reload**.

```ts
function handleSaveAndPublish() {
  // Honest demo stub — wave-6 audit. Wave-13 cleaned up the leaked
  // version identifiers; copy is now neutral.
  toast.message("Saved as draft — preview build", {
    description: "Tenant-wide publish lands in an upcoming release.",
  });
}
```

The trigger-config drawer's "Save" button likewise only mutates local
React state (`triggerConfigs` map keyed by step id, dropped on reload).

## 2. Backend path — already shipped

The Rust gateway already exposes the full CRUD surface (PR #1860 merged):

| Verb   | Path                                                           | Audit kind                  |
|--------|----------------------------------------------------------------|-----------------------------|
| GET    | `/api/v1/verticals/{v}/workflows`                              | n/a (read)                  |
| GET    | `/api/v1/verticals/{v}/workflows/{n}`                          | n/a (read)                  |
| GET    | `/api/v1/verticals/{v}/workflows/{n}/versions`                 | n/a (read)                  |
| GET    | `/api/v1/verticals/{v}/workflows/{n}/versions/{id}`            | n/a (read)                  |
| POST   | `/api/v1/verticals/{v}/workflows/{n}` (body = TOML)            | `WorkflowDefinitionCreated` |
| PUT    | `/api/v1/verticals/{v}/workflows/{n}/head` (`{version_id}`)    | `WorkflowHeadChanged`       |
| DELETE | `/api/v1/verticals/{v}/workflows/{n}/versions/{id}`            | `WorkflowVersionArchived`   |

Handlers live at
`crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs`.
Mounted in `routes_dispatch_get.rs:34` + `routes_dispatch_post.rs:149`.

**Audit emission is real** — every mutating handler calls
`AuditSink::new_with_tenant(...).write(...)` with payload
`{ actor, tenant_id, vertical_id, name, version_id }`. The three audit
kinds round-trip in `argo-audit::event_tests::workflow_definition_kinds_round_trip_and_listed`.

**Capabilities** — `workflow_edit_capability()` (owner+admin) and
`workflow_read_capability()` (all roles incl. guest) defined at
`crates/argo-capability/src/lib.rs:70`. `WorkflowEdit` action is enforced
via `role_gate::enforce_role` in `handle_mutate` (`routes_workflows.rs:121`)
**before** the DB call — 403 does not leak existence.

## 3. Database schema — version semantics

Migration `v72.rs` (slot 118, already-shipped) creates two tables:

```sql
CREATE TABLE workflow_definition (
    id              TEXT PRIMARY KEY,    -- UUID v7
    tenant_id       TEXT NOT NULL,
    vertical_id     TEXT NOT NULL,
    name            TEXT NOT NULL,
    definition_toml TEXT NOT NULL,
    version         INTEGER NOT NULL,    -- monotonic per (tenant, vertical, name)
    parent_id       TEXT,                -- chain to previous version
    lifecycle_state TEXT NOT NULL DEFAULT 'draft',
    created_at      TEXT NOT NULL,
    modified_by     TEXT NOT NULL,
    UNIQUE (tenant_id, vertical_id, name, version)
);

CREATE TABLE workflow_definition_index (
    tenant_id, vertical_id, name TEXT NOT NULL,
    head_id     TEXT NOT NULL,
    PRIMARY KEY (tenant_id, vertical_id, name)
);
```

**Version increment**: automatic. `create()` at
`workflow_definition_write.rs:48` runs
`SELECT COALESCE(MAX(version), 0) + 1` per `(tenant, vertical, name)`.
The operator cannot name versions.

**Lifecycle**: rows land as `draft`. `set_head()` atomically (single
transaction) (a) upserts the index row pointing at the new id and (b)
flips that row's `lifecycle_state` to `active`. Note the prior head
row stays `active` in `lifecycle_state` after promotion — there is no
demote-to-draft sweep on the old head. Operationally harmless (index
table is the source of truth for "which is live") but slightly leaky.

**Archive**: flips state to `archived` AND clears the index row if the
archived id was head — downstream loaders won't pick it.

## 4. Web surface gap — NO save/publish wiring exists

- **No Next.js API route**: `web/app/api/workflows/` contains only
  `propose/route.ts` (AI propose endpoint). No
  `save/`, `publish/`, `versions/`, or `[verticalId]/[name]/route.ts`.
- **No client hook**: `web/lib/hooks/use-workflows.ts` does not exist.
  Grep of `web/` for `useWorkflows`, `useWorkflow`, `version_id`, `head_id`,
  `If-Match`, `etag` returns **zero matches**.
- **No `lib/api/workflows.ts`**: grep returns only `lib/workflow-templates.ts`
  (a static seed file).
- The viewer at
  `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` and the
  catalog at `web/app/(app)/workflows/page.tsx` both render from
  `SEED_WORKFLOWS` static maps. Comments at lines 31 and 17 explicitly
  defer the swap to a follow-up brief.

## 5. In-flight briefs touching this area

| Brief                                          | Status     | Covers                                |
|------------------------------------------------|------------|---------------------------------------|
| `mission-pm-workflow-persistence-schema`       | MERGED #1846 | v72 migration + store crate           |
| `mission-pm-workflow-edit-api`                 | MERGED #1860 | Gateway routes + audit + capabilities |
| `web-workflows-live-data-binding`              | AUTHORED, unclaimed | Read-only viewer + catalog swap |
| `web-workflow-builder-ai-prompt-wired`         | AUTHORED, unclaimed | AI propose flow + approve POST    |
| `web-workflow-rollback-feature`                | AUTHORED   | Rollback (set_head to older version)  |
| `web-workflow-trigger-config-ux`               | LANDED     | Trigger drawer UI (still client-only) |

**No brief exists that wires the "Save + publish" button itself.**
`web-workflow-builder-ai-prompt-wired` mentions an Approve→POST step
but only for AI-proposed diffs, not manual canvas edits.

## 6. The 5 dimensions

### 6.1 Save UX

**Today**: no save at all. Local React state only — page reload loses
everything (canvas position, selected node, palette open state, trigger
configs, AI-prompt drafts).

**Schema fit**: the `lifecycle_state = 'draft'` row supports the obvious
"save without publishing" pattern. `set_head` is the explicit
"publish this draft as the live version".

**Recommendation**: explicit "Save draft" button distinct from
"Save + publish". Autosave should NOT write a new DB row per
keystroke — the version chain is the audit trail, polluting it with
500 micro-versions per editing session destroys diff/rollback value.
Autosave belongs in browser-local storage (IndexedDB) until the
operator clicks "Save draft", which writes ONE new `workflow_definition`
row with `lifecycle_state = 'draft'`.

### 6.2 Publish UX

**Today**: single button labeled "Save + publish" conflates the two.

**Schema fit**: 2-step is natural — POST creates the draft, PUT
`/head` promotes. Operator should see "this will be the live version
for the entire tenant" + a confirm step (especially given the audit
row carries `actor` for compliance traceability).

**No staging/prod split exists**. The schema has no `environment` /
`stage` column. Multi-environment publish is a real gap if SKYCOT
wants to sell "promote from staging to prod" — but per CLAUDE.md
v1 strategy ("ship the 80% solution"), explicit publish-confirm is
sufficient for v1.

### 6.3 Version increment

**Automatic + monotonic**. `MAX(version) + 1` per triple. Operator
cannot name versions. `version` is the i64 visible in `GET /versions`;
`parent_id` chains back. The UUID-v7 `id` is the stable handle for
PUT/DELETE.

**Gap**: the web display today shows `workflow.version = "v3"` as a
static string. A real version display needs `v{version}` from the DB
row + a "by {modified_by} at {created_at}" attribution.

### 6.4 Audit row

**Fires on every mutation, correct payload, correct kind**. Verified
in `routes_workflows_handlers.rs:259`. AuditSink writes are async +
warn-logged on failure — they do NOT roll back the workflow write.
This is the right trade-off for v1 (audit is a side-channel, not a
2-phase commit), but worth noting: a write that succeeds + an audit
row that fails leaves you with a workflow change that is not auditable.

**Gap**: no audit row fires for "saved as draft" today because no
save API exists at the web layer. Once the save flow lands, the
existing `WorkflowDefinitionCreated` kind covers draft creation, and
`WorkflowHeadChanged` covers publish. **No new audit kind needed.**

### 6.5 Concurrent-edit handling

**Zero protection today** at any layer. None of the following exist:

- No `If-Match` / `ETag` headers on the gateway.
- No `version` or `head_id` parameter on POST (the `parent_id`
  defaults to whatever `get_head` returns at the moment of the
  POST, which races with another operator's POST).
- No row-level locking — both POSTs would succeed with different
  version numbers (the UNIQUE constraint is on
  `(tenant, vertical, name, version)`, which `MAX+1` respects, but
  whoever PUTs `/head` last wins, silently overwriting the other's
  intent).
- No web-side optimistic concurrency check.

**Failure mode**: operator A and operator B both open the builder,
both edit, both click Save + publish. Both POSTs succeed (creating
v4 and v5 respectively). Whoever's PUT `/head` lands second is the
"winner" — but the loser has no signal that their work was
superseded, and no recovery UX (they'd have to manually diff v4 vs
v5 in the rollback surface to even see what happened).

**Recommendation**: client sends `parent_version_id` on POST. Gateway
rejects with 409 Conflict if the current head no longer matches.
Web surface shows "this workflow was edited by {modified_by} {x}
minutes ago — load latest or force-overwrite?" similar to
Notion / Linear / Figma.

## 7. Top 3 gaps

1. **No web→gateway POST/PUT bridge for save or publish**. The
   gateway is ready (#1860); the web layer's Save+publish button is
   a toast. A brief titled
   `web-workflow-builder-save-and-publish-wired` is missing from
   `docs/build-plan/streams/`. Authoring it is the single biggest
   unblock for the "operator edits and ships a workflow" demo.

2. **No concurrent-edit detection at any layer**. Gateway accepts a
   POST without a `parent_version_id` precondition; client tracks no
   ETag. Two operators in the same workflow at once is silent
   data-loss territory. Pre-launch this needs a `parent_version_id`
   precondition on POST + a 409-Conflict response, plus a web-side
   "stale draft" warning before publish.

3. **Save vs publish are conflated; "draft" is unreachable**. Schema
   supports `lifecycle_state = 'draft'`, store layer creates rows as
   draft by default, but there is no UX surface that lets an operator
   create a draft without immediately publishing it. The button
   labeled "Save + publish" needs to split into "Save draft" (POST,
   stays as draft) and "Publish this draft" (PUT /head). Bonus:
   surface a "drafts" tab in the versions list — today the rollback
   feature brief only mentions promoting older active versions.

## 8. Files referenced

- `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (the stub)
- `~/code/argo/web/app/api/workflows/propose/route.ts` (only existing workflow API route)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_workflows.rs` (dispatch + auth)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs` (handlers + audit)
- `~/code/argo/crates/argo-store/src/workflow_definition.rs` (types)
- `~/code/argo/crates/argo-store/src/workflow_definition_write.rs` (create/set_head/archive)
- `~/code/argo/crates/argo-store/src/migrations/v72.rs` (schema)
- `~/code/argo/crates/argo-audit/src/event.rs` (audit kinds 237, 243, 251)
- `~/code/argo/crates/argo-capability/src/lib.rs:70` (`workflow_edit_capability`)
- `~/code/argo/docs/build-plan/streams/web-workflows-live-data-binding.md` (unclaimed read-only brief)
- `~/code/argo/docs/build-plan/streams/web-workflow-builder-ai-prompt-wired.md` (AI-prompt brief, mentions Approve→POST)
