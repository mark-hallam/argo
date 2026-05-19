# Stream Audit-P1-2 — SQLCipher fail-closed in enterprise mode + release test

**Owner**: claude.
**Depends on**: `mission-audit-sqlcipher-prepared-stmt-fix` (the lint must be green before adding enterprise paths through the same file).
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-sqlcipher-enterprise-default`.
**Source**: 2026-05-19 codex audit P1-5 (sub-issue 2), validated by claude `VALIDATION.md` (CONFIRMED — `sections_security.rs:91` defaults `key_source = "none"`; `encryption.rs:13-19` documents that stock builds silently ignore `PRAGMA key`; SQLCipher is feature-gated under `bundled-sqlcipher`).

## Scope

A default Skycot install on stock SQLite (no `bundled-sqlcipher` feature) will silently ignore `PRAGMA key` and write plaintext database pages. `crates/argo-config/src/sections_security.rs:91` makes `key_source = "none"` the default. There is **no startup check** that an enterprise deployment is running against a SQLCipher-linked binary with a real key. A self-host deployment that thinks it's encrypted, isn't — until the operator notices an unencrypted page on disk during a breach.

This brief: (a) introduces an "enterprise mode" flag (`ARGO_GATEWAY_PROFILE=production` already exists per `mission-audit-bearer-fail-closed-prod` — reuse it, do not add a second flag); (b) when enterprise mode is on, fail-close startup unless both the `sqlcipher` Cargo feature is enabled at compile time AND `key_source != "none"`; (c) add a release-test that opens an encrypted on-disk DB under `bundled-sqlcipher` and confirms a plaintext-config deployment refuses to boot in enterprise mode.

This brief does NOT change the dev default (key_source=none stays for local dev). It hardens the production path only.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/encryption.rs` — add `pub fn sqlcipher_linked() -> bool` returning `cfg!(feature = "sqlcipher")`. Pure compile-time const.
- `crates/argo-store/src/store_open.rs` — at store-open entry (the function that the gateway calls at boot), call `enforce_enterprise_encryption(config)` which: if `is_production_mode() && (!sqlcipher_linked() || config.security.key_source == "none")` → return `Err(ArgoError::Config("enterprise mode requires SQLCipher-linked binary AND non-none key_source"))`. Only the production path fails; dev unchanged.
- `crates/argo-store/src/store_open_enterprise_tests.rs` (NEW) — three test cases: (1) dev mode + key_source=none → opens (back-compat); (2) production mode + key_source=none → returns `Err`; (3) production mode + key_source=env → opens (assuming env var is set). All run under the `sqlcipher` feature.
- `crates/argo-store/Cargo.toml` — under `[features]`, ensure `sqlcipher` is documented as the "enables SQLCipher PRAGMA enforcement" feature; `bundled-sqlcipher` is the full bundled build.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — propagate any `store_open` enterprise-encryption error as a fatal startup error (same pattern as `mission-audit-bearer-fail-closed-prod`'s bearer-token check). The two production-mode gates may share a `validate_production_config(&config) -> ArgoResult<()>` helper if convenient.
- `docs/skycot/operator/release.md` (NEW or extend) — add a "Enterprise build profile" section: (a) build with `--features bundled-sqlcipher`; (b) set `ARGO_GATEWAY_PROFILE=production`; (c) provide a key via `security.key_source = "env"` and `ARGO_DB_KEY=<hex>`. If the file doesn't exist, create it with this single section.

## Files you must NOT touch

- The dev default `key_source = "none"` — preserved for local dev.
- The SQLCipher PRAGMA call itself (`store_open.rs:114`) — owned by `mission-audit-sqlcipher-prepared-stmt-fix`.
- The schema or migrations — encryption is at the file level, not the row level. No table changes.
- Any caller of `store_open` (gateway boot is the only one that needs to surface the error).

## Acceptance criteria

- [ ] `sqlcipher_linked()` is a single source of truth for the compile-time feature flag.
- [ ] In production mode, `store_open` fails with a clear error if either: (a) `sqlcipher` feature is not enabled, OR (b) `key_source == "none"`. Error message includes both possible causes and the fix.
- [ ] In dev mode, all current behavior is preserved (no regression on existing tests).
- [ ] Three new tests in `store_open_enterprise_tests.rs` pass under `cargo test -p argo-store --features sqlcipher`.
- [ ] `docs/skycot/operator/release.md` documents the enterprise build flow (or `// DOC-MISSING:` marker added if the doc-tree owner prefers a separate brief).
- [ ] `cargo clippy --workspace --all-targets --features sqlcipher -- -D warnings` clean (note: feature-gated; default build is also clean).
- [ ] `cargo test --workspace --features sqlcipher` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-audit-sqlcipher-enterprise-default`
- PR title: `store: fail-closed encryption-at-rest in production mode (mission-audit-sqlcipher-enterprise-default)`

## Notes for the implementer

- The `validate_production_config` helper is the right shape: a single function that the boot path calls before listening starts. Multiple production-mode gates share the same wrapper; each gate is one `?` chain inside.
- The release test ideally runs on a CI runner with the `bundled-sqlcipher` feature compiled — but CI runners are blocked on billing per the audit. Add the test; document that CI verification waits on billing fix.
- The error message format: `"enterprise mode requires SQLCipher: rebuild with --features bundled-sqlcipher AND set security.key_source != \"none\" (got: feature_enabled={}, key_source={})"`. Show the operator both sides of the check.
- Do NOT add a "warn + continue" mode. Fail-closed is the whole point.

## Out of scope (follow-up briefs)

- Key rotation / re-keying flows (rotate `PRAGMA rekey` workflow) — separate brief once enterprise customers ask.
- Hardware-security-module (HSM) integration for key storage — v1.x.
- Per-tenant encryption keys (one DB key per tenant) — orthogonal architectural choice; not addressed here.
- Audit row on key access — audit chain hardening, separate brief.
