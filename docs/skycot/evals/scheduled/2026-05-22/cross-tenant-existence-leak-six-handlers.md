# Eval: Cross-tenant existence-leak across six worker handlers

**Date**: 2026-05-23 (round 4)
**Pillar**: Cross-pillar (mission claim 6 — multi-tenant by construction)
**Mission claim**: 6 — Multi-tenant by construction · Enterprise-grade architecture (operator memory `feedback_enterprise_grade_defaults.md`)
**Verdict**: 🟡 **P2 enterprise-grade-violation** — not a data-exposure, but a tenant existence-oracle. Six handlers distinguish "exists in another tenant" (403) from "doesn't exist at all" (404), letting a caller enumerate which worker_ids belong to other tenants.

## Setup

Round-3 surfaced an audit-export DATA leak. Round 4 walked the worker handlers to see if the cross-tenant scoping is tighter, looser, or inconsistent. Reading `routes_worker_activity.rs` first, I noticed it returns:

```rust
Ok(true)  => return forbidden(&format!("worker {worker_id} not in this tenant")),  // exists elsewhere
Ok(false) => return not_found(&format!("worker {worker_id} not found")),           // doesn't exist
```

Two distinct error codes + messages. A caller in tenant_b who knows a worker_id can distinguish "this worker belongs to a different tenant" from "this worker doesn't exist".

## Action

```bash
rg "not in this tenant|forbidden\(.*tenant" crates/argo-cli/src/handlers/gateway/ -l
rg "not in this tenant" crates/argo-cli/src/handlers/gateway/
```

## Expected

If the existence-leak is one-off, only `worker_activity.rs` shows the pattern. If it's systemic, multiple handlers show it.

## Observed

**Six handlers** use the same 403-vs-404 distinction (substring `"not in this tenant"`):

| Handler | Endpoint (representative) |
|---|---|
| `routes_worker_activity.rs` | `GET /api/v1/workers/<id>/activity` |
| `routes_worker_detail.rs` | `GET /api/v1/workers/<id>` |
| `routes_worker_overview.rs` | `GET /api/v1/workers/<id>/overview` |
| `routes_worker_policies.rs` | `GET\|PATCH /api/v1/workers/<id>/policies` |
| `routes_worker_settings_helpers.rs` | `GET\|POST /api/v1/workers/<id>/settings` |
| `routes_duties_helpers.rs` | (worker duty CRUD; mounted under several worker sub-paths) |

All six follow the same pattern:

```rust
match probe_cross_tenant_existence(store, tenant, worker_id).await {
    Ok(true)  => return forbidden(&format!("worker {worker_id} not in this tenant")),
    Ok(false) => return not_found(&format!("worker {worker_id} not found")),
    Err(e)    => return server_error(&format!("cross-tenant probe: {e}")),
}
```

The probe itself is correct — it does a `SELECT id FROM worker WHERE id = ? AND tenant_id != ? LIMIT 1`, scoped to "exists in other tenants". But the **response shape leaks the answer**.

## Contrast — the workflows handler does it right

For comparison, the workflow-definition handler at `crates/argo-cli/src/handlers/gateway/routes_workflows.rs:13-21` explicitly says:

> **Authenticated callers without the required capability get a generic `403 Forbidden`** so the response does **NOT** leak whether the `(tenant, vertical, name)` triple exists.

That handler returns 403 with no body distinction whether the resource exists or not. **Same problem, opposite solution, same codebase.** The worker handlers and the workflow handler are inconsistent.

## Anti-theater check

Not theatre — the SQL probe is real and the response strings are real. I verified by grepping the exact string `"not in this tenant"` and counting 6 distinct handlers. A second grep on `"worker.*not found"` confirms the alternative branch returns a recognizably different error.

The attack flow: a caller in tenant_b with a list of worker_ids (e.g. obtained via a side channel — a leaked URL, a guess, or a search engine cache of an earlier leak) can call `GET /api/v1/workers/<id>/overview` against each one. The HTTP status + JSON body let them sort into "exists somewhere" (403) vs "doesn't exist" (404). That's a worker_id enumeration oracle.

It's smaller than a data exposure (no row contents leak) but it's an **identifier-enumeration tool** that an attacker can use to size up the target organization's worker pool.

## Enterprise-grade hold-the-line

Per operator memory `feedback_enterprise_grade_defaults.md`:

> Enterprise-grade architecture; SMB-shaped v1 behind seams. Strict defaults are the target architecture; v1 implementations may be permissive behind tagged seams that swap to strict later.

The current behavior is "permissive" — distinguishable error codes leak existence. For enterprise customers expecting tenancy isolation guarantees, the strict version is "return 404 for both branches" (or "return 403 for both branches" — pick one consistently).

## Gap / follow-up brief candidates

**🟡 P2 — `mission-cross-tenant-worker-handler-no-existence-leak`**: Change the six handlers to return the same response shape for both branches. Two options:

**Option A** (recommended — matches the workflows handler convention): return generic `404 Not Found` with body `{"error":"worker not found"}` in BOTH the "exists in another tenant" AND "doesn't exist" cases. Probe-and-discriminate code path is preserved internally for telemetry, but the user-facing response is identical.

**Option B**: return 403 in both cases (less informative but signals "you don't have access" without revealing existence). Diverges from workflows handler convention.

Acceptance:
- Same response status code + JSON body whether the worker exists in another tenant or doesn't exist at all.
- All 6 handlers have a regression test that pins this — `cross_tenant_worker_<verb>_returns_same_404_as_unknown_worker`.
- The `routes_workflows.rs:13-21` invariant comment is replicated across the 6 fixed handlers.

~150 LOC including tests. Single sweep brief. Touches argo-cli only.

**🟢 INFO — broader inconsistency audit**: The fact that workflow_routes does it right and worker_routes does it wrong suggests this would benefit from a project-wide "tenant existence-leak audit". Worth a one-off auditor task: grep every handler for `not.*tenant\|"not found"` and compare against `routes_workflows.rs:13-21`'s explicit invariant. The 6 worker handlers found here may not be the only offenders.

## What this scenario proves

🟡 The **tenant isolation primitive** at the audit_record / worker SQL layer is sound (rows are scoped). But the **error-response layer** in 6 worker handlers leaks tenant_id × worker_id existence pairs. Not P0, but worth fixing before "enterprise" demos go out. Brief I'd queue: `mission-cross-tenant-worker-handler-no-existence-leak`.

🟢 The cross-tenant SQL-row scoping is genuinely good — six handlers do the right thing at the data layer and just leak at the response layer. That's a much smaller fix than "wire tenant scoping everywhere from scratch".
