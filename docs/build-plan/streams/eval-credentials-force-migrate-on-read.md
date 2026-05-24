# eval-credentials-force-migrate-on-read

**Owner**: claude
**Priority**: P2 - credentials at-rest hardening
**Authored against**: origin/main `ef55f5422` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-credentials-force-migrate-on-read`.

## Source

Evaluator round 9 cleared the original argo-credentials cross-tenant concern as
workspace-scoped by design, but left one P2 residue: legacy plaintext entries
are still accepted on read through `EncryptedEntry::Plain`. The crate already
has `migrate_plaintext_in_place`; the gap is making the safe path automatic.

## Goal

Make normal credential reads opportunistically migrate legacy plaintext entries
to encrypted entries while preserving the existing encrypted read/write path.

## May Touch

- `crates/argo-credentials/src/lib.rs`
- `crates/argo-credentials/src/lib_tests.rs`
- `crates/argo-credentials/src/age_wrap.rs` (only if needed for existing helper reuse)
- `crates/argo-credentials/src/master_key.rs` (only if needed for existing helper reuse)
- `docs/skycot/evals/scheduled/2026-05-24/eval-credentials-force-migrate-on-read.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change credential call sites in CLI, gateway, connectors, or config.
- Do not introduce SQL, tenant columns, keyring behavior, or network calls.
- Do not change the on-disk encrypted document schema except through the
  existing wrapped-entry format.

## Acceptance

- [ ] A normal read of a file containing plaintext legacy entries returns the same secret values and rewrites those entries encrypted.
- [ ] The rewritten file is written atomically and has `0600` permissions on Unix, matching the current migration helper contract.
- [ ] A normal read of an already encrypted file does not rewrite or corrupt it.
- [ ] Mixed plaintext/encrypted files preserve encrypted entries and encrypt only the plaintext entries.
- [ ] The report explains why workspace path remains the tenant boundary and lists any remaining caveats.
- [ ] `cargo test -p argo-credentials` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Keep this scoped to the crate-local legacy-accept path. Tenant-key separation and
connector cache isolation already have separate mission follow-ups.
