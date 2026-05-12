# Mission load-bearing wire — Automated key rotation tooling

**Owner**: claude. **Depends on**: encryption-at-rest briefs (sqlcipher + credentials.toml) for the keys-being-rotated.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #217. Manual key rotation is error-prone (rotate one, forget the other; rotate prod but not staging). Need a single `argo security rotate-keys` flow that walks the operator through every key on the system with a dry-run + commit pattern.

## Scope

NEW `argo security rotate-keys` CLI surface with subcommands:
- `list` — enumerate all named keys + their last_rotated date + age in days
- `plan <key-name>` — show the rotation plan: who depends on this key, what will need re-encryption / re-keying
- `commit <key-name> [--new-key-source <src>]` — execute the plan idempotently
- `audit-key-age` — flag keys older than the operator's rotation-policy (default 90 days; configurable)

Key categories covered:
1. `ARGO_DB_ENCRYPTION_KEY` (from SQLCipher brief) — rotate via `argo db rekey`
2. `ARGO_CREDENTIALS_MASTER_KEY` (from credentials.toml brief) — rotate via `argo credentials encrypt-all --new-master-key`
3. `ARGO_GATEWAY_BEARER_TOKEN` — rotate via config edit + restart
4. `ARGO_CHANNEL_HMAC_SECRET` (voice DTMF) — rotate via config edit + restart
5. Per-connector OAuth refresh tokens — rotate via connector-specific `argo connector reauth <name>`

This is NOT in scope: actual cryptographic key generation (uses OS RNG via `rand` crate); HSM integration; key escrow; cross-process synchronization (single-tenant deployments only in v1).

## Files you may touch

- `crates/argo-cli/src/handlers/security_rotate_keys.rs` (NEW, ≤ 400 LOC)
- `crates/argo-cli/src/handlers/security_rotate_keys_tests.rs` (NEW, sibling, ≤ 300 LOC) — 7 tests
- `crates/argo-cli/src/handlers/security_rotate_keys_registry.rs` (NEW, ≤ 200 LOC) — pluggable key-registry mapping
- `crates/argo-cli/src/cli_security_cmds.rs` — `RotateKeys { ... }` variant
- `docs/skycot/RUNBOOK.md` — append "Key rotation" section

## Files you must NOT touch

- Existing connector credential paths
- Existing `argo db rekey` / `argo credentials encrypt-all` (this brief calls them; doesn't reimplement them)

## Acceptance criteria

- [ ] `argo security rotate-keys list` enumerates 5+ known key categories with last-rotated dates
- [ ] `plan <name>` shows a dry-run impact summary
- [ ] `commit <name>` is idempotent — re-running with same args is a no-op
- [ ] `audit-key-age --threshold 90d` exits non-zero if any key is older
- [ ] All operations write audit rows (kind: `security.key_rotated`)
- [ ] No keys logged in cleartext; redacted Debug impls
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-automated-key-rotation`
- PR title: `security: key-rotation CLI (mission-automated-key-rotation)`

## Notes

- The CLI is a coordinator that calls existing subsystem-specific rotation commands. Don't duplicate the rotation logic.
- `audit-key-age` exit codes: 0 = all keys fresh, 1 = at least one stale, 2 = config error. Designed for cron-driven monitoring.
- Document the operator workflow in RUNBOOK: weekly `audit-key-age`, monthly `plan` for any stale, then `commit` during a maintenance window.
