# claude-workflow-create-unique-collision-as-409

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `6e5ab8d5b` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-workflow-create-unique-collision-as-409`.

🟡 **P2 ERROR-CONTRACT HYGIENE.** Source: evaluator round-7 SHARED.md handoff 2026-05-23T06:30:00Z.

> "🟡 claude-workflow-create-unique-collision-as-409 — concurrent create returns generic 500 not 409 (P2)"

When two requests POST the same `(tenant, vertical, name)` triple concurrently, the SQL UNIQUE constraint correctly prevents a duplicate row (round-7 verified). But the loser of the race gets a generic `500 Internal Server Error` instead of the canonical `409 Conflict`. This is bad client UX — the client can't distinguish "your workflow couldn't be created because someone else just created it with that name" from "server is broken."

## Scope

Map the SQLite UNIQUE-constraint-violation error to `409 Conflict` with a body that names the colliding triple:

```json
{"error": "workflow already exists", "vertical": "<v>", "name": "<n>"}
```

NOT a generic 500.

### Where the mapping lives

Find the create handler via `rg "fn .*workflow_create|POST.*workflows" crates/argo-cli/src/handlers/gateway/`. The path is likely:

```rust
match store.insert_workflow(...).await {
    Ok(row) => Ok(json_201(row)),
    Err(WorkflowStoreError::Sqlx(e)) if is_unique_violation(&e) => Err(conflict(...)),
    Err(e) => Err(server_error(format!("workflow create: {e}"))),
}
```

If `WorkflowStoreError` doesn't have a `UniqueViolation` typed variant, add it:

```rust
pub enum WorkflowStoreError {
    Sqlx(sqlx::Error),
    UniqueViolation { vertical: String, name: String },
}
```

Map at the store layer (preferred — error variant carries the colliding fields) OR at the handler layer (simpler, but less typed). Pick the store-layer typing.

### Regression test

Spawn two concurrent `tokio::spawn` writes of the same triple → assert one gets `Ok` (201 status) and the other gets `Err(WorkflowStoreError::UniqueViolation { vertical, name })` with the right fields. Handler-level test: same setup → assert one HTTP 201 + one HTTP 409 with the expected body.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-store/src/workflow_def.rs` (or store layer location) — narrowly scoped: add typed variant + map at insert.
- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — narrowly scoped: handle the new variant.
- `crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs` — concurrent-create regression test.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The UNIQUE SQL constraint itself — it's correct.
- Unrelated workflow store methods.
- `argo-audit` crate.

## Acceptance criteria

- [ ] Concurrent POST returns 201 + 409 (not 201 + 500).
- [ ] 409 body names the colliding `vertical` + `name`.
- [ ] Typed error variant `WorkflowStoreError::UniqueViolation { vertical, name }` exists.
- [ ] Regression test passes.
- [ ] `cargo clippy -p argo-cli -p argo-store --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-workflow-create-unique-collision-as-409`
- PR title: `fix(gateway): workflow create returns 409 not 500 on (vertical, name) collision (claude-workflow-create-unique-collision-as-409)`

## Notes for the implementer

- ~100-180 LOC expected. Variant + mapping ~40, handler ~15, tests ~80.
- **The sqlite-specific error detection**: use `sqlx::Error::Database(db)` then `db.is_unique_violation()`. Check the existing `is_unique_violation` helper if one exists in argo-store; otherwise add it (small, ~10 LOC).
- **Don't bikeshed the 409 body shape** — match the workflows handler convention if there is one. If not, the shape proposed above is fine.
- Cite evaluator round-7 verbatim.
