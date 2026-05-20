# web-gateway-endpoints-worker-skills-batch — implement 5 worker-skills endpoints

**Owner**: claude (Rust crate work — gateway side; codex acceptable too).
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-worker-skills-batch`.
**Priority**: 🔴 **P0 demo-blocker** — wave-3 API-audit's "single highest-
priority missing endpoint" is `GET /api/v1/workers/<id>/skills`. The
design-locked worker-skills tab (Active / Library / Learned) is 100% mock
today. Closes the worker-detail page's biggest API gap + unblocks the
"AI worker learns and gets promoted" narrative from the 2026-05-19 worker-
detail design lock.
**Sizing**: ~700-900 LOC across argo-store (existing `worker_skill` table
extension) + argo-skill (state-transition helpers) + argo-cli gateway (5
handlers) + argo-audit (1 reused kind) + tests + web wire-up.

---

## Why this brief exists

From wave-3 API-wiring audit (verbatim):

> All 5 worker-skills endpoints (`GET`/`POST`/`DELETE`
> `/api/v1/workers/<id>/skills*`) — the Active / Library / Learned
> tab is 100% mock.
>
> **Single highest-priority missing endpoint:**
> `GET /api/v1/workers/<id>/skills`. It closes the worker-detail
> page's biggest mock surface and unblocks the "AI worker learns
> and gets promoted" narrative locked in the 2026-05-19 worker-
> detail design.

The CLI substrate exists: `argo worker skills <id>` lists available skills
via `SkillStore::available_for_worker(reader, tenant, worker)`. **That logic
returns "Active" — skills the worker can call right now.** Missing:
- "Library" — skills installed in the tenant but NOT bound to this worker.
- "Learned" — skills this worker proposed via the learning loop but not yet
  reviewed.

Three-state list + 4 mutating endpoints unlock the trust-ratchet promotion
UX.

---

## Pre-flight verification

```bash
# 1. Confirm CLI substrate works (proves SkillStore::available_for_worker exists).
rg "available_for_worker" crates/argo-skill/src/ -l

# 2. Confirm no gateway handler exists yet for worker-skills.
rg "/api/v1/workers/.*/skills" crates/argo-cli/src/handlers/gateway/

# 3. Confirm the worker_skill table substrate exists.
rg "CREATE TABLE.*worker_skill" crates/argo-store/migrations/

# 4. Confirm the trust-grade reader pattern.
rg "WorkerTrustReader" crates/argo-skill/src/ -A 3 | head -20

# 5. Confirm AuditKind::SkillCandidatePromoted is the reusable variant.
rg "SkillCandidatePromoted" crates/argo-audit/src/event.rs -B 1 -A 4
```

---

## The 5 endpoints

| Verb | Path | Body | Returns |
|------|------|------|---------|
| GET | `/api/v1/workers/<id>/skills` | — | `{ active: [...], library: [...], learned: [...], total_counts: { active, library, learned } }` |
| POST | `/api/v1/workers/<id>/skills` | `{ skill_id, trust_grade_min }` | `{ ok, binding_id }` — move skill from Library → Active |
| DELETE | `/api/v1/workers/<id>/skills/<skill_id>` | — | `{ ok }` — move skill from Active → Library |
| GET | `/api/v1/workers/<id>/skills/<skill_id>/history` | — | `{ events: [{kind, at, payload}, ...] }` |
| POST | `/api/v1/workers/<id>/skills/<skill_id>/promote` | `{ to_trust_grade }` | `{ ok, from_grade, to_grade }` — Learned → Active OR raise existing Active's min_trust |

---

## Phase 1 — Tri-state derivation (no schema change)

The simpler v1 path: derive state from existing data instead of adding a
new `state` column. The three buckets:

- **Active**: skill rows where `SkillStore::available_for_worker` returns
  them AND a row exists in `worker_skill_binding` joining worker+skill.
- **Library**: skill rows in the tenant's `skill` table that ARE NOT in
  `worker_skill_binding` for this worker.
- **Learned**: skill rows where `provenance = "learned_by_worker"` AND
  `lifecycle_state != "approved"` (i.e., proposed but not promoted).

If `worker_skill_binding` doesn't exist on origin/main today, this brief
adds it via additive migration (see Phase 2). Verify with:

```bash
rg "worker_skill_binding" crates/argo-store/ -l
```

### Phase 2 — Migration (conditional)

If `worker_skill_binding` is missing:

**File**: `crates/argo-store/migrations/NNN__worker_skill_binding.sql` (next
free slot — check `scripts/check-migration-slots.sh`).

```sql
-- Stream `web-gateway-endpoints-worker-skills-batch` — per-worker
-- explicit skill bindings. The CLI's "available_for_worker" gives us
-- the union of in-tenant skills the worker COULD call; this table
-- records the operator-curated subset the worker actually has.
--
-- Slot rationale: highest existing filename slot + 1. Versioned slot
-- = max(v*) + 1. Additive — never modifies existing rows.

CREATE TABLE IF NOT EXISTS worker_skill_binding (
    id              TEXT      NOT NULL PRIMARY KEY,
    tenant_id       TEXT      NOT NULL,
    worker_id       TEXT      NOT NULL,
    skill_id        TEXT      NOT NULL,
    bound_at        INTEGER   NOT NULL,
    bound_by        TEXT      NOT NULL,
    min_trust_grade INTEGER   NOT NULL DEFAULT 0,
    UNIQUE(tenant_id, worker_id, skill_id)
);

CREATE INDEX IF NOT EXISTS idx_worker_skill_binding_worker
    ON worker_skill_binding(tenant_id, worker_id);
CREATE INDEX IF NOT EXISTS idx_worker_skill_binding_skill
    ON worker_skill_binding(tenant_id, skill_id);
```

Then `crates/argo-store/src/migrations/vNN.rs` wraps it the same way
`v76.rs` wraps `125__notifications.sql` (use that as the template).

### Phase 3 — Store ops

**File**: `crates/argo-store/src/worker_skill_binding.rs` (new) — extends
the read/write pattern from `notifications.rs`.

```rust
//! Per-worker skill bindings — the curated subset of in-tenant
//! skills a worker has been granted via gateway POST /workers/<id>/skills.
//! Active = skill in this table. Library = in-tenant skill NOT in this
//! table. Learned = derived from the `skill.provenance` column.

use crate::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerSkillBinding {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub worker_id: String,
    pub skill_id: String,
    pub bound_at_micros: i64,
    pub bound_by: String,
    pub min_trust_grade: u8,
}

pub async fn create_binding(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
    skill_id: &str,
    bound_by: &str,
    min_trust_grade: u8,
) -> ArgoResult<Uuid> {
    let id = Uuid::now_v7();
    let now = chrono::Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO worker_skill_binding \
         (id, tenant_id, worker_id, skill_id, bound_at, bound_by, min_trust_grade) \
         VALUES (?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(tenant_id, worker_id, skill_id) DO UPDATE \
         SET min_trust_grade = excluded.min_trust_grade",
    )
    .bind(id.to_string())
    .bind(tenant.to_string())
    .bind(worker_id)
    .bind(skill_id)
    .bind(now)
    .bind(bound_by)
    .bind(i64::from(min_trust_grade))
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("create_binding: {e}")))?;
    Ok(id)
}

pub async fn delete_binding(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
    skill_id: &str,
) -> ArgoResult<u64> {
    let result = sqlx::query(
        "DELETE FROM worker_skill_binding \
         WHERE tenant_id = ? AND worker_id = ? AND skill_id = ?",
    )
    .bind(tenant.to_string()).bind(worker_id).bind(skill_id)
    .execute(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("delete_binding: {e}")))?;
    Ok(result.rows_affected())
}

pub async fn list_bindings_for_worker(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
) -> ArgoResult<Vec<WorkerSkillBinding>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, String, i64)>(
        "SELECT id, tenant_id, worker_id, skill_id, bound_at, bound_by, min_trust_grade \
         FROM worker_skill_binding \
         WHERE tenant_id = ? AND worker_id = ? \
         ORDER BY bound_at DESC",
    )
    .bind(tenant.to_string()).bind(worker_id)
    .fetch_all(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("list_bindings: {e}")))?;
    rows.into_iter().map(|(id, t, w, s, ba, bb, mt)| {
        Ok(WorkerSkillBinding {
            id: Uuid::parse_str(&id).map_err(|e| ArgoError::Provider(format!("{e}")))?,
            tenant_id: TenantId(Uuid::parse_str(&t).map_err(|e| ArgoError::Provider(format!("{e}")))?),
            worker_id: w, skill_id: s, bound_at_micros: ba, bound_by: bb,
            min_trust_grade: u8::try_from(mt).unwrap_or(0),
        })
    }).collect()
}

pub async fn update_min_trust(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
    skill_id: &str,
    new_min: u8,
) -> ArgoResult<()> {
    let result = sqlx::query(
        "UPDATE worker_skill_binding SET min_trust_grade = ? \
         WHERE tenant_id = ? AND worker_id = ? AND skill_id = ?",
    )
    .bind(i64::from(new_min)).bind(tenant.to_string()).bind(worker_id).bind(skill_id)
    .execute(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("update_min_trust: {e}")))?;
    if result.rows_affected() == 0 {
        return Err(ArgoError::NotFound(format!(
            "no binding for worker '{worker_id}' skill '{skill_id}'"
        )));
    }
    Ok(())
}
```

---

## Phase 4 — Gateway handlers

**File**: `crates/argo-cli/src/handlers/gateway/routes_worker_skills.rs` (new)

Five handler functions following the established `routes_notifications.rs`
pattern (dispatch_get + dispatch_post + per-handler functions). Each:

1. Reads `X-Argo-Principal` for actor.
2. Resolves tenant via `resolve_workspace_tenant(store)`.
3. Cross-tenant guard: confirms the `<id>` worker belongs to this tenant.
   Use `IdentityStore::get_worker(tenant, worker_id)` — `Err(NotFound)` →
   return 404 (NOT 403, to avoid leaking existence across tenants).
4. RBAC gate: depends on `web-rbac-jwt-role-claim-and-gateway-enforce`. For
   v1 (without RBAC), the user role can list+promote; admin can mutate
   bindings. Once RBAC lands, the capability map for this brief:
   - `GET` → `worker_read`
   - `POST /skills` → `worker_skill_bind` (admin/owner only)
   - `DELETE /skills/<id>` → `worker_skill_bind`
   - `POST /skills/<id>/promote` → `worker_skill_promote` (admin/owner only)
   - `GET /skills/<id>/history` → `worker_read`

### GET /api/v1/workers/<id>/skills

```rust
async fn list_worker_skills_response(
    worker_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store { Some(s) => s, None => return store_unavailable() };
    let tenant = match resolve_workspace_tenant(store).await { /* ... */ };

    // 1. Cross-tenant guard.
    let identities = IdentityStore::new(store);
    let worker_uuid = match Uuid::parse_str(worker_id) {
        Ok(u) => u, Err(_) => return bad_request("invalid worker id"),
    };
    let worker = match identities.get_worker(tenant.clone(), WorkerId(worker_uuid)).await {
        Ok(Some(w)) => w,
        _ => return not_found(&format!("worker '{worker_id}' not found")),
    };

    // 2. Resolve all in-tenant skills via SkillStore.
    let skill_store = SkillStore::new(Arc::clone(store));
    let trust_reader = IdentityWorkerTrustReader::new(&identities);
    let available = skill_store.available_for_worker(
        &trust_reader, tenant.clone(), worker_uuid
    ).await?;

    // 3. Load explicit bindings.
    let bindings = argo_store::worker_skill_binding::list_bindings_for_worker(
        store, &tenant, worker_id
    ).await?;
    let bound_skill_ids: HashSet<_> = bindings.iter().map(|b| b.skill_id.clone()).collect();

    // 4. Partition into active / library / learned.
    let mut active = Vec::new();
    let mut library = Vec::new();
    let mut learned = Vec::new();
    for skill in available {
        let row = json!({
            "id": skill.id.0.to_string(),
            "name": skill.frontmatter.name,
            "version": skill.frontmatter.version,
            "description": skill.frontmatter.description,
            "provenance": skill.frontmatter.provenance,
            "min_trust_grade": skill_store.get_min_trust_grade(&skill.frontmatter.name).await?
                .map(format_trust),
        });
        if bound_skill_ids.contains(&skill.id.0.to_string()) {
            active.push(row);
        } else if skill.frontmatter.provenance == "learned_by_worker"
                  && skill.lifecycle_state != "approved" {
            learned.push(row);
        } else {
            library.push(row);
        }
    }
    let body = json!({
        "active":  active,
        "library": library,
        "learned": learned,
        "total_counts": {
            "active":  active.len(),
            "library": library.len(),
            "learned": learned.len(),
        },
    });
    ("200 OK", body.to_string())
}
```

### POST /api/v1/workers/<id>/skills (Library → Active)

Body: `{ "skill_id": "...", "trust_grade_min": "tenant" }`.

```rust
async fn bind_skill_response(
    worker_id: &str,
    body: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
    sink: &dyn AuditSink,
) -> (&'static str, String) {
    /* parse body, cross-tenant guard worker, validate skill exists in
     * tenant, call worker_skill_binding::create_binding, emit
     * AuditKind::SkillCandidatePromoted (reused) with payload {
     * actor, worker_id, skill_id, from: "library", to: "active",
     * min_trust_grade } */
}
```

### DELETE /api/v1/workers/<id>/skills/<skill_id> (Active → Library)

Returns 200 + `{ ok }` if binding existed and was deleted, 404 if no binding.

### GET /api/v1/workers/<id>/skills/<skill_id>/history

Reads from `audit_record` filtered on `payload_json LIKE '%"skill_id":"<id>"%"' AND payload_json LIKE '%"worker_id":"<wid>"%'` (or with a structured `tenant_id` + `worker_id` index — see how `routes_audit.rs` does the LIKE-on-payload search). Returns event timeline.

### POST /api/v1/workers/<id>/skills/<skill_id>/promote

Body: `{ "to_trust_grade": "tenant" }`.

Two cases:
1. Binding exists → `update_min_trust(...)` to raise the floor.
2. No binding (skill is in Learned bucket) → `create_binding(...)` to move
   to Active with the requested floor.

Either way, emit `AuditKind::SkillCandidatePromoted` with payload
`{ actor, worker_id, skill_id, from: "learned"|"active",
   from_grade, to_grade }`.

---

## Phase 5 — Tests

**File**: `crates/argo-cli/src/handlers/gateway/routes_worker_skills_tests.rs` (new)

8+ tests:
1. `list_partitions_skills_into_three_buckets`
2. `bind_skill_creates_binding_and_audit_row`
3. `bind_skill_is_idempotent_on_existing_binding`
4. `delete_skill_removes_binding`
5. `delete_skill_returns_404_when_no_binding`
6. `promote_raises_min_trust_grade_on_active_binding`
7. `promote_creates_binding_when_skill_was_learned`
8. `cross_tenant_list_returns_404`
9. `history_returns_events_filtered_to_worker_skill_pair`
10. `bind_returns_400_when_skill_not_in_tenant`

---

## Phase 6 — Web wire-up

**File**: `web/lib/api/worker-skills.ts` (likely exists — check first)

The hooks `useWorkerSkills(workerId)`, `useBindSkill()`, `useUnbindSkill()`,
`usePromoteSkill()` already exist mockerd. Replace the mock with real API
calls. The envelope shape above matches what the UI expects.

**File**: `web/app/(app)/workers/[workerId]/skills/page.tsx`

Should already render `Active` / `Library` / `Learned` tabs. The 3-bucket
envelope from the GET endpoint feeds these directly. Remove any mock-
fallback once endpoints are live.

---

## may-touch

- `crates/argo-store/src/worker_skill_binding.rs` (new).
- `crates/argo-store/src/worker_skill_binding_tests.rs` (new).
- `crates/argo-store/migrations/NNN__worker_skill_binding.sql` (new, IF table missing).
- `crates/argo-store/src/migrations/vNN.rs` (new wrapper, IF migration needed).
- `crates/argo-cli/src/handlers/gateway/routes_worker_skills.rs` (new) — 5 handlers.
- `crates/argo-cli/src/handlers/gateway/routes_worker_skills_tests.rs` (new).
- `crates/argo-cli/src/handlers/gateway/routes.rs` — dispatch wiring.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — POST/DELETE arms.
- `web/lib/api/worker-skills.ts` — replace mock fallback.

## must-NOT-touch

- `argo-skill::SkillStore` — the read substrate is correct; only call it.
- `skill` table schema — no changes.
- Trust-grade engine — separate concern (see `web-worker-trust-promotion-engine`).
- Existing CLI `argo worker skills` — keep working in parallel.

---

## Acceptance criteria

- [ ] All 5 endpoints return 200 with realistic envelopes on a running gateway.
- [ ] GET returns `{ active, library, learned, total_counts }` with skills partitioned correctly.
- [ ] POST adds a skill to Active and emits `AuditKind::SkillCandidatePromoted`.
- [ ] DELETE moves a skill from Active to Library.
- [ ] Promote endpoint raises min_trust_grade or moves Learned → Active.
- [ ] Cross-tenant attempts (different worker_id) return 404.
- [ ] Cross-tenant skill_id in body returns 400 "skill not in tenant".
- [ ] 10+ unit tests pass.
- [ ] `cargo clippy -p argo-store -p argo-skill -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Web `/workers/<id>/skills` shows live data from the gateway.
- [ ] Web fallback to mock removed (mock can stay for storybook stories).

---

## Gotchas

1. **`SkillStore::available_for_worker` returns the UNION** of skills the
   worker can call now — it doesn't know about explicit Library / Learned
   distinction. The partition logic is THIS brief's concern.
2. **`provenance` field**: confirm the exact string value used by learned
   skills (`"learned_by_worker"` per the brief's assumption). If the value
   is different (e.g., `"learned"`), the partition logic needs adjustment.
3. **`min_trust_grade` u8 inversion**: same gotcha as the trust engine
   brief — `worker_skill_binding.min_trust_grade` u8 inverts `TrustGrade`'s
   enum order. Use the existing `trust_grade_as_u8` / `trust_grade_from_u8`
   helpers from `crates/argo-skill/src/store_trust_grade.rs`.
4. **History endpoint payload-LIKE search is O(N)**: for v1 this is OK
   because audit_record is bounded; for v2 add a (`tenant_id`, `worker_id`,
   `skill_id`) index column derived during emit.
5. **DELETE on a Library skill** (already not bound) returns 404 not 200 —
   match the standard "no resource to delete" semantics.
6. **Bind+unbind doesn't emit `SkillRolledBack`** — that variant is for
   skill-VERSION rollback. The cross-cutting "skill removed from worker"
   event reuses `SkillCandidatePromoted` with a `from: "active"` /
   `to: "library"` payload field. Audit consumers filter on that.

---

## References

- CLI substrate: `crates/argo-cli/src/handlers/worker_skills.rs::gather`
- SkillStore: `crates/argo-skill/src/lib.rs::available_for_worker`
- u8 ↔ TrustGrade: `crates/argo-skill/src/store_trust_grade.rs`
- Sibling pattern: `crates/argo-cli/src/handlers/gateway/routes_notifications.rs`
- Cross-tenant guard pattern: `crates/argo-cli/src/handlers/gateway/routes_workflows.rs`
- Audit kind: `crates/argo-audit/src/event.rs::SkillCandidatePromoted`
- Audit: `docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md`
- Design lock: `project_worker_detail_design_locked.md` (memory)
