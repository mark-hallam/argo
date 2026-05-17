# codex-sqlcipher-default-or-document — wire SQLCipher default OR document plaintext-at-rest (P0 audit-finding)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high — operator-decision-shaped (two paths).
**Depends on**: none.
**Authored against**: origin/main `fa849bbb` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-sqlcipher-default-or-document`.
**Source**: 2026-05-17 claude conductor audit finding P0-3. Default cargo builds do not link SQLCipher; `apply_encryption_pragma` silently no-ops; `~/.argo/db.sqlite` writes plaintext pages.

## Mission alignment

- **Security** (CLAUDE.md priority #4) — at-rest encryption is a hard requirement for SOC2 / HIPAA / PCI. Without it, any backup leak = full data leak.
- **Commercial usability** (#5) — enterprise procurement asks for this on day 1.

## Scope

P0. **Pick ONE path** at claim time (worker decides based on operator's enterprise timeline; document choice in PR body):

### Path A — Wire SQLCipher as the default for `cargo build --release --bin argo`

1. Add `sqlcipher` to `[features]` default in `crates/argo-store/Cargo.toml`.
2. Switch the rusqlite dep to `bundled-sqlcipher` or equivalent SQLCipher-linked variant.
3. Ensure `PRAGMA key` actually encrypts under the default feature set.
4. Provide a key-resolution chain: `[security.encryption.key_source]` config block already exists — verify it covers `env_var` / `file` / `keychain` / `vault`.
5. Add integration test: spin up a DB, write a row, close, verify file is NOT plaintext-greppable for the row content (`grep -a "<test-row-marker>" db.sqlite` returns no matches).
6. Operator runbook: `docs/skycot/security/at-rest-encryption.md` covers key generation, rotation, backup-with-key-separation.
7. Document the FIPS / commercial-SQLCipher path as separate cargo features that override the default.

**LOC if Path A:** ~400.

### Path B — Document plaintext-at-rest as the v1 limitation; defer SQLCipher to v1.x

1. Update `docs/skycot/security/at-rest-encryption.md` (NEW) to explicitly state: "v1 default build writes plaintext pages. SQLCipher is operator-opt-in via `cargo build --features sqlcipher`. Enterprise / regulated tenants must request a SQLCipher-linked build OR use full-disk encryption (FileVault on macOS) as a compensating control."
2. Add a startup warning at `argo gateway start` boot: "⚠️ at-rest encryption: OFF (compile with --features sqlcipher to enable)" — visible to the operator on every boot until they explicitly suppress via `[security.encryption.acknowledge_plaintext] = true`.
3. Add a `argo doctor` check that reports SQLCipher status (already half-implemented per #1804 doctor probes — extend).
4. Add `mission-sqlcipher-default-v1x.md` follow-up brief authored at PR-merge time for the eventual default-on flip.

**LOC if Path B:** ~150.

### How to decide

Path A is strictly better if SOC2 / first-enterprise-customer is imminent (this month). Path B is the right call if v1 demo + first-3-design-partners are the immediate target and enterprise is v1.x.

Per the audit's bottom-line + operator-locked strategy (enterprise focus, not consumer), Path A is the more aligned choice. **Default recommendation: Path A** unless operator explicitly defers.

## Files you may touch (varies by path)

**Path A:**
- `crates/argo-store/Cargo.toml` — add `sqlcipher` to default features. ≤5 modified LOC.
- `crates/argo-store/src/encryption.rs` — verify `apply_encryption_pragma` actually encrypts now. ≤40 modified LOC.
- `crates/argo-store/src/encryption_tests.rs` — integration test for plaintext-grep-fails. ≤60 LOC.
- `crates/argo-config/src/sections_security.rs` — verify key-resolution chain. ≤30 modified LOC.
- `docs/skycot/security/at-rest-encryption.md` (NEW) — operator runbook. ≤100 LOC.

**Path B:**
- `crates/argo-cli/src/handlers/gateway/boot_router.rs` (or boot.rs) — startup banner. ≤30 modified LOC.
- `crates/argo-cli/src/handlers/doctor_storage.rs` (existing per #1804) — add SQLCipher status. ≤20 modified LOC.
- `crates/argo-config/src/sections_security.rs` — add `acknowledge_plaintext` knob. ≤15 modified LOC.
- `docs/skycot/security/at-rest-encryption.md` (NEW). ≤80 LOC.
- `docs/build-plan/streams/mission-sqlcipher-default-v1x.md` (NEW follow-up brief). ≤120 LOC.

## Files you must NOT touch

- Existing SQLCipher integration framework (`apply_encryption_pragma` API) — read-only consumer.
- Audit chain or any other store-consuming crate — encryption is at the SQLite-driver layer.
- Public APIs of `argo-store` — encryption is transparent to consumers.

## Acceptance criteria (Path A)

- [ ] Default cargo build links SQLCipher (verify via `cargo build --release && otool -L target/release/argo | grep -i sqlcipher` on macOS OR equivalent).
- [ ] Fresh DB write + close + plaintext grep returns 0 matches for a test marker string.
- [ ] `argo doctor` reports `encryption: ON (SQLCipher)`.
- [ ] Operator runbook documents key rotation procedure.
- [ ] No regression in existing test suite.
- [ ] BUILD_STATUS.md row updated.

## Acceptance criteria (Path B)

- [ ] `argo gateway start` boot banner shows the plaintext warning (suppressed via config).
- [ ] `argo doctor` reports `encryption: OFF (plaintext)` clearly.
- [ ] Operator runbook documents the v1 limitation + the FileVault compensating control + the v1.x roadmap entry.
- [ ] Follow-up brief `mission-sqlcipher-default-v1x.md` is authored + in BUILD_STATUS.md as TODO.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-sqlcipher-default-or-document`
- Title: depends on path — `feat(store): SQLCipher default-on + key resolution` OR `docs: document plaintext-at-rest v1 limitation + boot warning`
- Worker verification: per SWARM_PROTOCOL.md.

## Hidden constraints

- **`bundled-sqlcipher` may pull a C compiler dep + 2-3 min cold build cost.** Acceptable for v1; document the trade.
- **Backup strategy changes with encryption ON.** Backups become useless without the key. Runbook MUST document key-backup-separation (store key in 1Password / passbook / equivalent, NOT alongside the DB backup).
- **Key rotation** — SQLCipher has `PRAGMA rekey`; runbook covers when to rotate (suspected key exposure, after offboarding an admin, etc.).
- **FIPS / Commercial SQLCipher** — keep the existing `[security.encryption] key_source` config block compatible; downstream FIPS builds flip a different cargo feature.
- **Closes audit finding P0-3** when shipped (either path).
