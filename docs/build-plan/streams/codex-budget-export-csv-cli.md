# V1.4 — Budget CSV export CLI

**Owner**: codex. **Depends on**: H-budget-cli (#820) merged.

## Scope

`argo budget show/recent` (#820) emit human-readable + `--json`
output. Missing: a CSV format that operators can pipe into
spreadsheets, finance tools, or BigQuery for billing review.

Add a single small flag to existing budget commands:

- `argo budget show <worker_id> --format csv` — single-row CSV with
  headers
- `argo budget recent --format csv` — multi-row CSV
- `argo budget anomalies --format csv` (#892) — multi-row CSV
- `argo budget alerts list --format csv` (#902) — multi-row CSV

`--format` accepts `human` (default), `json` (existing), or `csv`
(new). Output goes to stdout. RFC 4180-compliant: comma-separated,
double-quoted strings with embedded `"` doubled, newline-terminated.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/budget.rs` — narrowly: add csv arm
- `crates/argo-cli/src/handlers/budget_anomalies.rs` — narrowly:
  add csv arm
- `crates/argo-cli/src/handlers/budget_watch.rs` — narrowly: add
  csv arm to alerts list (if alerts list lives here)
- `crates/argo-cli/src/cli_budget_cmds.rs` — narrowly: add
  `--format` flag if not already present, accepting human|json|csv
- `crates/argo-cli/tests/budget_csv_smoke.rs` (NEW) — verifies all
  4 commands' CSV output parses cleanly with `csv` crate

## Acceptance criteria
- [ ] `argo budget show w-1 --format csv` emits 1 header row + 1 data row
- [ ] `argo budget recent --format csv` emits 1 header row + N data rows
- [ ] `argo budget anomalies --format csv` emits 1 header row + N data rows
- [ ] `argo budget alerts list --format csv` emits 1 header row + N data rows
- [ ] CSV output round-trips via the `csv` crate's reader without
      errors (verified in smoke test)
- [ ] Embedded commas, quotes, and newlines in fields are properly
      escaped (RFC 4180)
- [ ] `--format human` and `--format json` continue to work unchanged
- [ ] 4 unit tests + 1 smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-budget-export-csv-cli`
- PR title: `budget: --format csv across budget show/recent/anomalies/alerts (codex-budget-export-csv-cli)`
