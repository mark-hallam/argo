# claude-session-worker-id-schema — add worker_id to session table + thread through gateway + CLI

**Owner**: claude. **Depends on**: nothing — schema gap that has bounced 3+ tabs (memory-compose-context's per-worker scope blocked on this; bearer-auth audit-stamping needs principal which needs worker; per-Principal duty scheduling needs this). **Phase B claim-1 closure brief**.

## Why

`SessionHeader` (in `crates/argo-store/src/models.rs:11`) has `tenant_id` + `vertical_id` + `workspace_id` but **no `worker_id`**. The `session` table DDL (`crates/argo-store/src/migrations/v1.rs:36`) defines the same shape. This blocks every "per-worker" feature in the mission claim list:

- `MemoryNs::worker(tenant, worker_id, scope)` cannot be resolved from a session — only `shared_team(tenant)` works (this is why #945 shipped workspace-shared-only as a v1).
- Per-principal audit-row stamping (#946 follow-up) cannot identify which worker performed an action.
- Duty-scheduler (#939) ties duties to workers but the corresponding session has no link back.

This brief adds the column, threads it through the gateway POST `/sessions` route + CLI `--worker` flag, and unblocks the cascade.

## Scope

1. **Schema migration** — NEW `crates/argo-store/migrations/38__session_worker_id.sql`:
   ```sql
   ALTER TABLE session ADD COLUMN worker_id BLOB;
   CREATE INDEX idx_session_worker ON session(worker_id) WHERE worker_id IS NOT NULL;
   ```
   Nullable, partial index — every existing row keeps NULL (back-compat).

2. **`SessionHeader` + `SessionHeaderRow`** — narrowly add `pub worker_id: Option<WorkerId>` (and the BLOB Vec<u8> form on the row). `WorkerId` is at `argo_identity::types::WorkerId` (verified at `crates/argo-identity/src/types.rs:19`). Add `#[serde(default, skip_serializing_if = "Option::is_none")]` so old JSON deserialises (matches the `vertical_id` pattern at line 31).

3. **Insert + select queries** — update `session_meta.rs` insert + `session_query.rs` select to include `worker_id`. Both files have ample LOC headroom (157 / 214 LOC).

4. **Gateway POST `/sessions`** — accept optional `worker_id` from request body. Currently the route lives in `routes_post_sessions.rs` (183 LOC). Parse from JSON body, validate via `argo_identity::resolve_worker_id` (or equivalent — confirm exists), pass to session creation.

5. **CLI flag** — add `--worker <id-or-name>` to `argo session start`. Resolves to `WorkerId` via the existing identity lookup; passes through to gateway. (BLANKET CLI ALLOWLIST per Tick 32 covers `cli.rs` + `main.rs` + `handlers/sessions/*`.)

6. **Tests** — round-trip persistence (write + read), gateway integration smoke (`POST /sessions` with worker_id → `GET /sessions/{id}` returns worker_id), CLI smoke (`argo session start --worker <id>`).

## Files you may touch

- `crates/argo-store/migrations/38__session_worker_id.sql` (NEW, ~5 LOC)
- `crates/argo-store/src/migrations/mod.rs` or `lib.rs` — narrowly: register migration 38 (the existing v1.rs holds DDL; numbered migrations live alongside via the existing apply-migrations machinery — confirm path)
- `crates/argo-store/src/models.rs` — narrowly: add `worker_id: Option<WorkerId>` to SessionHeader + Vec<u8> form to SessionHeaderRow + conversion in `From` impl at line 216
- `crates/argo-store/src/session_meta.rs` — narrowly: update insert query + binding
- `crates/argo-store/src/session_query.rs` — narrowly: update select query + row mapping
- `crates/argo-store/Cargo.toml` — narrowly: add `argo-identity` to deps if not already there
- `crates/argo-cli/src/handlers/gateway/routes_post_sessions.rs` — narrowly: parse worker_id from request body
- `crates/argo-cli/src/handlers/sessions/*` — narrowly: add `--worker` flag handling (per BLANKET CLI ALLOWLIST)
- `crates/argo-cli/src/cli.rs` — narrowly: add `--worker <id-or-name>` arg to SessionStart variant (per BLANKET CLI ALLOWLIST)
- `crates/argo-cli/tests/session_worker_id_smoke.rs` (NEW, ≤ 200 LOC) — gateway POST + GET round-trip + CLI smoke
- `crates/argo-store/src/session_worker_id_tests.rs` (NEW, ≤ 200 LOC, sibling-pattern via `#[cfg(test)] #[path = "..."] mod tests;`) — 6 unit tests:
  - insert with worker_id → select returns same worker_id
  - insert without worker_id → select returns None
  - migration upgrade preserves existing rows (NULL worker_id)
  - JSON serialisation skips field when None
  - Gateway-shaped INSERT happy path
  - Gateway-shaped INSERT with invalid worker_id BLOB → error

## Files you must NOT touch

- `MemoryNs::worker(...)` constructor or the build_system_prompt swap — that's the **next** brief (`mission-memory-per-worker-scope`). This brief just lands the schema + plumbing.
- `routes_dispatch_post.rs` — auth-side changes are separate.
- The 8 verticals' `vertical.toml` files — worker_id is supplied at session-start, not in the vertical declaration.
- `argo_orchestrator` or `argo_judge` integrations — those don't read session.worker_id today; later briefs can extend.

## Acceptance criteria

- [ ] Migration 38 runs cleanly on a fresh DB and against an existing DB with N pre-existing sessions (all keep NULL worker_id)
- [ ] `SessionHeader` exposes `worker_id: Option<WorkerId>` with serde-default for back-compat
- [ ] Round-trip: write SessionHeader with `Some(WorkerId)` → read SessionHeader → returns same WorkerId
- [ ] Round-trip: write SessionHeader with `None` → read SessionHeader → returns None
- [ ] Gateway `POST /sessions` accepts `{"worker_id": "<uuid>"}` in request body and persists it
- [ ] Gateway `GET /sessions/{id}` returns `worker_id` field (or omits when None)
- [ ] CLI: `argo session start --worker <id-or-name>` creates a session with that worker_id
- [ ] CLI: `argo session start` (no flag) creates a session with NULL worker_id (back-compat)
- [ ] 6 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-session-worker-id-schema`
- PR title: `session: add worker_id column + thread through gateway + CLI (claude-session-worker-id-schema)`

## Notes for the implementer

- **3 prior tabs bounced off this** — the failure mode was treating it as "just memory-compose-context's blocker" rather than as a load-bearing schema brief in its own right. Treat this as standalone foundation work.
- Migration files in `crates/argo-store/migrations/` are numbered `NN__name.sql`. Highest current is 37 (verified via `ls migrations/` — gaps from past briefs may exist; pick the next free number ≥ 38).
- `WorkerId` is `pub struct WorkerId(pub Uuid)` per `argo-identity/src/types.rs:19` — convert to/from BLOB the same way `TenantId` does (look at line 39 for the pattern).
- For the CLI `--worker` flag: parse as either UUID directly or as a name that resolves via existing identity-lookup CLI; reuse the helper if one exists, otherwise just accept UUID strings in v1 (call out in PR body).
- Boundary deviation pattern: if `argo-identity` brings in heavy deps argo-store doesn't currently use, consider re-defining a minimal `WorkerId` newtype in argo-store and converting at the boundary. Document in PR body.
- Mission claim 1 ("teammate, persistent identity over time") goes from ~50% → ~75% with this brief. The follow-up `mission-memory-per-worker-scope` brings it to ~95%.
