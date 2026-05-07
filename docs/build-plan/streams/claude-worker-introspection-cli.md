# Rich worker introspection (`argo worker show`)

**Owner**: claude. **Depends on**: —

## Why

Today, `argo identity get <id>` returns a lean Worker dump — basically
the struct. To treat workers as teammates, an operator running
`argo worker show <id>` deserves: persona + charter (when present),
channels, the last 5 sessions they touched, the last 10 audit rows,
their current duties, and spend YTD. One screen, one command, every
relevant fact.

## Scope

- New CLI `argo worker show <id>` rendering markdown by default,
  `--json` for machines.
- Composes from existing read-side surfaces (persona, channels, audit_query,
  worker_budget, role_duties, sessions). No new schema, no new migrations.
- Render is layered: missing data (no charter, no duties) renders
  cleanly with "(none)" placeholders, never a panic.
- Replaces nothing — `argo identity get` stays as the lean shape;
  `worker show` is the rich one.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/worker_show.rs` (~320 LOC):
   - `WorkerShowReport` struct aggregating all sections.
   - `assemble_report(store, id) -> ArgoResult<WorkerShowReport>` —
     6 store reads in parallel via `tokio::join!` for latency.
   - `render_markdown(&WorkerShowReport) -> String` (~120 LOC, the bulk).
   - `render_json(&WorkerShowReport) -> serde_json::Value`.
2. **NEW** `crates/argo-cli/src/handlers/worker_show_tests.rs` (~250 LOC) —
   golden-file markdown render test, JSON shape test, missing-charter
   test, missing-duties test, missing-sessions test, deleted-worker test.
3. **Modify** `crates/argo-cli/src/cli.rs` — 1 new
   `Worker::Show { id: WorkerId, json: bool }` variant.
   EXHAUSTIVE-MATCH note: handlers/mod.rs dispatch arm.
4. **NEW** `crates/argo-cli/tests/worker_show_smoke.rs` (~150 LOC) —
   seed a worker with charter + 1 channel + 1 audit row + 1 session,
   assert markdown contains all four section headings.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_show.rs` (NEW, ≤ 380 LOC — at risk; split if it grows)
- `crates/argo-cli/src/handlers/worker_show_tests.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (1 pub mod + dispatch arm)
- `crates/argo-cli/src/cli.rs` (BLANKET CLI ALLOWLIST — 1 enum variant)
- `crates/argo-cli/tests/worker_show_smoke.rs` (NEW)
- `crates/argo-cli/tests/fixtures/worker_show_golden.md` (NEW) — golden render

## Files you must NOT touch

- `argo-store/` — only call existing public read APIs
- `argo-identity/` — Worker shape unchanged
- `argo identity get` — leave the existing lean command untouched

## Acceptance criteria

- [ ] `argo worker show <id>` renders markdown with sections: Persona, Role, Charter (if present, else "(none)"), Channels, Recent Sessions, Recent Audit, Duties, Spend YTD
- [ ] `argo worker show <id> --json` emits stable JSON shape
- [ ] Missing charter → "(none)" not a panic
- [ ] Worker with no sessions → "Recent Sessions: (none)" cleanly
- [ ] Non-existent WorkerId → clean error, exit 1
- [ ] All 6 underlying store reads happen concurrently (verify via tracing or test)
- [ ] Golden-file markdown stable across runs
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC (worker_show.rs is at risk — split if you cross 400 mid-write)

## Branch + PR

- Branch: `claude-worker-introspection-cli`
- PR title: `worker: rich `argo worker show` introspection (claude-worker-introspection-cli)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies.
- "Spend YTD" comes from `worker_budget` rolled up — verify the
  existing helper before assuming. If only daily/monthly exists,
  use whichever is largest available; do NOT add a new SQL.
- Charter section depends on `claude-worker-charter-storage` if you
  want it populated; if that brief hasn't merged, render
  "Charter: (storage not yet wired)" — DON'T fail the command.
- Markdown render is large; extract section helpers (`render_persona`,
  `render_channels`, etc.) so each is ≤ 30 LOC and individually testable.
