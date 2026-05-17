# mission-sqlcipher-2-keyring-wire — OS keyring integration for SQLCipher encryption keys (v1.0 SHIP-GATE, PR 2 of 4)

**Owner**: claude. **Sizing hint**: cross-platform integration + tests.
**Depends on**: `mission-sqlcipher-1-libsqlite3-patch` MERGED (workspace can build with SQLCipher).
**Authored against**: origin/main `bc3841a4` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sqlcipher-2-keyring-wire`.
**Source**: 2026-05-18 audit Decision 4 — encryption-at-rest in v1.0. This is PR 2 of a 4-PR sequence.

## Mission alignment

- **Security** (CLAUDE.md priority #4) — encryption keys must NOT be plaintext on disk; OS keyring is the standard mechanism.
- **Commercial usability** (#5) — enterprise security reviews check for keyring-based key storage.

## Scope

P0 v1.0 SHIP-GATE. Land the cross-platform OS-keyring integration so SessionStore can derive its encryption key from a secret persisted in the OS-managed credential store. Does NOT flip the default yet.

### What lands here

1. New module `argo_config::cred::db_key`:
   - `pub async fn get_or_create_db_key(tenant_id: TenantId) -> ArgoResult<SecretString>`
   - On first call per tenant: generates 32 bytes via `OsRng`, stores in OS keyring under service `skycot.db.encryption`, account `tenant-<uuid>`.
   - On subsequent calls: retrieves from keyring.
2. Per-OS backend:
   - macOS: `security-framework` crate (Keychain).
   - Linux: `secret-service` crate (libsecret / GNOME Keyring / KWallet via D-Bus).
   - Windows: out of scope (Windows is DEFERRED-v1.x per operator decision).
3. Headless / CI fallback:
   - Env var `SKYCOT_DB_KEY_BASE64` overrides keyring lookup (for ephemeral Fly deploys + CI).
   - Document the priority: env > keyring > error.
4. Tests:
   - Unit test using an in-memory mock backend (so CI doesn't poke the real Keychain).
   - Integration test that round-trips a key through the env-var fallback.

### Choice of crate

- `keyring = "2"` — cross-platform, supports macOS + Linux + Windows. Single API.
- Alternative: `security-framework` (macOS) + `secret-service` (Linux) directly — finer control, more code.

Worker picks `keyring = "2"` unless it fails a smoke test; declare choice in PR body.

LOC: ~300.

## Files you may touch

- `crates/argo-config/Cargo.toml` — add `keyring = "2"` (or alternative). ≤4 modified LOC.
- `crates/argo-config/src/cred/db_key.rs` (NEW) — main module. ≤200 LOC.
- `crates/argo-config/src/cred/db_key_tests.rs` (NEW) — unit + integration tests. ≤80 LOC.
- `crates/argo-config/src/cred/mod.rs` — module declaration. ≤3 modified LOC.
- `docs/skycot/security/encryption-at-rest.md` — extend the existing doc (from #1820) with keyring backend description. ≤30 modified LOC.

## Files you must NOT touch

- `SessionStore::open` — still unencrypted-default; PR 4 wires this in.
- `argo-types::TenantId` — read-only consumer.
- Boot warning sites — PR 4 removes them.

## Acceptance criteria

- [ ] `argo_config::cred::db_key::get_or_create_db_key(t)` returns a 32-byte secret on macOS via Keychain.
- [ ] Same on Linux via D-Bus secret-service (worker can verify on a Linux container).
- [ ] Env-var fallback `SKYCOT_DB_KEY_BASE64` works (for Fly deploys without keyring).
- [ ] Unit tests pass without poking the real Keychain (mock backend).
- [ ] `cargo clippy -p argo-config --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-sqlcipher-2-keyring-wire`
- Title: `feat(config): OS keyring backend for SQLCipher encryption keys (mission-sqlcipher-2-keyring-wire) — PR 2/4`
- Worker verification: crate-scoped clippy + tests; manual macOS Keychain + Linux secret-service smoke.

## Hidden constraints

- **No keyring poke in tests.** Real Keychain access prompts for the user password; tests must use a mock backend.
- **Env-var fallback is mandatory for Fly.** Fly ephemeral containers have no persistent keyring; the operator must be able to ship a base64-encoded key via env.
- **Per-tenant keys, not global.** Multi-tenant deploys need per-tenant key isolation — share-key-across-tenants is a security regression.
- **PR 4 is the wire-in point.** This PR only provides the API.
