# V2.1 — Data classification (PII / PHI / financial tags)

**Owner**: claude. **Depends on**: F-memory-layering (#786),
H-audit-query-extensions (#833) merged.

## Scope

Tag data at write-time (memory rows, audit payloads, action
parameters) with classifications: `pii`, `phi`, `financial`,
`secret`, `public`. Per-classification handling rules apply
to retention, redaction, audit-export, and replication.

- New enum `DataClass` (Pii / Phi / Financial / Secret /
  Public / Untagged)
- New table `data_classification_tag(record_kind, record_id,
  class, tagged_at, tagged_by)` — one row per tag (a record
  can have multiple)
- Memory writes accept an optional `class: DataClass` parameter
- Audit payloads with PII auto-redacted in
  `argo audit export --gdpr-redact-emails` (already exists)
- New CLI: `argo classify <kind>:<id> <class>`,
  `argo classify list [--class <c>] [--worker <id>]`,
  `argo classify rules` (print configured handling rules)
- Classification rules in workspace config:
  `[classification.pii] retention_days = 90, redact_in_export = true`

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies (DataClass enum)
- `crates/argo-identity/src/classification.rs` (NEW)
- `crates/argo-identity/src/lib.rs` — narrowly: re-export
- `crates/argo-store/migrations/<next>__data_classification.sql`
  (NEW)
- `crates/argo-store/src/classification.rs` (NEW)
- `crates/argo-memory/src/lib.rs` — narrowly: accept
  optional class param on writes
- `crates/argo-cli/src/handlers/classify_*.rs` (3 NEW)
- `crates/argo-cli/src/cli_classify_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: add
  `Classify` subcommand

## Acceptance criteria
- [ ] DataClass enum + tag table persisted
- [ ] Memory writes can carry classification
- [ ] Per-class retention rule honored (retention_days)
- [ ] PII tags redact emails in audit export
- [ ] `argo classify` CLI works for all 3 subcommands
- [ ] `--json` universal output
- [ ] 10 unit tests + 1 integration test (full tag→retention
      cycle)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-data-classification`
- PR title: `governance: data classification tags (PII/PHI/financial) (claude-data-classification)`
