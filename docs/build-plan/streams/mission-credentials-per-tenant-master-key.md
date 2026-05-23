# mission-credentials-per-tenant-master-key

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-credentials-per-tenant-master-key`.

🔴 **P0 PROCESS-GLOBAL CREDENTIAL ENCRYPTION KEY.** Source: evaluator round-9 PR #2080 finding §3 P0.

> "`crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:180-183` — process-global `OnceLock<MasterKey>` with `MasterKey::generate()` ephemeral fallback. Data loss on restart + cross-tenant blast radius."

The credentials encryption-at-rest layer currently uses ONE process-global master key for ALL tenants. Two problems:
1. **Data loss**: if the `OnceLock` falls through to `generate()` (key not in env/keyring), the random ephemeral key is forgotten on restart → existing encrypted credentials become unreadable.
2. **Cross-tenant blast radius**: any leak of the master key compromises EVERY tenant's credentials at once.

## Scope

Replace process-global master key with per-tenant master keys derived from a workspace root key.

### Key hierarchy

```
workspace_root_key   (in env / Keychain / hardware module — single source)
  └─ HKDF derive  →  per_tenant_master_key(tenant_id)
                       └─ used to AEAD-encrypt that tenant's credentials
```

### Implementation

`crates/argo-credentials/src/master_key.rs` (or wherever the OnceLock lives — find first):

```rust
pub fn master_key_for_tenant(workspace_root: &WorkspaceRootKey, tenant_id: &TenantId) -> MasterKey {
    let mut output = [0u8; 32];
    hkdf::Hkdf::<sha2::Sha256>::new(Some(b"argo-cred-master-v1"), workspace_root.as_bytes())
        .expand(tenant_id.as_str().as_bytes(), &mut output)
        .expect("HKDF expand 32 bytes always fits");
    MasterKey::from_bytes(output)
}
```

Then every encrypt/decrypt callsite takes `tenant_id` and calls this helper. Per-tenant cache via `DashMap<TenantId, MasterKey>` to avoid HKDF cost per call.

### Migration

Existing credentials encrypted with the old process-global key must be re-encrypted under the per-tenant key. Three options:
- **Re-encrypt on read**: lazy migration. On first decrypt with old key, re-encrypt with new key + write back. Mark with a `key_version: 2` byte.
- **Operator-driven**: `argo creds rotate-master-keys` CLI command does the sweep. Document in operator runbook.
- **Hard cut**: backup + re-onboard. Brutal but simplest.

**Pick lazy migration** — it's the only operator-zero-effort path.

### Test

Sibling test: bootstrap tenant_a + tenant_b, write credentials in each, assert each tenant's master key is DIFFERENT, assert decrypt-with-tenant_b-key on tenant_a's payload FAILS (not silent-empty).

### Workspace root key

`workspace_root_key` lives where the SQLCipher key lives — env var `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` (with macOS Keychain fallback). Same `~/.argo/env.local` plumbing as the DB key.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-credentials/src/master_key.rs` (or wherever — find via `rg 'OnceLock<MasterKey>'`)
- `crates/argo-credentials/src/master_key_tests.rs` — sibling tests.
- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:180-183` — replace OnceLock usage.
- Per-tenant encrypt/decrypt callsites (find via `rg 'MasterKey|encrypt_credential|decrypt_credential' crates/`).
- `~/.argo/env.local.example` (or equivalent) — document the new env var.
- `docs/skycot/operator/credentials-config.md` — update for per-tenant master keys.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The SQLCipher key (separate concern).
- The AEAD cipher choice (use existing, just key it differently).

## Acceptance criteria

- [ ] Per-tenant master key derived via HKDF from workspace root + tenant_id.
- [ ] OnceLock removed; cache via DashMap.
- [ ] Lazy migration path: old `key_version=1` payloads re-encrypted on read with `key_version=2`.
- [ ] Cross-tenant negative test: tenant_a's master ≠ tenant_b's; decrypt cross-tenant fails.
- [ ] `cargo clippy -p argo-credentials -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Workspace clippy clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-credentials-per-tenant-master-key`
- PR title: `fix(credentials): per-tenant master keys via HKDF — no more process-global key (mission-credentials-per-tenant-master-key)`

## Notes for the implementer

- ~300-500 LOC expected. Key derivation ~50, per-tenant cache ~40, migration ~60, callsite sweep ~80, tests ~150, doc ~30.
- **HKDF with sha256** is the right primitive — `hkdf` crate is already a workspace dep (check first).
- **Per-tenant key cache** is mandatory — without it, every credential read pays HKDF cost (~50µs but adds up).
- **The lazy migration is load-bearing** — operators can't tolerate "all credentials lost on upgrade." Test the migration path explicitly.
- Cite round-9 evaluator + #2080 §3 P0 verbatim.
