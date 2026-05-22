# V1.1 — Compliance audit-dump CLI

**Owner**: codex. **Depends on**: H-audit-query-extensions
(#833), H-action-replay (#826) merged.

## Scope

SOC2 / GDPR / generic-audit auditors need a one-command dump
of the platform's audit trail for a date range. This wraps
the existing `argo audit query` filters into a shippable
single-file bundle.

- New: `argo audit export --since <date> --until <date>
  --format ndjson|csv|tarball [--bundle-out <path>]
  [--worker-id <id>] [--tenant <id>]`
- Tarball mode bundles: `audit.ndjson` (the records),
  `manifest.json` (date range, query parameters, generator
  version, sha256 of audit.ndjson), `README.txt` (human
  notes for the auditor)
- ndjson + csv modes write to stdout or `--bundle-out`
- Hash chain integrity check runs before export — exit
  non-zero with clear message if chain is broken
- `--gdpr-redact-emails` strips email addresses from
  payloads (replaces with `redacted-<hash>@example.com`)
  for GDPR data-minimization

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/audit_export.rs` (NEW)
- `crates/argo-cli/src/cli_audit_cmds.rs` — narrowly:
  `Export` subcommand variant
- `crates/argo-cli/tests/audit_export_smoke.rs` (NEW)

## Acceptance criteria
- [ ] All 3 formats (ndjson / csv / tarball) produce
      well-formed output
- [ ] Chain integrity check runs + reports break clearly
- [ ] `--gdpr-redact-emails` replaces emails consistently
- [ ] Tarball includes manifest + README + ndjson
- [ ] `--json` follows universal envelope (returns export
      summary, not the records themselves)
- [ ] 6 unit tests (formatter + redactor) + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-compliance-export-cli`
- PR title: `audit: argo audit export for SOC2/GDPR-ready dumps (codex-compliance-export-cli)`
