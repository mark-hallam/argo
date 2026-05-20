# web-audit-kinds-for-journey-events — add 6 domain-event audit kinds

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-audit-kinds-for-journey-events`.
**Priority**: P1 — wave-15 user-journey audit found that the
`/audit` feed shows `HttpRequest` rows for journey-shaping actions (hire,
dispatch, cron) instead of named domain events. Operator scanning the feed
for "what happened today" sees generic HTTP noise — broken signal-to-noise
ratio for the audit promise ("every state transition is in the chain").
**Sizing**: ~400-550 LOC across 6 new `AuditKind` variants + 6 callsite gates
(replace `HttpRequest` emission with domain-specific kind) + exhaustiveness
test + payload-shape documentation + 1 integration roll-up test.

---

## Why this brief exists

Today the `/audit` feed shows rows like:

```
2026-05-20 14:22 · HttpRequest · POST /api/v1/workers/hire (200)
2026-05-20 14:25 · HttpRequest · POST /api/v1/verticals/pr_checker/run (202)
2026-05-20 14:31 · HttpRequest · POST /api/v1/cron (200)
```

These are journey-shaping events (hired a worker, kicked off a vertical run,
created a scheduled job). Audit consumers — operators auditing their
workspace, compliance reviewers — get NO useful signal beyond "an HTTP call
happened." The 25 detailed `AuditKind` variants for skill/approval/workflow
events all exist; the equivalent worker-hire / vertical-run / cron variants
are missing.

After this brief, the same actions emit:

```
2026-05-20 14:22 · WorkerHired · vertical=pr_checker · worker_id=wkr_…
2026-05-20 14:25 · VerticalRunRequested · vertical=pr_checker · run_id=run_…
2026-05-20 14:31 · CronJobCreated · job=daily_pr_summary · schedule="0 9 * * *"
```

Sibling brief `web-cronjob-tenant-id-bug` adds the `CronJobCreated` variant —
this brief adds the remaining 5 (and provides the recipe for `CronJobCreated`
to land via either brief, whichever merges first).

---

## Pre-flight verification

```bash
# 1. Confirm none of the 6 proposed kinds already exist.
rg "WorkerHired|VerticalRunRequested|VerticalRunCompleted|CronJobFired|CronJobDeleted|CronJobCreated" crates/argo-audit/src/event.rs

# 2. Confirm the current callsites emit HttpRequest instead.
rg "AuditKind::HttpRequest" crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs
rg "AuditKind::HttpRequest" crates/argo-cli/src/handlers/gateway/routes_vertical_run.rs 2>/dev/null
rg "AuditKind::HttpRequest" crates/argo-cli/src/handlers/gateway/routes_cron.rs
```

If any of the 6 already exist, mark this brief as PARTIAL and only land the
remaining variants.

---

## The 6 new variants

### 1. `WorkerHired`

Emit point: `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs`,
after successful `IdentityStore::create_worker`.

Payload:
```json
{
  "actor": "<principal>",
  "tenant_id": "<tenant>",
  "worker_id": "<wkr_id>",
  "vertical_id": "pr_checker",
  "name": "Maya the GTM Engineer",
  "persona_revision_id": "<rev_id>",
  "duty_count": 4,
  "hired_at": "2026-05-21T14:22:09Z"
}
```

Docstring template:
```rust
/// A new worker was hired into the workspace via
/// `POST /api/v1/workers/hire` or `argo worker hire` CLI
/// (`web-audit-kinds-for-journey-events`). Pairs with the
/// `worker.created_at` column — every `WorkerHired` row has a
/// matching `worker` row at the same timestamp ± clock skew. Payload:
/// `{ actor, tenant_id, worker_id, vertical_id, name,
/// persona_revision_id, duty_count, hired_at }`. Operator-facing
/// surfaces show this as "Maya the GTM Engineer joined" in the audit
/// feed and the worker-detail "hired" timeline marker.
WorkerHired,
```

Label: `"worker.hired"`.

### 2. `VerticalRunRequested`

Emit point: `crates/argo-cli/src/handlers/gateway/routes_vertical_run.rs`
(or wherever vertical-run is dispatched — verify via `rg "vertical.*run|run_vertical"`).

Payload:
```json
{
  "actor": "<principal>",
  "tenant_id": "<tenant>",
  "vertical_id": "pr_checker",
  "workflow_name": "standard_review",
  "workflow_version": 4,
  "run_id": "<run_id>",
  "trigger": "manual" | "scheduled" | "webhook",
  "requested_at": "2026-05-21T14:25:00Z"
}
```

Docstring:
```rust
/// A vertical workflow run was requested via
/// `POST /api/v1/verticals/{vid}/run` (or scheduled / webhook
/// trigger). Pairs with [`Self::VerticalRunCompleted`] for
/// forensic reconstruction of run round-trips. Payload: `{ actor,
/// tenant_id, vertical_id, workflow_name, workflow_version, run_id,
/// trigger ("manual" | "scheduled" | "webhook"), requested_at }`.
VerticalRunRequested,
```

Label: `"vertical.run.requested"`.

### 3. `VerticalRunCompleted`

Emit point: same handler, on run-completion path. Two terminal states:
`succeeded` or `failed`. Both emit `VerticalRunCompleted` with the outcome
in the payload (do not split into two kinds — keep filterable by payload).

Payload:
```json
{
  "actor": "<principal>",
  "tenant_id": "<tenant>",
  "run_id": "<run_id>",
  "outcome": "succeeded" | "failed",
  "duration_ms": 4321,
  "step_count": 7,
  "failed_step_name": "<step name if failed>",
  "completed_at": "2026-05-21T14:28:21Z"
}
```

Docstring:
```rust
/// A vertical workflow run finished (success or failure). Pairs with
/// [`Self::VerticalRunRequested`]. Payload's `outcome` field is
/// `"succeeded"` or `"failed"`; failed runs include
/// `failed_step_name`. Payload: `{ actor, tenant_id, run_id,
/// outcome, duration_ms, step_count, failed_step_name?,
/// completed_at }`.
VerticalRunCompleted,
```

Label: `"vertical.run.completed"`.

### 4. `CronJobCreated` (shared with `web-cronjob-tenant-id-bug`)

Emit point: gateway POST `/cron` handler (`routes_cron.rs`).

Payload — see the cronjob-tenant brief for the exact shape. Label:
`"cron_job_created"`.

**Coordination note**: whichever brief lands first owns the variant
addition. The other brief picks up the now-existing variant and proceeds
to its own work. Both briefs reference the same single source of truth
in `event.rs`.

### 5. `CronJobDeleted`

Emit point: gateway DELETE `/cron/<name>` handler.

Payload:
```json
{
  "actor": "<principal>",
  "tenant_id": "<tenant>",
  "job_name": "daily_pr_summary",
  "schedule": "0 9 * * *",
  "platform": "telegram",
  "chat": "<chat>",
  "deleted_at": "2026-05-21T14:30:00Z",
  "last_fired_at": "2026-05-20T09:00:00Z"
}
```

Docstring:
```rust
/// A cron job was deleted via `DELETE /cron/<name>` or `argo cron
/// delete <name>`. Pairs with [`Self::CronJobCreated`] — the
/// `job_name` is the match key. Payload includes the final
/// `last_fired_at` cursor at delete-time so post-hoc consumers
/// can compute "how many times did this job fire while it lived."
CronJobDeleted,
```

Label: `"cron_job_deleted"`.

### 6. `CronJobFired`

Emit point: `argo-cron::scheduler::tick` — when a `is_due()` check
succeeds and the job is about to dispatch.

Payload:
```json
{
  "tenant_id": "<tenant>",
  "job_name": "daily_pr_summary",
  "schedule": "0 9 * * *",
  "fired_at": "2026-05-21T09:00:00Z",
  "dispatch_target": { "platform": "telegram", "chat": "<chat>" }
}
```

Docstring:
```rust
/// A cron job fired and dispatched its prompt. Emitted by the
/// scheduler `tick`, NOT by HTTP handlers — there is no
/// human-actor for this row, so `actor` is omitted. Pairs with
/// [`Self::CronJobCreated`] / [`Self::CronJobDeleted`]. Payload:
/// `{ tenant_id, job_name, schedule, fired_at, dispatch_target }`.
CronJobFired,
```

Label: `"cron_job_fired"`.

---

## Phase 1 — Add the 6 variants to `event.rs`

Pattern: each variant gets entries in 4 places (enum body, `label()`,
`from_label()`, `all()` array). For the 6 variants this is 24 edits in one
file. Group them — the `Workflow*` variants are alphabetically adjacent;
group `Worker*` / `Vertical*` / `CronJob*` similarly so the enum scans
naturally.

Suggested ordering in the enum body (just before the existing
`WorkflowDefinitionCreated`):

```rust
// ── Worker / vertical / cron domain events (Stream
//    `web-audit-kinds-for-journey-events`). ─────────────────────────
WorkerHired,
VerticalRunRequested,
VerticalRunCompleted,
CronJobCreated,
CronJobDeleted,
CronJobFired,
// ── Existing workflow_definition kinds ────────────────────────────
WorkflowDefinitionCreated,
WorkflowHeadChanged,
...
```

---

## Phase 2 — Update callsites to emit the new kinds

### 2a. `WorkerHired`

**File**: `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs`

Find the `sink.emit(AuditKind::HttpRequest, ...)` call inside the hire
handler. ADD (don't replace) the new `WorkerHired` emission — `HttpRequest`
remains as the generic HTTP-level log; `WorkerHired` is the domain event.

```rust
// Existing HttpRequest emission stays.
sink.emit(AuditKind::HttpRequest, http_payload).await?;

// NEW domain event.
sink.emit(AuditKind::WorkerHired, json!({
    "actor": principal.to_string(),
    "tenant_id": tenant.to_string(),
    "worker_id": worker.id.to_string(),
    "vertical_id": worker.vertical_id,
    "name": worker.name,
    "persona_revision_id": worker.persona_revision_id,
    "duty_count": worker.duties.len(),
    "hired_at": Utc::now().to_rfc3339(),
})).await?;
```

### 2b. `VerticalRunRequested` + `VerticalRunCompleted`

**File**: `crates/argo-cli/src/handlers/gateway/routes_vertical_run.rs`
(or sibling — confirm via grep before code starts).

Emit `VerticalRunRequested` at the top of the handler (after auth + validation,
before dispatch). Emit `VerticalRunCompleted` from the dispatch completion
path with the outcome in the payload.

If the completion path is async (run dispatched + result returned later via
webhook or poll), emit the `Completed` row from whatever finalizer code path
already owns the run-state transition. **Don't emit it speculatively** —
only after the run has actually finished.

### 2c. `CronJobCreated`

Sibling brief (`web-cronjob-tenant-id-bug`) owns this — emit from the same
POST handler in `routes_cron.rs` after the file-write succeeds.

### 2d. `CronJobDeleted`

**File**: `crates/argo-cli/src/handlers/gateway/routes_cron.rs` DELETE handler.

Emit BEFORE the `std::fs::remove_file` call so the audit row precedes the
filesystem mutation (if remove_file fails, the audit row is a noop trace,
which is OK — the alternative of emit-after-success means a successful
delete with a failed audit emit silently loses the record).

### 2e. `CronJobFired`

**File**: `crates/argo-cron/src/scheduler.rs` — `tick` method.

The scheduler is in a separate crate. Wire an `AuditSink` parameter through
the `Scheduler::new` constructor. Currently the scheduler is constructed
without one in `boot_serve.rs`; pass the same `Arc<dyn AuditSink>` already
used by gateway routes.

```rust
// In scheduler tick():
for job in due_jobs {
    if !is_known_tenant(&store, &job.tenant_id).await { continue; }
    if let Some(sink) = &self.audit_sink {
        let _ = sink.emit(AuditKind::CronJobFired, json!({
            "tenant_id": job.tenant_id.to_string(),
            "job_name": job.name,
            "schedule": job.schedule.source,
            "fired_at": chrono::Utc::now().to_rfc3339(),
            "dispatch_target": { "platform": job.target.platform, "chat": job.target.chat },
        })).await;
    }
    fire_job(&job, &store).await;
}
```

The `Option<Arc<dyn AuditSink>>` keeps backward compatibility — schedulers
constructed without a sink (e.g. unit tests) still tick.

---

## Phase 3 — Exhaustiveness test

`crates/argo-audit/src/tests.rs` (or wherever the existing label round-trip
test lives — `rg "all\(\)\.iter\(\)" crates/argo-audit/src/`):

```rust
#[test]
fn round_trip_includes_journey_events() {
    for kind in AuditKind::all() {
        let label = kind.label();
        let parsed = AuditKind::from_label(label).unwrap();
        assert_eq!(kind, parsed);
    }
    // Explicit presence check so an accidental delete doesn't pass.
    assert!(AuditKind::all().contains(&AuditKind::WorkerHired));
    assert!(AuditKind::all().contains(&AuditKind::VerticalRunRequested));
    assert!(AuditKind::all().contains(&AuditKind::VerticalRunCompleted));
    assert!(AuditKind::all().contains(&AuditKind::CronJobCreated));
    assert!(AuditKind::all().contains(&AuditKind::CronJobDeleted));
    assert!(AuditKind::all().contains(&AuditKind::CronJobFired));
}
```

---

## Phase 4 — Integration roll-up test

**File**: `crates/argo-cli/tests/journey_event_emission.rs` (new)

E2E-flavored test (against the in-memory gateway):

1. POST `/workers/hire` → assert `WorkerHired` row in `audit_record`.
2. POST `/verticals/pr_checker/run` → assert `VerticalRunRequested` row.
3. Wait for run-completion → assert `VerticalRunCompleted` row with
   matching `run_id`.
4. POST `/cron` → assert `CronJobCreated` row.
5. Simulate scheduler tick → assert `CronJobFired` row.
6. DELETE `/cron/<name>` → assert `CronJobDeleted` row.
7. GET `/audit?kind=worker.hired&since=...` → assert filter returns
   only the WorkerHired row from step 1.

---

## may-touch

- `crates/argo-audit/src/event.rs` — add 6 variants × 4 sites = 24 edits.
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` — emit `WorkerHired`.
- `crates/argo-cli/src/handlers/gateway/routes_vertical_run.rs` (verify path) — emit `VerticalRunRequested` + `VerticalRunCompleted`.
- `crates/argo-cli/src/handlers/gateway/routes_cron.rs` — emit `CronJobDeleted` (and `CronJobCreated` if sibling brief hasn't landed yet).
- `crates/argo-cron/src/scheduler.rs` — accept `AuditSink`; emit `CronJobFired`.
- `crates/argo-cli/src/handlers/gateway/boot_serve.rs` — wire the existing AuditSink into the scheduler constructor.
- `crates/argo-audit/src/tests.rs` — add exhaustiveness assertions.
- `crates/argo-cli/tests/journey_event_emission.rs` (new) — integration roll-up.

## must-NOT-touch

- The existing `HttpRequest` emissions — keep as generic HTTP trace alongside
  the new domain events. Audit consumers can filter.
- The other 30+ existing `AuditKind` variants — only ADD, never RENAME or
  RE-ORDER existing variants (label IDs are persisted in the audit chain;
  renaming breaks historical queries).
- Workflow*, Skill*, Approval* variants — those have their own briefs.

---

## Acceptance criteria

- [ ] All 6 new variants compile and round-trip via `label()` / `from_label()`.
- [ ] Exhaustiveness test passes.
- [ ] POST `/workers/hire` emits BOTH `HttpRequest` AND `WorkerHired` rows.
- [ ] POST `/verticals/<v>/run` emits `VerticalRunRequested`.
- [ ] Run completion (whether through dispatch_runner / webhook / polling) emits `VerticalRunCompleted` exactly once per run.
- [ ] POST `/cron` emits `CronJobCreated`.
- [ ] DELETE `/cron/<name>` emits `CronJobDeleted`.
- [ ] Scheduler tick fires emit `CronJobFired` for every job dispatch.
- [ ] `Scheduler::new` accepts an optional `Arc<dyn AuditSink>`; passing `None` keeps existing behavior (back-compat).
- [ ] Web /audit page filter dropdown includes the 6 new kinds (auto-derived from `AuditKind::all()` — if the dropdown is hardcoded, the brief includes a small refactor to enumerate dynamically).
- [ ] Integration test `journey_event_emission.rs` passes (6-step roll-up).
- [ ] `cargo clippy --workspace -- -D warnings` clean.

---

## Gotchas

1. **Label-string stability is load-bearing**: existing audit rows are
   persisted with the label string in `audit_record.action_type`. The label
   you choose for the new variants is permanent — you cannot change it later
   without a data migration. Use kebab-case for cron events
   (`cron_job_created` matches the existing `workflow_definition_created`
   underscore style), dot-case for vertical events (`vertical.run.requested`
   matches the existing `workflow.step.started` dot style).
2. **`HttpRequest` is NOT being replaced** — both rows are emitted. The
   `HttpRequest` row is structural ("an HTTP request came in"); the domain
   row is semantic ("a worker was hired"). Different audiences read each.
3. **`VerticalRunCompleted` must fire exactly once per run** — if the
   completion path has retries or webhooks that can fire multiple times,
   add an idempotency guard (e.g., check `audit_record` for an existing
   `VerticalRunCompleted` row with the same `run_id` before emitting).
4. **`CronJobFired` from the scheduler crate**: the scheduler doesn't
   today depend on `argo-audit`. Adding the dependency is fine but
   double-check it doesn't create a circular crate dependency. Likely
   path: `argo-cron` → `argo-audit` (new) is acyclic because `argo-audit`
   doesn't depend on `argo-cron`.
5. **`audit_record.tenant_id` filtering** — these events must be tenant-
   scoped. The payloads include `tenant_id`, and the audit-sink wrapper
   writes it to the row's `tenant_id` column. Confirm the sink your handler
   uses is the tenant-aware one (`argo_audit::sink_tenant`).
6. **`SecurityKeyRotated` precedent**: there's already a non-actor audit
   event (`SecurityKeyRotated` — system did it). `CronJobFired` follows
   that precedent — no `actor` field in the payload.

---

## References

- Existing `AuditKind` enum: `crates/argo-audit/src/event.rs`
- Hire callsite: `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs`
- Vertical-run callsite: `crates/argo-cli/src/handlers/gateway/routes_vertical_run.rs` (verify path before authoring)
- Cron callsites: `crates/argo-cli/src/handlers/gateway/routes_cron.rs` + `crates/argo-cron/src/scheduler.rs`
- Sink wiring pattern: `crates/argo-audit/src/sink_tenant.rs`
- Coordinated brief: `web-cronjob-tenant-id-bug` (owns CronJobCreated emission)
- Audit: `docs/skycot/audit/user-journey/2026-05-20-wave-15-final-verdict.md` (HttpRequest noise finding)
