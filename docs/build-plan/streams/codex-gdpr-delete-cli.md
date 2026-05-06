# V2.1 — GDPR right-to-be-forgotten CLI

**Owner**: codex. **Depends on**: codex-compliance-export-cli
(in-flight) merged.

## Scope

GDPR "right to be forgotten" / "right to erasure" requires
the platform to permanently delete personal data on subject
request. This brief ships `argo gdpr delete` to walk the
data graph and purge:

- Memory rows whose `author` field matches the subject email
- Audit records' payloads where the subject email appears
  (replace with `redacted-<sha256-prefix>@example.com`;
  audit chain stays intact via redaction-not-deletion)
- Skill candidates whose `proposed_by` matches subject
- NotifyChannel registrations targeting the subject's email
- Worker `operator_email` references
- Session payloads where the subject is involved

Behaviors:
- `argo gdpr delete --subject <email> --dry-run` (default)
  prints the affected row count per table, exits 0
- `argo gdpr delete --subject <email> --confirm` requires
  exact-match retype of the subject email
- `argo gdpr delete --subject <email> --confirm
  --bundle-out <path>` writes a tarball of pre-deletion
  records (for compliance proof of deletion)
- Hash-chain audit records get an entry: `{kind:
  "gdpr_subject_erasure", subject_hash: <sha256>,
  affected_tables: [...], at: <ts>}` — the subject hash
  lets future audits prove deletion happened without
  re-revealing the email

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/gdpr_delete.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-cli/src/cli_gdpr_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly:
  register `Gdpr` subcommand
- `crates/argo-store/src/gdpr.rs` (NEW) — the per-table
  deletion + redaction helpers
- `crates/argo-store/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/gdpr_delete_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `--dry-run` is the default (must say `--confirm` to
      delete)
- [ ] `--confirm` requires exact-match email retype
- [ ] All 6 affected surfaces touched
- [ ] Audit chain stays intact (redactions, not deletions,
      for audit payloads)
- [ ] `gdpr_subject_erasure` audit row written
- [ ] Pre-deletion bundle (when requested) is a valid
      tarball
- [ ] `--json` follows universal envelope
- [ ] 8 unit tests + 1 CLI smoke (full delete flow)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-gdpr-delete-cli`
- PR title: `gdpr: argo gdpr delete — right-to-be-forgotten CLI (codex-gdpr-delete-cli)`
