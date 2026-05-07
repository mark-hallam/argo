# Surface spend anomaly rows via `argo costs anomalies`

**Owner**: claude. **Depends on**: none.

> SOURCE: production hardening Phase C. `crates/argo-store/src/spend_anomaly.rs`
> records anomaly rows (z-score deviations from
> `spend_baseline`), but there is no CLI surface that lets a
> human inspect them. This brief adds a read-only CLI that
> queries the table and prints a markdown table or JSON.

## Scope

New CLI subcommand `argo costs anomalies [--since <date>] [--limit <n>] [--format markdown|json] [--tenant <id>]`.

Behavior:

- Default: prints a markdown table of the most recent 50 spend
  anomalies, columns: timestamp, tenant_id, action, baseline,
  observed, z_score, severity.
- `--since <date>`: filter anomalies recorded after the given
  date (RFC3339 or `YYYY-MM-DD`). Default: last 7 days.
- `--limit <n>`: cap rows. Default 50; hard max 1000.
- `--format json`: emit a JSON array of records (one object per
  anomaly) for piping into jq / dashboards.
- `--tenant <id>`: filter to one tenant.

This is read-only. No mutations, no deletions. The handler
queries `spend_anomaly` via the existing `ArgoStore::list_spend_anomalies`
helper (verify exact name during implementation; if it doesn't
exist, add a narrow wrapper in `argo-store::spend_anomaly` —
keep the file under 500 LOC).

## Files you may touch

- `crates/argo-cli/src/handlers/costs_anomalies.rs` (NEW) —
  ~130 LOC; clap subcommand definition, query, markdown render,
  json render
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: register the
  new handler
- `crates/argo-cli/src/cli.rs` — narrowly: add `Costs::Anomalies`
  variant under the existing `Costs` subcommand enum
- `crates/argo-cli/src/main.rs` — narrowly: dispatch to handler
- `crates/argo-cli/tests/help_smoke.rs` — narrowly: add the new
  variant's help line to the pinned snapshot
- `crates/argo-cli/tests/costs_anomalies_e2e.rs` (NEW) — 3 tests:
  empty state, 5-row populated state with markdown format,
  same with json format
- `crates/argo-store/src/spend_anomaly.rs` — narrowly (only if
  needed): a `list_anomalies(filter)` helper. File is < 500
  LOC; keep it that way.

## Files you must NOT touch

- The `spend_anomaly` schema (migrations dir). Read-only consumer.
- Any other handler file. Self-contained subcommand.
- The `costs` subcommand's existing variants — additive only.

## Acceptance criteria

- [ ] `argo costs anomalies` runs and prints a markdown table
- [ ] `argo costs anomalies --format json` emits valid JSON
      (parses with `jq`)
- [ ] `--since`, `--limit`, `--tenant` flags filter correctly
- [ ] Default `--limit` is 50; max 1000 (clap validator rejects
      higher with a clear error message)
- [ ] Empty result prints "No anomalies in window" (markdown)
      or `[]` (json) — never errors
- [ ] Help-smoke test pins the new clap variant
- [ ] 3 e2e tests pass under `cargo test -p argo-cli --test costs_anomalies_e2e`
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli && cargo test -p argo-store` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-spend-anomaly-vis`
- PR title: `cli: argo costs anomalies (read-only spend anomaly view) (claude-spend-anomaly-vis)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies — `cli.rs`, `main.rs`,
  `handlers/mod.rs`, `tests/`, `Cargo.toml` are allowed in this
  brief.
- Markdown table renderer should reuse the existing helper used
  by `argo costs summary` (or whatever is in the costs handler
  family today) — do NOT roll a new one.
- The `severity` column should map z_score to a label
  (`info`, `warn`, `crit`) using the existing thresholds in
  `spend_anomaly.rs` — read them, don't redefine.
- DO NOT add a `--threshold` filter in this brief; that's a
  follow-up if anyone asks.
