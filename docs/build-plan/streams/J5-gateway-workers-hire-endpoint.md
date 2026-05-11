# Stream J5 — Gateway hire endpoint (`POST /api/v1/workers`)

**Owner**: claude. **Depends on**: J4 MERGED (Genesis HTTP exposure so hire context comes through the same path).
**Authored against**: origin/main `9c9e899c` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J5-gateway-workers-hire-endpoint`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 5 of 18 — unblocks frontend E2-e hire commit.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Quota enforcement | None — any authenticated user may hire | Per-tenant worker-count quota with admin override | `mission-enterprise-tenant-worker-quota` |
| Multi-approval | Hire commits on the requesting Principal's authority | Department-lead approval workflow for worker hires in regulated tenants | `mission-enterprise-hire-multi-approval` |
| Hire-context bundle | Single audit row capturing problem + answers + draft + workerId | Full hire-event bundle (input + classify response + draft + adjust history + commit) signed and exportable | `mission-enterprise-hire-audit-bundle` (named earlier in E2-e) |
| Voice profile materialization | Stored as embedded JSON on the worker row | Promoted to its own table with version history; rollback supported | `mission-enterprise-voice-profile-versioning` |
| Role / team / department binding | Optional; nullable in v1 | Required for enterprise tenants; default to operator's own team if unspecified | `mission-enterprise-worker-org-binding-required` |

v1 is acceptable: hire commits create a Worker row
with a tenant_id and the draft fields; the
architectural target's org-binding + multi-approval
are policy layers on top.

## Why

E2-e's frontend brief POSTs to `/api/v1/workers` to
commit a hire. Production mode hard-errors on 404
(dev mode falls back to a stub workerId). This brief
lands the real endpoint so production hire-flow
actually creates a worker.

## Scope

- 1 JSON endpoint:
  - `POST /api/v1/workers` body
    `{ worker_draft: WorkerDraft, problem: string, answers: Record<string, QuestionAnswer> }`
    → `201 { worker_id: string, success: true }`.
- Hire-context payload is audit-logged as a single
  row.
- Tenant binding from `Principal.tenant_id`.
- Voice profile stored embedded on the worker row
  (matches existing `mission-voice-profile-richer-
  fields` MERGED schema).
- Duty list materialized: each duty in the draft
  becomes a row in `worker_duty` with cron + entry
  action.
- Memory namespace allocated atomically.
- Default approval policy set per the draft's
  `approval_policy.level` (conservative / balanced
  / aggressive).
- Idempotency key: client may send
  `Idempotency-Key: <uuid>` header; same key →
  returns the existing workerId (no duplicate
  creation).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
    (narrowly add 1 arm)
  - `crates/argo-identity/src/store.rs` (Worker
    creation — extend the existing API)

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs`
  — NEW. The hire-commit handler. ≤ 400 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire_tests.rs`
  — NEW. ≥ 10 unit tests. ≤ 450 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
  — narrowly scoped: 1 arm for POST /api/v1/workers.
  ≤ 300 LOC after.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1-line module add.
- `crates/argo-identity/src/store.rs` — narrowly
  scoped: add `IdentityStore::create_worker(draft,
  tenant_id, principal_id) -> Result<WorkerId>`.
  Existing CRUD untouched. ≤ 400 LOC after.
- `crates/argo-identity/src/store_tests.rs` —
  narrowly scoped: extend with 4 tests on
  `create_worker`. ≤ 450 LOC after.

## Files you must NOT touch

- `crates/argo-genesis/**` — read-only.
- Other Track-J handler files.
- Existing Worker / Persona / Role schemas (the
  fields needed already exist).

## Acceptance criteria

- [ ] `POST /api/v1/workers` with valid body returns
      201 + `{ worker_id, success: true }`.
- [ ] Worker row inserted with:
      - tenant_id from Principal.
      - persona JSON (name + tagline + voice_profile).
      - charter text.
      - approval_policy_level from draft.
      - suggested_vertical from draft.
- [ ] Duties materialized: each duty in the draft
      creates a row in `worker_duty` with cron +
      entry_action + enabled flag.
- [ ] Memory namespace allocated: a new
      `memory_namespace` row created scoped to the
      worker.
- [ ] Hire audit row written with full context
      (problem + answers + draft + worker_id).
- [ ] Empty problem string → 400 with envelope.
- [ ] Missing `worker_draft.suggestedName` → 400.
- [ ] Idempotency: `Idempotency-Key: <uuid>` header
      makes the call idempotent. Same key + same
      body → 200 + existing workerId. Same key +
      different body → 409 Conflict.
- [ ] Bearer auth required (401 otherwise).
- [ ] 10 unit tests pass.
- [ ] `cargo build -p argo-cli -p argo-identity`
      clean.
- [ ] `cargo clippy -p argo-cli -p argo-identity
      --all-targets -- -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J5-gateway-workers-hire-endpoint`
- PR title: `gateway: POST /api/v1/workers hire endpoint (J5)`

## Notes for the implementer

- **Idempotency-key store.** Per-tenant 24h
  in-memory map: `(tenant_id, key) → worker_id`.
  Drop entries older than 24h via a periodic
  sweep. v1 in-process is fine; multi-instance
  gateway needs Redis (Track-H follow-up).

- **Atomic create.** Worker + duties + memory
  namespace must commit atomically (single
  transaction). If any insert fails, all roll
  back. Use sqlx transaction.

- **Channels preference.** The hire flow's Step 5
  (E2-g) sets channel preferences via a PATCH on
  the worker AFTER hire. J11 (gateway-worker-
  settings-endpoints) ships that PATCH. v1 hire
  creates the worker with no channel preference;
  Step 5's PATCH adds them.

- **Default channel.** If hire happens without
  any subsequent channel-prefs PATCH (user skips
  Step 5), default to the tenant's `default_
  notify_channel` setting (or 'webinbox' if
  unset). Worker can still be paged via the
  default until the user configures preferences.

- **Validation rules:**
  - `worker_draft.suggestedName.len()` ≥ 1
  - `worker_draft.charter.len()` ≥ 20
  - `worker_draft.duties.len()` ≥ 1
  - `worker_draft.approvalPolicy.level` ∈
    {"conservative", "balanced", "aggressive"}
  - At least 1 duty has `defaultEnabled: true`
    (worker isn't inert at creation if at least
    one duty is on).

- **Vertical id resolution.** `suggested_vertical`
  from the draft is a string id (e.g.
  "marketplace-seller"). Verify it exists in the
  vertical manifest catalog. Unknown vertical →
  400.

- **Audit row content.** Include the full hire
  context but redact PII in the audit-log copy
  (use J2's `redact_text` for the problem +
  answers fields). The worker row itself stores
  raw values — those are inside the worker's
  scope, not the audit log.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-tenant-worker-quota` —
  per-tenant worker-count quota.
- `mission-enterprise-hire-multi-approval` —
  department-lead approval workflow.
- `mission-enterprise-voice-profile-versioning`
  — voice profile in its own table with
  versioning.
- `mission-enterprise-worker-org-binding-required`
  — required org binding for regulated tenants.
