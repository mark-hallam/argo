# web-cronjob-tenant-id-bug — fix orphan-tenant cron jobs

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-cronjob-tenant-id-bug`.
**Priority**: 🔴 **P0 — security / multi-tenant isolation bug.** Two gateway
callsites + one CLI callsite construct `CronJob` with `TenantId::new()` — a
**fresh anonymous tenant id** — instead of the caller's actual tenant. Every
cron job created via `POST /cron` or via the CLI file path is orphaned to a
tenant no operator owns, and the scheduler walks ALL jobs across all tenants.
This is the same class of bug as the `McpConnectionPool` cross-tenant leak
(#mission-mcp-pool-per-tenant) but in the cron subsystem.
**Sizing**: ~400-500 LOC across 2 gateway callsite fixes + 1 CLI callsite fix
+ resolve-tenant helper + 4 unit tests + 2 integration tests (one cross-tenant
negative, one fire-time tenant-match positive) + 1 audit-row addition.

---

## Why this brief exists

The bug, in 3 lines of `git grep`:

```
crates/argo-cli/src/handlers/gateway/routes_cron.rs:~ CronJob::new(TenantId::new(), name.clone(), ...)
crates/argo-cli/src/handlers/cron_files.rs:~          CronJob::new(TenantId::new(), name, ...)
crates/argo-cli/tests/system_jobs_smoke.rs            CronJob::new(TenantId::new(), ...)  // ← test fixture, acceptable
```

`TenantId::new()` produces a **brand-new UUIDv7 tenant ID that no one owns**.
Every job created through these paths is parented to a phantom tenant.
Downstream consequences observed in wave-14 user-journey audit:

1. **Operator can't see their own cron job in `/cron` tenant-scoped listings**
   (because their tenant_id doesn't match the job's phantom tenant).
2. **Scheduler still fires the job** (it walks `list_all()` across tenants),
   so the operator IS getting hit by the prompt running — they just can't
   manage the job.
3. **Audit trail emits with phantom tenant**, breaking
   per-tenant audit isolation guarantees.
4. **Worker dispatch under the phantom tenant** runs the agent with the
   phantom tenant's permissions — typically zero capabilities — so the agent
   silently fails to access skills, MCP connectors, anything tenant-scoped.

The one correct callsite is `worker_create.rs` which threads the real
`tenant_id` from the duty record. The pattern exists; the two broken
callsites just forgot to wire it.

---

## Pre-flight verification

```bash
# 1. Confirm the bug is still present (these 3 lines must match).
rg "CronJob::new\(\s*TenantId::new\(\)" crates/

# 2. Confirm the correct counter-example.
rg "CronJob::new\(\s*tenant_id" crates/argo-cli/src/handlers/worker_create.rs

# 3. Confirm the tenant-resolution helper exists or identify the bootstrap path.
rg "resolve_workspace_tenant|resolve_tenant" crates/argo-cli/src/handlers/gateway/ -l

# 4. Confirm the JobStore::save signature passes the full CronJob (with tenant_id).
rg "pub async fn save" crates/argo-cron/src/job.rs -A 2
```

---

## Root cause

The route handler `routes_cron.rs` POST is invoked WITHOUT a tenant context.
Looking at the surrounding code:

```rust
// In routes_cron.rs POST handler:
let cron = match CronExpr::parse(&schedule) { ... };
let path = dir.join(format!("{name}.json"));
if path.exists() { ... 409 ... }
std::fs::create_dir_all(&dir);
let job = CronJob::new(
    TenantId::new(),                    // ← BUG: anonymous fresh tenant
    name.clone(), cron, prompt, CronTarget { platform, chat },
);
```

The handler has access to the workspace via `cron_dir()` (filesystem path),
but never asks "whose workspace is this?" The workspace IS tenant-scoped in
v1 (single-tenant workspace pattern), so the fix is to look up the active
workspace's tenant exactly the way `routes_worker_trust_grade.rs` does:

```rust
async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let workspace = store.ensure_workspace().await?;
    Ok(workspace.tenant_id)
}
```

This pattern is canonical — 7+ gateway handlers already use it.

---

## Phase 1 — Fix `routes_cron.rs` POST handler

**File**: `crates/argo-cli/src/handlers/gateway/routes_cron.rs`

### 1a. Thread `SessionStore` into the POST handler

Look up the dispatcher that calls the POST handler. The POST handler today
takes `(body: &str)` only. It needs `(body: &str, store: &SessionStore)`.

```rust
pub(super) async fn cron_create_response(
    body: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return (
            "500 Internal Server Error",
            format!("{{\"error\":\"resolve tenant: {e}\"}}"),
        ),
    };
    // ... existing parse + dir-create logic ...
    let job = CronJob::new(
        tenant,                          // ← FIX: real workspace tenant
        name.clone(), cron, prompt, CronTarget { platform, chat },
    );
    // ... existing file-write + audit-row ...
}
```

### 1b. Update sibling dispatch + tests

`routes.rs` (or `routes_dispatch_post.rs`) — find the existing call to
`cron_create_response(body)` and update to pass `store`.

### 1c. Emit audit row with the correct tenant

The existing POST flow already emits an audit row but with the phantom tenant.
After the `tenant` resolution above, that audit emission automatically uses
the right tenant (assuming it reads `job.tenant_id`).

Verify: search the POST handler for `sink.emit(...)` calls and ensure the
payload's `tenant_id` field reads from the resolved tenant, not a fresh one.

---

## Phase 2 — Fix `cron_files.rs`

**File**: `crates/argo-cli/src/handlers/cron_files.rs`

The CLI path `argo cron add ...` flows through this file. It's a slightly
different problem — the CLI runs LOCALLY against the workspace dir, so there's
no HTTP layer; the tenant must come from the workspace bootstrap.

```rust
pub(super) async fn create_job_file(
    dir: &std::path::Path,
    name: &str,
    schedule: &str,
    prompt: &str,
    platform: &str,
    chat: &str,
    store: &SessionStore,             // ← NEW param
) -> ArgoResult<()> {
    let cron = CronExpr::parse(schedule)
        .map_err(|e| ArgoError::Provider(format!("schedule '{schedule}': {e}")))?;
    let tenant = store.ensure_workspace().await?.tenant_id;     // ← NEW
    let job = CronJob::new(
        tenant,                                                  // ← FIX
        name,
        cron,
        prompt.to_string(),
        CronTarget { platform: platform.into(), chat: chat.into() },
    );
    // ... existing file-write ...
}
```

CLI callsites — `crates/argo-cli/src/cli_cron_cmds.rs` (or sibling) — must
pass the `store` they already construct.

---

## Phase 3 — Scheduler fire-time tenant safety check

Even with the constructor fixed, defensive depth: when the scheduler picks
a job and is about to invoke the agent, it should re-verify the tenant exists
in the identity store. A job whose `tenant_id` doesn't resolve to a known
workspace is a stale-row signal and should be **skipped + logged**, not fired.

**File**: `crates/argo-cron/src/scheduler.rs`

In the `tick` method, when iterating due jobs:

```rust
for job in due_jobs {
    if !is_known_tenant(&store, &job.tenant_id).await {
        tracing::warn!(
            target: "cron::scheduler",
            tenant_id = %job.tenant_id,
            job_name = %job.name,
            "skipping cron job with unknown tenant — possible orphan"
        );
        continue;
    }
    fire_job(&job, &store).await;
}
```

Helper:

```rust
async fn is_known_tenant(store: &SessionStore, tenant: &TenantId) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM workspace WHERE tenant_id = ?",
    )
    .bind(tenant.to_string())
    .fetch_one(store.pool())
    .await
    .map(|c| c > 0)
    .unwrap_or(false)
}
```

This catches future regressions and any existing-on-disk orphan jobs from
before this fix.

---

## Phase 4 — Migration / cleanup of existing orphan jobs

Existing workspaces may have orphan jobs on disk from the pre-fix builds. The
brief proposes a **defensive cleanup on next boot**:

**File**: `crates/argo-cron/src/scheduler.rs` (or a new `migrate.rs` sibling)

```rust
/// One-shot cleanup at scheduler-start. Walks `<workspace>/cron/jobs/`,
/// reads each job, and if the tenant_id doesn't match the workspace's
/// canonical tenant, REWRITES the job file with the canonical tenant.
/// Logs a `WARN` for each rewrite. Idempotent — running it twice is
/// a no-op the second time.
pub async fn rewrite_orphan_cron_jobs(
    dir: &std::path::Path,
    canonical_tenant: TenantId,
) -> ArgoResult<usize> {
    if !dir.exists() { return Ok(0); }
    let mut rewritten = 0;
    for entry in std::fs::read_dir(dir).map_err(|e| ArgoError::Io(format!("{e}")))? {
        let entry = entry.map_err(|e| ArgoError::Provider(format!("{e}")))?;
        let path = entry.path();
        if !path.extension().is_some_and(|x| x == "json") { continue; }
        let body = std::fs::read_to_string(&path).map_err(|e| ArgoError::Io(format!("{e}")))?;
        let mut job: CronJob = serde_json::from_str(&body)
            .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))?;
        if job.tenant_id == canonical_tenant { continue; }
        tracing::warn!(
            target: "cron::migrate",
            file = %path.display(),
            old_tenant = %job.tenant_id,
            new_tenant = %canonical_tenant,
            "rewriting orphan cron job to canonical tenant",
        );
        job.tenant_id = canonical_tenant.clone();
        let rewritten_body = serde_json::to_string(&job)
            .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?;
        std::fs::write(&path, rewritten_body)
            .map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))?;
        rewritten += 1;
    }
    Ok(rewritten)
}
```

Invoke once at scheduler boot, before the first tick.

**Caveat**: this is correct ONLY because v1 is single-tenant-per-workspace.
The day v1 supports multi-tenant workspaces, this migration becomes
incorrect — flag with a TODO comment + a CI lint that fails the cleanup if
a `multi_tenant` feature flag is enabled.

---

## Phase 5 — Audit emission

Add a new `AuditKind::CronJobCreated` row when the gateway POST handler
creates a job. Follows the same 4-site pattern as `WorkflowDefinitionCreated`
(see rollback brief for the recipe).

Variant docstring:

```rust
/// A cron job was created via `POST /cron` or `argo cron add`
/// (`web-cronjob-tenant-id-bug`). Payload carries `actor`,
/// `tenant_id`, `job_name`, `schedule`, `platform`, `chat`,
/// `prompt_excerpt` (first 200 chars). Pairs with future
/// `CronJobFired` / `CronJobDeleted` rows when those briefs land.
CronJobCreated,
```

Label: `"cron_job_created"`. (Sibling brief
`web-audit-kinds-for-journey-events` adds the matching `CronJobFired` +
`CronJobDeleted` variants — these three rolls of audit kinds form the cron
event-stream surface.)

---

## may-touch

- `crates/argo-cli/src/handlers/gateway/routes_cron.rs` — POST handler signature + tenant resolution + audit emission.
- `crates/argo-cli/src/handlers/cron_files.rs` — `create_job_file` signature + tenant resolution.
- `crates/argo-cli/src/handlers/gateway/routes.rs` (or `routes_dispatch_post.rs`) — wire `store` into the POST dispatch arm.
- `crates/argo-cli/src/cli_cron_cmds.rs` — CLI call sites pass `store`.
- `crates/argo-cron/src/scheduler.rs` — fire-time tenant safety check + orphan rewrite migration.
- `crates/argo-audit/src/event.rs` — add `CronJobCreated` variant (4 sites).
- `crates/argo-cli/tests/cron_tenant_isolation.rs` (new) — cross-tenant negative test.
- `crates/argo-cli/tests/cron_orphan_rewrite.rs` (new) — migration test.

## must-NOT-touch

- `argo-cron` `CronJob` struct or `CronJob::new` signature — the bug is at the
  CALL sites, not the constructor. The constructor already correctly takes
  `tenant_id`.
- `JobStore` trait — no schema change.
- The 3 `tenant_id` test-fixture callsites in `system_jobs_smoke.rs` — those
  are intentional test isolation with synthetic tenants.

---

## Acceptance criteria

- [ ] `rg "CronJob::new\(\s*TenantId::new\(\)" crates/` returns ZERO matches outside `/tests/` paths.
- [ ] POST `/cron` with no auth context returns 401 (existing behavior preserved).
- [ ] POST `/cron` from an authenticated session creates a job whose `tenant_id` matches `workspace.tenant_id`.
- [ ] `GET /cron` filtered to the caller's tenant returns the job they just created (no longer invisible).
- [ ] Scheduler tick skips + logs WARN for any job whose `tenant_id` is unknown to the identity store.
- [ ] `rewrite_orphan_cron_jobs` migrates existing on-disk orphans to the canonical workspace tenant on first boot; idempotent on subsequent boots.
- [ ] Audit row `kind: cron_job_created` is emitted for every gateway POST.
- [ ] Cross-tenant integration test passes: tenant A creates a job, tenant B's `GET /cron` does NOT see it.
- [ ] Migration test passes: write a `*.json` job file with a phantom tenant, boot the scheduler, verify the file has been rewritten to the canonical tenant.
- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] `cargo test -p argo-cli --test cron_tenant_isolation` passes.
- [ ] `cargo test -p argo-cli --test cron_orphan_rewrite` passes.

---

## Gotchas

1. **`store.ensure_workspace()` returns a future**: callsites that were
   previously sync must become async — that ripples through the dispatch
   layer. Match the pattern in `routes_worker_trust_grade.rs` exactly.
2. **`TenantId::new()` is a valid API** — it's used legitimately for fresh
   tenant creation, test fixtures, and synthetic IDs. Don't try to delete
   it. The fix is at the callsites, not the surface.
3. **The orphan-rewrite migration assumes single-tenant-per-workspace** —
   correct for v1 but breaks the day multi-tenant workspaces ship. Add a
   `multi_tenant` feature-flag guard so the migration is a compile error
   in that future build.
4. **Test fixtures intentionally use synthetic tenants**: `system_jobs_smoke.rs`
   creates a fresh `TenantId::new()` because the test owns its workspace.
   Don't "fix" those — they're not bugs.
5. **`workspace.tenant_id` resolution can fail**: when the workspace
   bootstrap hasn't run (cold start, no `~/.argo/` directory), the
   resolution returns `NotFound`. Handle this — POST /cron from a cold
   workspace should return 400 with "workspace not initialized" rather
   than 500.
6. **Scheduler fire-time check + migration overlap**: if the migration
   rewrites on boot, the fire-time check should rarely catch anything.
   That's fine — defense-in-depth. Don't try to make them mutually
   exclusive.
7. **Audit emission on the CLI path**: the `argo cron add` CLI doesn't
   currently emit audit rows because there's no audit sink in the CLI
   context. This brief adds emission only on the gateway POST path. A
   follow-up can add CLI audit if needed.

---

## References

- The bug: `crates/argo-cli/src/handlers/gateway/routes_cron.rs:~150` + `crates/argo-cli/src/handlers/cron_files.rs:~80`
- Correct pattern: `crates/argo-cli/src/handlers/worker_create.rs` (threads real tenant_id)
- Resolve-tenant pattern: `crates/argo-cli/src/handlers/gateway/routes_worker_trust_grade.rs` — `resolve_workspace_tenant`
- Audit kind pattern: `crates/argo-audit/src/event.rs`
- Related: `web-audit-kinds-for-journey-events` brief (adds `CronJobFired` + `CronJobDeleted`)
- Audit: `docs/skycot/audit/user-journey/2026-05-20-wave-14-workflow-ops.md` (the user-journey audit that flagged this)
