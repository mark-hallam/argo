# V1.1 — Worker dependency graph + cycle detection

**Owner**: claude. **Depends on**: claude-worker-team-roster-cli
MERGED (depends on the same `worker` table being canonical).

## Why

Real org delegations have dependencies: the AR clerk depends on the
accountant for tax-coded ledger pulls; the recruiter-screener depends
on the calendaring worker for slot booking. Today nothing captures
this — handoff_routing tracks weights, not declared dependencies.

When a worker breaks (offline / failing / promoted), the operator has
no view of "what other workers will be impacted". A dependency graph
is the missing structural lens.

## Scope

- New `worker_dependency(a_worker_id, b_worker_id, kind, declared_at,
  note, PRIMARY KEY (a_worker_id, b_worker_id, kind))` table.
  `kind` enum: `data|approval|handoff|skill`.
- CLI:
  - `argo worker deps add <a> <b> --kind <kind> [--note <text>]`
  - `argo worker deps remove <a> <b> --kind <kind>`
  - `argo worker deps list <id> [--direction in|out|both] [--format
    markdown|json]`
  - `argo worker deps graph [--format markdown|dot|json]` — full
    workspace graph (DOT for graphviz; markdown for terminal)
  - `argo worker deps impact <id>` — transitive closure of workers
    that depend on `<id>` (downstream impact)
- Cycle detection on `add` — reject if it would create a cycle of the
  same `kind`. Cross-kind cycles allowed.

## Approach

1. Migration `43__worker_dependency.sql` — composite PK + indexes
   on both columns.
2. `crates/argo-store/src/worker_deps.rs` (NEW, ≤ 380 LOC) — CRUD
   + cycle check (DFS over same-kind edges) + transitive closure.
3. `crates/argo-cli/src/handlers/worker_deps.rs` (NEW, ≤ 380 LOC) —
   markdown / DOT renderers + cycle error formatter.

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-store/migrations/43__worker_dependency.sql` (NEW)
- `crates/argo-store/src/worker_deps.rs` (NEW)
- `crates/argo-store/src/lib.rs` — narrowly: pub mod
- `crates/argo-cli/src/handlers/worker_deps.rs` (NEW)
- `crates/argo-cli/src/cli_worker_cmds.rs` — `Deps` variant + subenum
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line

## Files you must NOT touch

- Worker struct — deps live in their own table
- handoff_routing — dependency graph is **declared** intent;
  routing is **runtime** weights. They co-exist.
- TUI — out of scope

## Acceptance criteria

- [ ] `argo worker deps add A B --kind data` inserts row
- [ ] `argo worker deps add B A --kind data` (would create cycle)
      → exits non-zero with "cycle detected: A→B→A" message
- [ ] Cross-kind: `add A B --kind data` then `add B A --kind approval`
      both succeed
- [ ] `argo worker deps list X --direction out` shows X's
      dependencies; `--direction in` shows dependents
- [ ] `argo worker deps graph --format dot` emits valid DOT
      (round-trip via `dot -Tsvg` in CI optional)
- [ ] `argo worker deps impact <id>` returns transitive set sorted
      by depth
- [ ] 12 unit tests (cycle, cross-kind, transitive closure depth,
      empty graph) + 3 CLI smoke + 1 DOT snapshot
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-dependency-tracking`
- PR title: `worker: dependency graph + cycle detection CLI (claude-worker-dependency-tracking)`

## Notes

- DFS for cycle detection: standard 3-color (white/gray/black). Cap
  graph size at 10k edges to avoid pathological DFS.
- `impact` query is iterative BFS, depth-limited to 8 levels (enough
  for any reasonable org).
- DOT output: nodes labeled with worker display name; edges colored
  by kind (data=blue, approval=red, handoff=green, skill=purple).
