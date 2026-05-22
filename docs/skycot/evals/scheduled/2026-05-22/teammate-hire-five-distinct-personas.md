# Eval: Teammates — five-distinct-persona hire (anti-Mariah-clone)

**Date**: 2026-05-22
**Pillar**: Teammate creation
**Mission claim**: 2 — Delegation is the unit · 3 — Platform builds workers (partial)
**Verdict**: 🟡 **PARTIAL** — core hire pipeline works, audit-feed surfacing is missing.

## Setup

- Gateway running at `http://127.0.0.1:7400` (PID 28198, uptime 92h).
- Bearer auth bypassed via `ARGO_GATEWAY_AUTH_BYPASS=1` in `web/.env.local` (dev posture).
- Tenant: cwd-derived workspace tenant (per `routes_workers_hire.rs:11`).
- Pre-state: `GET /api/v1/team` returned `total_count = 0` (clean tenant).

## Action

Submit 5 distinct hires + 2 idempotency-key tests via `POST /api/v1/workers` (see `/tmp/eval-hire-payloads.sh`). Each hire used:

- Different `suggestedName` (EvalEva-DataEntry, EvalCleo-Support, EvalHarold-Bookkeep, EvalMaya-Onboard, EvalRina-LeadResearch)
- Different `suggestedVertical` (data-entry-clerk, customer-support, bookkeeping-assistant, customer-onboarding, lead-research)
- Different `charter` (4 distinct sentence patterns)
- Same `approvalPolicy.level = "balanced"` and same one-duty cron pattern (held constant to isolate the persona-uniqueness variable).

Then two idempotency tests:
1. Same idem-key, **same body** twice → must return same worker_id with `idempotent: true`.
2. Same idem-key, **different body** twice → must return 409 idempotency_conflict.

## Expected

- 5 hires → 5 × HTTP 201 with 5 distinct `worker_id` UUIDs.
- `GET /api/v1/team` returns 5 workers with the 5 names I submitted (NOT 5 clones of one default persona).
- Idempotency replay returns same worker_id + 200 + `"idempotent": true`.
- Idempotency-key-collision-with-different-body returns 409.
- `GET /api/v1/audit?range=24h` shows 5 `WorkerHired` events (or platform equivalent) with the new worker IDs and tenant.

## Observed

### Core hires ✅
```
Hire 1 — EvalEva-DataEntry      → HTTP 201  {"worker_id":"019e4d3f-b46f-7201-924b-aaab4aeea407"}
Hire 2 — EvalCleo-Support       → HTTP 201  {"worker_id":"019e4d3f-b507-7830-ad0e-4e72b4212522"}
Hire 3 — EvalHarold-Bookkeep    → HTTP 201  {"worker_id":"019e4d3f-b5a1-7900-8ea3-ff681e2faa27"}
Hire 4 — EvalMaya-Onboard       → HTTP 201  {"worker_id":"019e4d3f-b634-7790-bd94-58a6d04a18cc"}
Hire 5 — EvalRina-LeadResearch  → HTTP 201  {"worker_id":"019e4d3f-b6bb-7223-8ee3-82dedf5c07fa"}
```

`GET /api/v1/team` returned exactly 7 workers (5 above + 2 dup-test workers, see below) with the 7 distinct persona names I sent:

```
EvalReplay              | 019e4d3f-ef31-7141-bfe2-8d9cc1c2e903
EvalDup-First           | 019e4d3f-b734-7650-ab9b-aef24db989c0
EvalRina-LeadResearch   | 019e4d3f-b6bb-7223-8ee3-82dedf5c07fa
EvalMaya-Onboard        | 019e4d3f-b634-7790-bd94-58a6d04a18cc
EvalHarold-Bookkeep     | 019e4d3f-b5a1-7900-8ea3-ff681e2faa27
EvalCleo-Support        | 019e4d3f-b507-7830-ad0e-4e72b4212522
EvalEva-DataEntry       | 019e4d3f-b46f-7201-924b-aaab4aeea407
```

**Anti-Mariah-clone passed**: 7/7 personas have the names I submitted, no defaults. UUIDs are real UUID v7s (timestamp-prefixed), not stub/sequential.

### Idempotency replay ✅
```
First call:   {"success":true,"worker_id":"019e4d3f-ef31-7141-bfe2-8d9cc1c2e903"}  HTTP 201
Same body+key: {"idempotent":true,"success":true,"worker_id":"019e4d3f-ef31-7141-bfe2-8d9cc1c2e903"}  HTTP 200
```

Status code transitions correctly (201 → 200), `idempotent: true` flag added, same worker_id. Correct.

### Idempotency-collision rejection ✅
```
{"error":{"code":"idempotency_conflict","message":"Idempotency-Key 'EVAL-2026-05-22-dup-fixed-key' was previously used with a different body"}}
HTTP 409
```

Body-hash mismatch detected → 409. Correct (prevents accidental cross-contamination of two different hires with the same idem-key).

### Audit-feed surfacing 🔴 (the partial)
```
GET /api/v1/audit?range=24h&limit=50   → 4 items, all from 2026-05-18, verb=hipaa_compliance_check
GET /api/v1/audit?q=hired              → 0 items
GET /api/v1/audit?q=EvalEva            → 0 items
GET /api/v1/audit?range=lq&limit=200   → 4 items (same set as above)
```

The 7 `WorkerHired` events I just created **do not surface in the production audit feed at all**. The only rows visible are 4 days old and all of kind `hipaa_compliance_check` (which look like fixture data — `worker.id=""` and `worker.name="unknown"`).

## Anti-theater check

The hire endpoint **does** write to `audit_record` — verified at [routes_workers_hire.rs:71](crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs) line equivalent (`use argo_audit::{AuditKind, AuditSink};` + `let sink = AuditSink::new_with_tenant(...)` + `AuditKind::WorkerHired`). So this isn't theater at the write side.

But the production audit feed at `GET /api/v1/audit` (handler: `routes_audit_feed.rs:1-40`) is by-design scoped to **action records** (action_type / channel / status / approver / cost / classification — the 7 dim filters per v67 migration). `WorkerHired` is a **platform event**, not an **action event**, and is therefore invisible to the operator's audit page.

The hire write is real. The "operator can see who hired which worker via the audit page" path is **not implemented end-to-end** — the audit feed and the platform-event log are parallel surfaces.

## Gap / follow-up brief candidates

**🔴 P0 — `web-audit-feed-platform-events`**: Either (a) extend `routes_audit_feed.rs` to UNION platform events (WorkerHired, SkillCreated, SkillRejected, WorkflowRolledBack, etc.) into the feed with a `kind:platform` filter pill, OR (b) add a sibling page `/audit/platform` that surfaces `audit_record` rows where `kind IN (platform_event_set)`. Acceptance: hire a worker → row visible in operator's audit view within 1s, with worker_id / persona_name / actor_principal_id / tenant_id columns.

**🟡 P1 — `mission-hire-audit-payload-shape-validation`**: Verify the WorkerHired row actually contains the un-redacted worker_id, redacted problem text (no PII), and actor principal_id. This eval did not inspect the row directly — `sqlite3` doesn't work (SQLCipher) and there's no admin endpoint to read raw audit_record rows.

**🟢 INFO**: `GET /api/v1/team` returned `total_count: 7` after my 7 hires, suggesting the running gateway's tenant was empty before this eval. The conductor + workers use a separate tenant (or `ARGO_HOME`) for their work. Tagging the 7 EVAL workers in metadata would let conductor-side queries exclude them cleanly — currently they're just plain workers with names starting with "Eval".

## Cleanup note

The 7 EVAL workers (worker_ids above) remain in the cwd-derived workspace tenant. If you want me to issue `DELETE /api/v1/workers/<id>` or whatever the equivalent is at end of session, say so. For now they're harmless — the workers list endpoint already shows total_count: 7 / 7 are all EVAL-prefixed, easy to filter.
