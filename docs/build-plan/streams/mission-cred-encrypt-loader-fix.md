# mission-cred-encrypt-loader-fix — wire `cred::load` through `argo-credentials::decrypt_value`

**Owner**: claude. **Depends on**: `argo-credentials::{MasterKey, encrypt_value, decrypt_value}` (shipped); `argo-config::cred::load` (shipped, currently plaintext-only).
**Authored against**: origin/main `f3ca1cb4` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cred-encrypt-loader-fix`.

> 🚨 **SECURITY-URGENT — CEO-authorized 2026-05-14 after audit cluster 5 finding.** Today `argo cred encrypt-all` writes `[credentials.KEY] value = "<age-blob>"` table entries to `.argo/credentials.toml`. `argo-config::cred::load` deserializes the file into `BTreeMap<String, String>` — **any encrypted entry fails to parse**. **Any operator who runs `argo cred encrypt-all` bricks every cred read in production.** This brief lands the loader path so the two halves compose.

## Scope

1. **Extend `argo-config::cred::CredentialDocument`** to deserialize BOTH formats:
   - Plain: `"key" = "value"` (current behavior)
   - Encrypted table: `[credentials.key] value = "<age-blob>"`
   
   Use `serde::Deserialize` with `#[serde(untagged)]` on an internal `CredentialEntry { Plain(String), Encrypted { value: String } }` enum, then flatten at `load` boundary.

2. **`cred::get(name)` returns decrypted value transparently**:
   - If entry is `Plain(s)` → return `s` as-is.
   - If entry is `Encrypted { value }` → call `argo_credentials::decrypt_value(value, &master_key)` and return the result.
   - `MasterKey` resolution: same precedence as `encrypt_value` (env `ARGO_CREDENTIALS_MASTER_KEY` > keyring > file). Cached at `CredentialDocument::open` time so each `get` is cheap.

3. **Mixed-format file support**: a single `credentials.toml` may have both plaintext + encrypted entries (mid-migration state). `load` accepts both; `get` decrypts only the encrypted ones.

4. **Migration helper**: `argo-credentials::migrate_plaintext_in_place` already exists. Verify it produces the format `cred::load` now reads. If a mismatch exists, fix the producer side (in `argo-credentials`) to match the consumer side (in `argo-config`). The consumer side is canonical.

5. **Audit emission**: emit `AuditKind::HttpRequest` (or a new `CredentialsAccess` kind if appropriate) on every encrypted-entry decrypt, for tamper-evidence. Per-decrypt log is fine; this is NOT a hot path.

6. **Boundary check**: `argo cred encrypt-all` already exists in `handlers/credentials_encrypt_all.rs`. Verify it produces the format `cred::load` reads after this brief lands. End-to-end smoke: round-trip a key through encrypt-all → cred::get.

**Not in scope**: changing the at-rest encryption algorithm (age stays); UI for entering master key (operator sets env var or keyring entry manually); key rotation (separate brief `mission-credentials-master-key-rotation`).

## Files you may touch

- `crates/argo-config/src/cred.rs` (currently ~150 LOC) — narrowly: add the untagged-enum deserializer + decrypt path in `get`. ≤80 modified lines.
- `crates/argo-config/src/cred_tests.rs` (or sibling) — add 5 tests: plain-roundtrip, encrypted-roundtrip, mixed-file, master-key-missing, malformed-age-blob.
- `crates/argo-config/Cargo.toml` — narrowly: add `argo-credentials = { path = "../argo-credentials", version = "0.0.1" }` dep. Verify no dep cycle (`cargo metadata 2>&1 | grep cycle` BEFORE adding).
- `crates/argo-credentials/src/lib.rs` — IF `migrate_plaintext_in_place` produces a different format than `cred::load` now reads, fix the producer here. Narrowly: ≤30 modified lines.
- `crates/argo-cli/src/handlers/credentials_encrypt_all.rs` — narrowly: verify or align the on-disk format. ≤10 modified lines.
- `<NEW: crates/argo-config/tests/cred_encrypt_smoke.rs>` (≤150 LOC) — end-to-end smoke: write plain file, run encrypt-all, run cred::get, assert decrypted value matches original.

## Files you must NOT touch

- The age encryption primitives in `argo-credentials::encrypt_value` / `decrypt_value` — locked.
- `MasterKey` resolution order — locked.
- `BTreeMap` based `cred::get_with_backend` signature — preserve the public API (it returns `Option<String>`); internals only.

## Acceptance criteria

- [ ] `cred::load(path)` succeeds against a file with mixed plain + encrypted entries.
- [ ] `cred::get(name)` returns plaintext for plain entries; returns decrypted plaintext for encrypted entries.
- [ ] `MasterKey` resolution honors the existing env → keyring → file precedence; cached at `open` time.
- [ ] Missing master-key on an encrypted entry returns a typed error (e.g. `ArgoError::Provider("decrypt: master key not configured")` or a new `ArgoError::Credentials(String)` variant — pick one and document).
- [ ] Malformed age blob returns a typed error, NOT a panic.
- [ ] End-to-end smoke: write plain `credentials.toml` → `argo cred encrypt-all` → `cred::get("ANTHROPIC_API_KEY")` returns the original plaintext.
- [ ] **No regression for plaintext-only files** — the 95% of operators who never run encrypt-all see no behavior change.
- [ ] `cargo clippy -p argo-config -p argo-credentials --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-config -p argo-credentials` passes.
- [ ] No dep cycle.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-cred-encrypt-loader-fix`
- PR title: `config: cred::load decrypts encrypted entries via argo-credentials (mission-cred-encrypt-loader-fix)`

## Notes for the implementer

- This is a **security-urgency** brief. Land before any operator runs `argo cred encrypt-all` in production.
- The dep direction `argo-config → argo-credentials` is acceptable (config is a layer above credentials; the credential primitive doesn't depend on config). Verify with `cargo metadata` before adding.
- For the `MasterKey` cached at `open` time: in-memory only; do NOT persist anywhere.
- If the existing `encrypt-all` command produces a format that doesn't round-trip through this loader, **align the encrypter to the loader's expected format**, not the other way around. The loader is the canonical consumer.
