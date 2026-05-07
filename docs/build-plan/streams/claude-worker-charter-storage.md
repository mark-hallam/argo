# Worker charter + KPI storage (mission claim 2: delegation contract)

**Owner**: claude. **Depends on**: —

## Why

Mission claim 2 ("Workers operate under a written charter; the charter
states the work, the boundaries, the KPIs") today has no backing fields
on `Worker`. Persona is voice; Role is the role-name string; charter and
KPIs are nowhere on disk. This brief adds two columns and threads them
through the `Worker` struct, persistence, and a small CLI surface for
managing them.

## Scope

- Schema migration: extend the `worker` table with `charter_md TEXT NULL`
  + `kpi_json TEXT NULL` (KPI is structured but the smallest viable shape
  is a JSON blob for now — the engine just stores and renders it).
- `argo_identity::Worker` gains two fields: `charter: Option<String>`
  (markdown body, ≤ 16 KB enforced by writer) and `kpis: Option<Value>`
  (free-form JSON, ≤ 8 KB).
- Persistence read+write paths (`argo-store` worker repo) carry both
  fields.
- CLI: `argo worker charter set <id> --file <path>` /
  `argo worker charter get <id> [--md|--json]` /
  `argo worker charter clear <id>`.
- Auto_approval, channels, etc. unchanged — narrow add.

## Approach (concrete)

1. **NEW** migration `crates/argo-store/migrations/30__worker_charter.sql`
   (~15 LOC) — `ALTER TABLE worker ADD COLUMN charter_md TEXT;
   ALTER TABLE worker ADD COLUMN kpi_json TEXT;` (SQLite-compatible).
2. **Modify** `crates/argo-identity/src/types.rs` — narrowly: add
   `charter: Option<String>` and `kpis: Option<serde_json::Value>` to
   `Worker`, both `#[serde(default)]` so older rows deserialize cleanly.
   Update `Worker::draft` to default both to `None`. Document the
   16 KB / 8 KB ceilings in the doc-comment.
3. **Modify** `crates/argo-store/src/worker_repo.rs` (or whichever file
   holds `insert_worker` / `load_worker` — verify with
   `grep -n "INSERT INTO worker" crates/argo-store/src/`) — narrow column
   additions to SELECT + INSERT.
4. **NEW** `crates/argo-cli/src/handlers/worker_charter.rs` (~180 LOC)
   handling set/get/clear; markdown render via existing helpers.
5. **NEW** `crates/argo-cli/src/handlers/worker_charter_tests.rs` (~150 LOC)
   covering all three subcommands + size enforcement.
6. **Modify** `crates/argo-cli/src/cli.rs` — narrowly: 1 enum variant
   under the existing `Worker` subcommand. EXHAUSTIVE-MATCH note:
   if any `match` over Worker subcommands exists in handlers/mod.rs,
   add the new arm there too.

## Files you may touch

- `crates/argo-store/migrations/30__worker_charter.sql` (NEW)
- `crates/argo-identity/src/types.rs` (narrow — 2 fields + draft default)
- `crates/argo-store/src/worker_repo.rs` (or equivalent — narrow column ops)
- `crates/argo-cli/src/handlers/worker_charter.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/worker_charter_tests.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (1 pub mod line + match arm if needed)
- `crates/argo-cli/src/cli.rs` (BLANKET CLI ALLOWLIST — 1 enum variant; cli.rs is at 500 LOC, may need a 2-line trim elsewhere or split if not — STOP and surface if it grows)
- `crates/argo-cli/tests/worker_charter_smoke.rs` (NEW) — 1 round-trip integration test

## Files you must NOT touch

- Other `Worker` fields (persona, role, channels, auto_approval) —
  do not refactor them
- Any other migration file
- `argo-orchestrator/` — out of scope; charter is metadata only here

## Acceptance criteria

- [ ] `argo worker charter set <id> --file foo.md` writes charter_md
- [ ] `argo worker charter get <id>` renders markdown to stdout
- [ ] `argo worker charter get <id> --json` emits `{"charter": ..., "kpis": ...}`
- [ ] `argo worker charter clear <id>` nulls both columns
- [ ] Charter > 16 KB → error before write; KPIs > 8 KB → error before write
- [ ] Older worker rows (without these columns) load cleanly via `#[serde(default)]`
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-identity -p argo-store -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-charter-storage`
- PR title: `identity: store worker charter + KPIs (claude-worker-charter-storage)`

## Notes for the implementer

- Do NOT introduce a typed `Kpis` struct yet — JSON blob is the v0
  contract. Schema can tighten in a later brief once we know the shape.
- BLANKET CLI ALLOWLIST applies to all `crates/argo-cli/` files.
- If cli.rs is at the 500-LOC ceiling, your enum variant may need a
  trivial micro-split (extract one existing variant body into a helper).
  That's allowed under HARD RULE 6.
