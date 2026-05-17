# mission-sqlcipher-4-default-flip — flip SessionStore default to encrypted + acceptance tests (v1.0 SHIP-GATE, PR 4 of 4)

**Owner**: claude. **Sizing hint**: SessionStore::open refactor + ~1430 callsite verification + boot-warning removal.
**Depends on**: PRs 1+2+3 (`mission-sqlcipher-1-libsqlite3-patch`, `-2-keyring-wire`, `-3-migrate-cli`) ALL MERGED.
**Authored against**: origin/main `bc3841a4` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sqlcipher-4-default-flip`.
**Source**: 2026-05-18 audit Decision 4 — encryption-at-rest in v1.0. This is the final PR of the 4-PR sequence; the load-bearing flip.

## Mission alignment

- **Security** (CLAUDE.md priority #4) — switches the default posture from "unencrypted-at-rest" to "encrypted-at-rest." THIS is the v1.0 commitment.
- **Reliability** (#1) — the ~1430 sqlx-callsite verification is the integration test that proves nothing regresses.

## Scope

P0 v1.0 SHIP-GATE. Flip `SessionStore::open` to use the keyring-derived key by default. Remove the "encryption-off" boot warning from #1820. Validate via comprehensive acceptance tests.

### What lands here

1. **`SessionStore::open` default flip**:
   - Reads encryption key via `argo_config::cred::db_key::get_or_create_db_key(tenant_id)`.
   - Sets `PRAGMA key = ?` on every new connection in the pool.
   - On failure to derive key: error out clearly (don't silently fall back to unencrypted).
2. **Boot-warning removal**:
   - Delete the warning emitted by the gateway boot path (added by #1820, search `unencrypted` / `cipher_version`).
   - Verify the docs reference at `docs/skycot/security/encryption-at-rest.md` is updated to reflect the new default.
3. **Backward-compat for unencrypted DBs**:
   - If `SessionStore::open` detects an unencrypted DB (cipher_version PRAGMA empty), emit a clear error:  
     `"Database at <path> is not encrypted. Run 'argo migrate encrypt --db <path>' to upgrade."`
   - Do NOT auto-migrate (data integrity > convenience).
4. **Acceptance tests**:
   - `crates/argo-store/tests/encrypted_default_smoke.rs` (NEW): create fresh DB → write → close → reopen → read → assert encrypted.
   - `crates/argo-cli/tests/encrypted_session_e2e.rs` (NEW): full gateway-up → POST signup → POST verify → assert magic-link tokens row is in an encrypted DB (verify via `sqlite3 --pragma cipher_version` outside the binary).
   - Cross-tenant encryption isolation test: tenant A and tenant B have DIFFERENT keys; opening tenant B's DB with tenant A's key fails.
5. **Boot smoke**:
   - `bash scripts/smoke-skycot.sh` must still pass end-to-end.

### The ~1430 callsite verification

Most sqlx callsites pass through `SessionStore` and are transparent to the cipher mode. The ones that need explicit attention:

- Direct `sqlx::SqlitePool::connect_with(...)` callers — bypass `SessionStore`. Grep `SqlitePool::connect` in `crates/`. Each must either route through `SessionStore::open` OR explicitly opt into unencrypted (test-only).
- Migration runner — must run AFTER `PRAGMA key`, not before.
- The `argo migrate encrypt` CLI from PR 3 — it deliberately opens unencrypted DBs; must use a separate code path that bypasses the default-encrypted opener.

LOC: ~1100 (most is the wire-in + test coverage; pure code change is ~200 LOC).

## Files you may touch

- `crates/argo-store/src/session_store.rs` — `open` flip. ≤80 modified LOC.
- `crates/argo-store/src/store_open.rs` — apply `PRAGMA key`. ≤40 modified LOC.
- `crates/argo-store/src/multi_tenant_pool.rs` — per-tenant key derivation. ≤60 modified LOC.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — remove unencrypted-warning. ≤20 modified LOC.
- `crates/argo-cli/src/handlers/migrate_encrypt.rs` — ensure it uses the explicit-unencrypted opener (not the default). ≤10 modified LOC.
- `crates/argo-store/tests/encrypted_default_smoke.rs` (NEW). ≤120 LOC.
- `crates/argo-cli/tests/encrypted_session_e2e.rs` (NEW). ≤180 LOC.
- `crates/argo-store/tests/cross_tenant_encryption_isolation.rs` (NEW). ≤100 LOC.
- `docs/skycot/security/encryption-at-rest.md` — update default-status. ≤30 modified LOC.
- Other sqlx callsite touches discovered by grep — variable.

## Files you must NOT touch

- `argo_config::cred::db_key` (from PR 2) — read-only consumer.
- `argo migrate encrypt` CLI (from PR 3) — read-only consumer.
- `libsqlite3-sys` patch (from PR 1) — read-only consumer.
- Test fixtures that intentionally use plaintext DBs — they must explicitly opt out via a `SessionStore::open_unencrypted_for_tests` helper.

## Acceptance criteria

- [ ] Fresh `argo gateway serve` writes an encrypted DB (verify via `sqlite3 ... 'PRAGMA cipher_version;'` returning non-empty).
- [ ] `cargo test --workspace` clean (no regression in the 1430-callsite zone).
- [ ] `bash scripts/smoke-skycot.sh` clean (all local + deployed scenarios).
- [ ] Opening a tenant A DB with tenant B's key fails with a clear error.
- [ ] The "encryption off" boot warning is gone.
- [ ] `docs/skycot/security/encryption-at-rest.md` reflects "encrypted by default."
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (or only known pre-existing lints declared as boundary notes).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-sqlcipher-4-default-flip`
- Title: `feat(store): flip SessionStore default to SQLCipher-encrypted (mission-sqlcipher-4-default-flip) — PR 4/4 — v1.0 SHIP-GATE`
- Worker verification: workspace clippy + tests + smoke + cross-tenant isolation test.

## Hidden constraints

- **Existing unencrypted DBs must fail-loud, not silent-fallback.** A user who upgrades their `argo` binary without running the migrate CLI will get a clear error and a one-line fix instruction. Silent fallback is a security regression.
- **Per-tenant keys are non-negotiable.** Shared-key-across-tenants is a security regression.
- **Test fixtures need an explicit unencrypted opener.** Don't make every test acquire a keyring secret; provide `SessionStore::open_unencrypted_for_tests` and use it in test setup.
- **Don't break the migrate CLI from PR 3.** It must still be able to open unencrypted source DBs.
- **Demo-record path stays GREEN.** `scripts/smoke-skycot.sh` must pass end-to-end; if it currently uses a hardcoded test DB, that DB needs to be encrypted-ready or use the unencrypted-test-opener.
- **Closes the encryption-at-rest v1.0 gate.** When this PR merges + smoke passes, the v1.0 launch is unblocked on encryption posture.
