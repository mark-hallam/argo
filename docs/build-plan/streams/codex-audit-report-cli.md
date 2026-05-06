# V2.1 — Compliance audit report CLI

**Owner**: codex. **Depends on**: codex-compliance-export-cli
(#855) merged.

## Scope

`argo audit report` extends export with formatted compliance
reports. Where `audit export` produces raw bundles for auditors
to ingest, `audit report` produces a human-readable summary
useful for board-level / executive review.

- `argo audit report --since <date> --until <date>
  --format markdown|html [--output <path>]
  [--worker-id <id>] [--tenant <id>]`
- Sections in the report:
  - Summary stats: actions executed, approvals granted, escalations triggered, anomalies detected
  - Approval-decision breakdown by NotifyChannel
  - Top 10 spending workers + their breakdowns
  - Pending-approval queue snapshot
  - Hash-chain integrity check verdict
  - Pre-existing-flake list (from BUILD_STATUS tick-60 amendment)

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/audit_report.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-cli/src/cli_audit_cmds.rs` — narrowly: `Report`
  variant
- `crates/argo-cli/tests/audit_report_smoke.rs` (NEW)

## Acceptance criteria
- [x] `markdown` format renders all 6 sections
- [x] `html` format produces valid standalone HTML
- [x] `--output` writes to file; absent → stdout
- [x] `--json` envelope (universal convention)
- [x] 4 unit tests + 1 smoke
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-audit-report-cli`
- PR title: `audit: argo audit report — formatted compliance summary (codex-audit-report-cli)`
