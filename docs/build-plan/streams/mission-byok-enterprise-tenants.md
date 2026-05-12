# Mission load-bearing wire — BYOK (Bring Your Own Key) for enterprise tenants

**Owner**: claude. **Depends on**: `mission-encryption-at-rest-sqlcipher` MERGED.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #230. Enterprise procurement teams want their KMS-managed key (AWS KMS / GCP CMEK / Azure Key Vault) to control SKYCOT's at-rest encryption, NOT a key the operator pastes into a config file.

## Scope

Extend the SQLCipher key-source enum with cloud-KMS variants:

1. `KeySource::AwsKms { key_arn, region }` — call STS to assume role + `kms:Decrypt` the wrapped DEK
2. `KeySource::GcpCmek { project, key_ring, key_name }` — gRPC to Cloud KMS
3. `KeySource::AzureKeyVault { vault_url, key_name }` — Azure SDK call
4. **DEK envelope encryption** — workspace stores a wrapped data-encryption-key (DEK); SKYCOT calls KMS to unwrap at startup; the unwrapped DEK is what feeds `PRAGMA key`. KMS never sees the data.
5. **Re-key flow**: `argo db rekey --to <new-kms-key>` re-wraps the DEK under the new KMS key (DEK unchanged → no data re-encryption needed → fast)

This is NOT in scope: HSM-direct integration (KMS is the abstraction); per-row encryption (page-level via SQLCipher); HashiCorp Vault (separate brief — same pattern); IAM role-assumption tutorials (operator-side).

## Files you may touch

- `crates/argo-store/src/encryption.rs` (≤ 480 LOC after — currently ≤200 from sqlcipher brief) — KeySource enum extended; cloud variants
- `crates/argo-store/src/kms/aws.rs` (NEW, ≤ 250 LOC) — STS + KMS Decrypt
- `crates/argo-store/src/kms/gcp.rs` (NEW, ≤ 250 LOC) — gRPC client
- `crates/argo-store/src/kms/azure.rs` (NEW, ≤ 250 LOC) — Azure SDK
- `crates/argo-store/src/kms/mod.rs` (NEW, ≤ 40 LOC) — trait + dispatch
- `crates/argo-store/src/kms_tests.rs` (NEW, sibling, ≤ 300 LOC) — wiremock-based per-provider happy + error paths
- `argo-config/src/sections.rs` — extend `[security.encryption.key_source]` parse
- `docs/skycot/RUNBOOK.md` — append "BYOK setup" with per-cloud snippets

## Files you must NOT touch

- The SQLCipher `PRAGMA key` wire-up itself (sibling brief owns); the audit chain; existing tests

## Acceptance criteria

- [ ] 3 cloud KMS variants compile against current SDK majors
- [ ] DEK envelope: KMS receives only the wrapped DEK, never the data
- [ ] `argo db rekey --to <new-kms>` re-wraps DEK without re-encrypting data
- [ ] KMS errors (network / IAM denied / key not found) yield typed `ArgoError::Provider` with operator-actionable messages
- [ ] All KMS calls covered by wiremock unit tests (no real-cloud calls in CI)
- [ ] Operator-runbook covers IAM policy snippets per cloud
- [ ] `cargo clippy -p argo-store --features sqlcipher,kms-aws,kms-gcp,kms-azure --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-byok-enterprise-tenants`
- PR title: `store: BYOK via cloud KMS (mission-byok-enterprise-tenants)`

## Notes

- DEK envelope encryption is the load-bearing safety: a stolen DB without KMS access is unreadable. Document the threat model explicitly.
- Each cloud variant behind its own Cargo feature so default builds don't pull all 3 SDKs.
- Re-key path keeps the DEK constant; only the WRAPPING changes. That's why a rotate is fast — no data re-encryption.
