# V1.1 — Delegation contract TOML template + apply CLI

**Owner**: claude. **Depends on**: claude-worker-charter-storage MERGED,
claude-worker-kpi-tracker MERGED, claude-worker-okr-cli MERGED.

## Why

Mission claim 2: "Delegation is the unit. 'Be my marketing intern' or
'handle inbound recruiter messages' is the contract — not 'run this one
job.'" Today an operator must hand-bootstrap charter / KPIs / OKRs via
three separate CLIs. There is no single artifact that captures a
delegation as a versioned, reviewable contract.

This brief introduces the `<role>.contract.toml` artifact + an apply
command that materializes it into the underlying store.

## Scope

- TOML schema (`docs/skycot/contracts/SCHEMA.md`):
  ```toml
  [contract]
  role = "ar-clerk"
  charter = "Pursue outstanding receivables ..."

  [[duties]]
  id = "send-reminder"
  description = "Send dunning reminder at +30/+45/+60 days"

  [[kpis]]
  name = "dso_days"
  description = "Days sales outstanding"
  target = 35
  comparison = "lte"

  [[objectives]]
  period = "2026Q2"
  title = "Drive DSO under 30"
  [[objectives.key_results]]
  kpi = "dso_days"
  target = 30
  comparison = "lte"
  weight = 1.0

  [success_criteria]
  min_sessions = 50
  min_success_rate = 0.85
  ```
- 3 starter templates: `ar-clerk.contract.toml`,
  `recruiter-screener.contract.toml`,
  `marketing-intern.contract.toml` — each ~50 lines, ready to apply.
- CLI:
  - `argo contract apply <file> [--worker <existing-id>] [--dry-run]`
  - `argo contract validate <file>` — schema check only
  - `argo contract show <worker-id>` — render current contract back
    out as TOML

## Approach

1. `crates/argo-cli/src/handlers/contract.rs` (NEW, ≤ 420 LOC) — TOML
   parse via existing `toml` workspace dep, then sequence:
   create/update charter, upsert KPI rows (definitions only — values
   come from `worker kpi record`), upsert objectives + KRs.
2. `--dry-run` prints planned mutations without writing.
3. Templates under `docs/skycot/contracts/*.toml` + `SCHEMA.md`.

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-cli/src/handlers/contract.rs` (NEW)
- `crates/argo-cli/src/cli.rs` — narrowly: `Contract` top-level subcmd
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line
- `docs/skycot/contracts/ar-clerk.contract.toml` (NEW)
- `docs/skycot/contracts/recruiter-screener.contract.toml` (NEW)
- `docs/skycot/contracts/marketing-intern.contract.toml` (NEW)
- `docs/skycot/contracts/SCHEMA.md` (NEW)
- `crates/argo-cli/tests/contract_apply_smoke.rs` (NEW)

## Files you must NOT touch

- store crate — apply is a thin orchestrator over existing
  `worker_kpi.rs`, `worker_okr.rs`, charter storage modules
- migrations — no new schema in this brief
- worker creation flow — `--worker` flag updates existing only;
  creation stays via existing `argo worker create`

## Acceptance criteria

- [ ] All 3 templates parse cleanly via `argo contract validate`
- [ ] `argo contract apply ar-clerk.contract.toml --worker <id>
      --dry-run` prints planned mutations
- [ ] Real apply round-trips through `argo contract show` (semantic
      equality — TOML key ordering may differ)
- [ ] Schema errors reported with line numbers
- [ ] 8 unit tests + 2 CLI smoke + 1 round-trip integration
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-delegation-contract-template`
- PR title: `contract: TOML delegation contract apply/validate/show (claude-delegation-contract-template)`

## Notes

- This is the user-facing "hire a worker" entry point — the templates
  matter as much as the code. Make them feel like real role descriptions,
  not toy examples.
- Re-use `toml` crate already in workspace — do NOT add a new dep.
