# mission-sqlcipher-default-v1x — wire SQLCipher into the default build (post-v1)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high.
**Depends on**: codex-sqlcipher-default-or-document (MERGED).
**Authored against**: origin/main `HEAD` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sqlcipher-default-v1x`.
**Source**: follow-up to codex-sqlcipher-default-or-document. The
v1 brief took Path B (document plaintext-at-rest + boot warning +
`acknowledge_plaintext` knob); this v1.x brief flips the default to
encrypted-at-rest by wiring SQLCipher into the default build.

## Mission alignment

- **Security** (CLAUDE.md priority #4) — at-rest encryption is a
  hard requirement for SOC2 / HIPAA / PCI. Backup leak = no leak
  once page-level encryption is the default.
- **Commercial usability** (#5) — first enterprise design partner
  expects this on day 1; the v1 `acknowledge_plaintext` opt-out
  becomes the test/dev convenience knob, not the prod posture.

## Scope

**Pick this brief once:**
- v1 demo path is recorded (macOS gateway start, computer-use run loop).
- First enterprise design partner is engaged AND has confirmed
  page-level encryption is a procurement gate.
- The conductor has confirmed no other in-flight brief is currently
  modifying `argo-store`'s SQLite driver layer (high merge-conflict
  surface).

**Tasks:**

1. Switch `argo-store`'s SQLite driver from pure-Rust `sqlx` to a
   SQLCipher-linked variant. Two viable approaches:
   - **A (preferred):** add a `rusqlite` dep with
     `bundled-sqlcipher-vendored-openssl` feature alongside `sqlx`,
     and route the SessionStore connection pool through it. Keep
     `sqlx` for the FTS5 and audit-chain queries that already work.
   - **B (more invasive):** swap the entire store layer to
     `rusqlite`. Larger blast radius (~1500 LOC across `argo-store`
     query helpers), but a cleaner long-term posture.

   Pick A unless the operator says otherwise at claim time.

2. Default-on the `sqlcipher` Cargo feature for the `argo` binary
   target: `cargo build --release --bin argo` links SQLCipher
   without a feature flag.

3. Ensure `apply_encryption_pragma` actually encrypts under the
   default feature set. Verify with the existing integration test
   pattern: write a row containing a known marker string, close the
   pool, `grep -a "<marker>" db.sqlite` returns 0 matches.

4. Flip the `[security.encryption]` default to a generated-on-first-
   boot key (stored in the OS keyring via `argo-credentials`) and
   migrate any existing plaintext DBs in place via `argo db migrate-
   plaintext-to-encrypted`. The migration command must be opt-in
   and prompted for backup-first.

5. Remove the `acknowledge_plaintext` boot-warning logic (added by
   codex-sqlcipher-default-or-document); plaintext-at-rest is no
   longer the default behaviour, so the advisory is moot. Keep the
   `acknowledge_plaintext` config field as a backward-compat shim
   that maps to "explicitly disable encryption" for the (rare)
   operator who really wants plaintext.

6. Update `docs/skycot/security/at-rest-encryption.md` to reflect
   the new default. Move the "v1 limitation" framing to a
   historical-context section near the bottom.

7. Update `docs/skycot/runbook/at-rest-encryption.md` first-write
   procedure to match the new boot flow (key generated automatically
   unless `key_source` is explicitly set).

## Files this brief may touch

- `crates/argo-store/Cargo.toml` — add `rusqlite` with bundled-
  sqlcipher; flip `default = ["sqlcipher"]`. ≤15 modified LOC.
- `crates/argo-store/src/store_open.rs` — route `SessionStore::open`
  through the SQLCipher-linked driver. ≤80 modified LOC.
- `crates/argo-store/src/encryption.rs` — `apply_encryption_pragma`
  now load-bearing. ≤30 modified LOC.
- `crates/argo-store/src/encryption_tests.rs` — integration test:
  fresh DB write → close → plaintext grep returns 0. ≤80 LOC.
- `crates/argo-cli/src/handlers/db_rekey.rs` — verify rekey path
  still works under bundled SQLCipher. ≤20 modified LOC.
- `crates/argo-cli/src/handlers/db_migrate_plaintext.rs` (NEW) —
  one-shot migration command. ≤150 LOC.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — remove the
  acknowledge_plaintext warning. ≤20 modified LOC.
- `crates/argo-cli/src/handlers/doctor_storage.rs` — update
  `encryption_status` to report ON/OFF accurately. ≤30 modified LOC.
- `crates/argo-config/src/sections_security.rs` — change default
  `key_source` from `"none"` to `"keyring:argo-db"`. ≤10 modified LOC.
- `docs/skycot/security/at-rest-encryption.md` — rewrite for default-
  on posture. ≤100 modified LOC.
- `docs/skycot/runbook/at-rest-encryption.md` — first-write update.
  ≤50 modified LOC.

## Files this brief must NOT touch

- Existing audit-chain query helpers — driver swap should be
  transparent to all callers of `SessionStore::pool()`.
- Public APIs of `argo-store` — encryption is at the driver layer.
- Other crates' Cargo.toml — the bundled-SQLCipher dep stays scoped
  to `argo-store`.

## Acceptance criteria

- [ ] `cargo build --release --bin argo` links SQLCipher (verify
      via `otool -L target/release/argo | grep -i sqlcipher` on
      macOS OR equivalent ldd grep on Linux).
- [ ] Fresh DB write + close + plaintext grep returns 0 matches for
      a test marker string.
- [ ] `argo doctor` reports `encryption: ON (SQLCipher)`.
- [ ] `argo db rekey` works under the new driver (existing
      integration test passes).
- [ ] `argo db migrate-plaintext-to-encrypted` migrates a v1
      plaintext DB to an encrypted DB in place, with a backup-first
      prompt and atomic swap on success.
- [ ] Boot no longer prints the `acknowledge_plaintext` warning
      under default config.
- [ ] No regression in existing `argo-store` or `argo-cli` test
      suites.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `<owner>-mission-sqlcipher-default-v1x`
- Title: `feat(store): SQLCipher default-on + plaintext migration`
- Worker verification: per SWARM_PROTOCOL.md.

## Hidden constraints

- **`bundled-sqlcipher-vendored-openssl` pulls a C compiler dep +
  2-3 min cold build cost.** Acceptable post-v1; document the trade
  in the security doc.
- **Backup strategy changes with encryption ON.** Backups are
  useless without the key. The runbook documents key-backup-
  separation (1Password / Vault, NOT alongside the DB backup).
- **First-boot key generation** must be deterministic for the
  smoke test path; use a fixed dev key under
  `--profile dev` and a fresh random key under default profile.
- **`argo-credentials` keyring integration** is the recommended
  storage for the auto-generated key; falls back to a tmpfs file
  under `~/.argo/keys/db.key` on hosts without a keyring.
- **Closes** the v1.x portion of audit finding P0-3 (the v1 portion
  was closed by `codex-sqlcipher-default-or-document`).
