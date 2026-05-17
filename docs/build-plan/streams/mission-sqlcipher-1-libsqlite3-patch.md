# mission-sqlcipher-1-libsqlite3-patch — workspace libsqlite3-sys → SQLCipher build feature (v1.0 SHIP-GATE, PR 1 of 4)

**Owner**: codex. **Sizing hint**: dependency-config + verification.
**Depends on**: none.
**Authored against**: origin/main `bc3841a4` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sqlcipher-1-libsqlite3-patch`.
**Source**: 2026-05-18 audit Decision 4 — operator confirmed encryption-at-rest is v1.0 scope. This is PR 1 of a 4-PR sequence; the earlier monolithic brief `mission-sqlcipher-default-v1x` was BLOCKED as structurally infeasible. This sequence replaces it.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) + **Security** (#4) — encryption-at-rest is a v1.0 ship-gate per operator decision.
- **Commercial usability** (#5) — enterprise / HIPAA-adjacent buyers will refuse unencrypted SQLite at rest.

## Scope

P0 v1.0 SHIP-GATE. Land the workspace `libsqlite3-sys` configuration so SQLCipher is available as the underlying engine. Does NOT flip the default yet — PRs 2/3/4 follow.

### What lands here

1. Add `bundled-sqlcipher` feature to the workspace `libsqlite3-sys` patch. If `sqlx` is the consumer, patch its transitive dep.
2. Verify build on macOS (operator's demo Mac) AND Linux (Fly target).
3. Add `scripts/verify-sqlcipher-link.sh` that confirms the binary actually links against SQLCipher (not vanilla SQLite). Runs `argo --version` against a tiny encrypted DB and asserts `cipher_version()` PRAGMA returns non-empty.
4. NO behavior change — existing unencrypted SQLite paths remain functional.

### Build-config approach

Two routes — pick whichever passes verification:

- **Route A (preferred)**: cargo `[patch.crates-io]` for `libsqlite3-sys` to enable `bundled-sqlcipher` feature without forking. Less invasive; uses the upstream feature flag.
- **Route B (fallback)**: vendor `libsqlite3-sys` as `crates/argo-libsqlite3-sys-vendored/` with the feature enabled. More work but bulletproof against upstream feature drift.

Worker picks based on verification result; declare choice in PR body.

LOC: ~80 (Cargo.toml + script + verification test).

## Files you may touch

**REVISED 2026-05-18 by conductor (post-BLOCK).** Worker verified that Cargo ignores `features` in `[patch.crates-io]` entries, so Route A as originally scoped is structurally invalid. Expanded may-touch:

- `Cargo.toml` (workspace) — `[workspace.dependencies]` block for `libsqlite3-sys`, OR `[patch.crates-io]` if vendoring. ≤30 modified LOC.
- `crates/argo-store/Cargo.toml` — direct `libsqlite3-sys` dep with `bundled-sqlcipher` feature (Route A revised). ≤8 modified LOC.
- `crates/argo-store/build.rs` — ONLY if needed for cargo-feature wire-through. ≤20 modified LOC.
- `scripts/verify-sqlcipher-link.sh` (NEW) — verification script. ≤50 LOC.
- `crates/argo-store/tests/sqlcipher_link_verify.rs` (NEW) — minimal integration test that opens an encrypted DB. ≤40 LOC.
- `Cargo.lock` — regen after dep update. (auto-update, do not hand-edit)

## Files you must NOT touch

- `SessionStore::open` — default unencrypted, untouched until PR 4.
- Any production opening of SQLite — read-only consumer.
- The 4 boot-warning sites added by #1820 — they remain until PR 4 removes them.

## Acceptance criteria

- [ ] `cargo build -p argo-cli --release` succeeds on macOS.
- [ ] `cargo build -p argo-cli --release` succeeds on Linux (Fly Docker target — `docker run --rm -v $(pwd):/work -w /work rust:1.84 cargo build -p argo-cli` or equivalent).
- [ ] `bash scripts/verify-sqlcipher-link.sh` reports SQLCipher version present.
- [ ] `crates/argo-store/tests/sqlcipher_link_verify.rs` passes.
- [ ] No regression on existing unencrypted-DB tests (`cargo test -p argo-store`).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-sqlcipher-1-libsqlite3-patch`
- Title: `build(store): patch libsqlite3-sys → SQLCipher build feature (mission-sqlcipher-1-libsqlite3-patch) — PR 1/4`
- Worker verification: cross-platform build verification.

## Hidden constraints

- **sqlx + SQLCipher integration is the technical bet.** Verify that opening with `PRAGMA key = '...'` works through the sqlx connection pool, not just raw rusqlite.
- **Linux verification matters.** Fly deploys to Linux containers; macOS build success alone is not sufficient.
- **Don't flip the default here.** PR 4 owns that. This PR makes encryption AVAILABLE; existing paths remain unencrypted.
- **PR sequence**: 1 (this) → 2/3 in parallel (`-2-keyring-wire`, `-3-migrate-cli`) → 4 (`-4-default-flip`) last.
