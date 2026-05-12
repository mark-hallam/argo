# Mission load-bearing wire — system-keyring backend for argo credentials

**Owner**: claude. **Depends on**: existing `crates/argo-config/src/cred.rs` (file-backed credentials store at `<workspace>/.argo/credentials.toml`).
**Authored against**: origin/main `84e47150` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-system-keyring-integration`.

> SOURCE: MISSION_BACKLOG.md Section 7 Claim 7 #216 ("System keyring integration"). **Tier S (security-infra).** Today: credentials live in `<workspace>/.argo/credentials.toml` (mode 0600 on Unix). For local development this is fine; for operators in regulated environments or shared workstations, plaintext-on-disk is a non-starter. This brief adds an OS-keyring backend (macOS Keychain / Linux Secret Service / Windows Credential Manager) behind a config flag, plus a migration CLI to move existing file credentials into the keyring.

## Scope

EXTEND `crates/argo-config/src/cred.rs` (≤ 100 LOC delta — surgical, not a rewrite):

- Add `#[derive(Debug, Clone, Copy)] pub enum CredentialBackend { File, Keyring, Auto }`. `Auto` = keyring if available + initialized for the workspace, else file (back-compat default).
- Add `pub fn get_with_backend(workspace: &Path, key: &str, backend: CredentialBackend) -> Result<String, VarError>` — dispatches to the matching backend. Env-var precedence is unchanged: if the key is in the process env, that wins regardless of backend.
- `get()` and `get_from_workspace()` keep their existing signatures + read backend from `<workspace>/.argo/argo.toml` `[credentials] backend = "..."` (default `auto`). Backward-compatible.

NEW `crates/argo-config/src/cred_keyring.rs` (NEW, ≤ 220 LOC):

- Depends on the [`keyring`](https://crates.io/crates/keyring) crate v3.x (cross-platform: macOS Keychain, Linux Secret Service, Windows Credential Manager).
- Each Argo workspace gets its own keyring **service** name `argo:<workspace_root_hash>` (8-byte BLAKE3 of the workspace path; avoids collisions when an operator has multiple Argo workspaces on one machine).
- Within a service, each credential is keyed by its env-var name (e.g. `OPENAI_API_KEY`).
- `pub fn keyring_get(workspace: &Path, key: &str) -> Result<String, KeyringError>` — read from OS keyring.
- `pub fn keyring_set(workspace: &Path, key: &str, value: &str) -> Result<(), KeyringError>` — write to OS keyring.
- `pub fn keyring_delete(workspace: &Path, key: &str) -> Result<(), KeyringError>` — remove from OS keyring.
- `pub fn keyring_list(workspace: &Path) -> Result<Vec<String>, KeyringError>` — list keys stored under the workspace's service (best-effort; some OS backends don't enumerate).
- `pub fn keyring_available() -> bool` — quick liveness probe (attempts a no-op set/get/delete on a probe key under service `argo:probe`).

NEW `crates/argo-config/src/cred_keyring_tests.rs` (NEW, ≤ 140 LOC, included via `#[path]`) — 6 unit tests using the `keyring` crate's `mock` provider (configured via `keyring::set_default_credential_builder`): keyring-set-then-get-roundtrip, keyring-delete-removes-key, workspace-isolation (same key under two workspaces is two distinct entries), get_with_backend-Auto-falls-back-to-file-when-keyring-empty, env-var-still-wins-even-with-keyring-backend, keyring-available-returns-true-with-mock-installed.

### CLI

EXTEND `crates/argo-cli/src/cli_cred_cmds.rs` (≤ 30 LOC delta) — new `CredCmd::MigrateToKeyring` and `CredCmd::Backend { backend: String }` (sets `[credentials] backend = "..."` in `argo.toml`).

NEW `crates/argo-cli/src/handlers/cred_migrate_to_keyring.rs` (NEW, ≤ 110 LOC):

- Reads all credentials from `<workspace>/.argo/credentials.toml`.
- For each entry: writes to OS keyring; on success, removes the entry from the file. On any failure, leaves the file untouched (atomic: either all migrate or none).
- Prints per-key status (`migrated` / `failed: <reason>`).
- After successful migration: prompts whether to delete the (now-empty) `credentials.toml`; default yes.
- Flag `--dry-run` lists what would be migrated without writing.
- Flag `--from-file <path>` to migrate a non-default credentials file.

NEW `crates/argo-cli/tests/cred_keyring_smoke.rs` (NEW, ≤ 130 LOC) — 2 integration tests using `keyring`'s mock backend: `cred migrate-to-keyring --dry-run` lists entries without touching them; `cred migrate-to-keyring` then `cred get <KEY>` round-trips through keyring.

### Config

EXTEND `crates/argo-config/src/lib.rs` (≤ 15 LOC delta) — parse `[credentials] backend = "file" | "keyring" | "auto"` from `argo.toml`. Default: `auto`.

NO migration (the only persistent state is OS keyring entries; SQLite schema unchanged).

This is NOT in scope: encryption-at-rest for the credentials.toml file itself (separate brief #214 — `mission-credentials-file-encryption` if operators ask); HSM-backed credentials (out-of-scope, BYOK brief covers that); credential rotation automation (separate brief #217); SAM / Vault / AWS Secrets Manager integration (out-of-scope; operator can write a custom backend later); per-tenant keyring isolation in multi-tenant deployment (the keyring is per-workspace; multi-tenant separation comes from filesystem boundaries, not keyring boundaries).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-config/src/cred.rs` (EXTEND, ≤ 100 LOC delta)
- `crates/argo-config/src/cred_keyring.rs` (NEW, ≤ 220 LOC)
- `crates/argo-config/src/cred_keyring_tests.rs` (NEW, ≤ 140 LOC)
- `crates/argo-config/src/lib.rs` (EXTEND, ≤ 15 LOC delta — re-export + config parse)
- `crates/argo-config/Cargo.toml` (EXTEND, ≤ 5 LOC delta — add `keyring = "3"` + `blake3` dev-dep already present? verify)
- `crates/argo-cli/src/cli_cred_cmds.rs` (EXTEND, ≤ 30 LOC delta)
- `crates/argo-cli/src/handlers/cred_migrate_to_keyring.rs` (NEW, ≤ 110 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (EXTEND, ≤ 3 LOC)
- `crates/argo-cli/tests/cred_keyring_smoke.rs` (NEW, ≤ 130 LOC)
- `docs/skycot/runbook/credentials-keyring.md` (NEW, ≤ 150 LOC) — operator runbook

## Files you must NOT touch

- `crates/argo-store/**` (no migration)
- `crates/argo-runtime/**` (orthogonal — runtime reads via `cred::get`)
- Any vertical manifest

## Acceptance criteria

- [ ] `cargo build -p argo-config -p argo-cli` succeeds
- [ ] `cargo test -p argo-config cred_keyring` passes all 6 unit tests
- [ ] `cargo test -p argo-cli --test cred_keyring_smoke` passes both integration tests
- [ ] `argo cred backend keyring` sets `[credentials] backend = "keyring"` in `argo.toml`
- [ ] `argo cred migrate-to-keyring --dry-run` lists entries without modifying state
- [ ] `argo cred migrate-to-keyring` round-trips all keys through OS keyring; on failure, leaves file untouched
- [ ] Env-var precedence preserved: `OPENAI_API_KEY` set in process env always wins, regardless of backend
- [ ] `cred::keyring_available()` returns `true` on macOS / Linux dev machines (smoke verifies via mock)
- [ ] Multi-workspace isolation: same key `OPENAI_API_KEY` under workspace A and workspace B are two distinct keyring entries
- [ ] Runbook walks through migrate → backend-flag → verify
- [ ] All new files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-system-keyring-integration`
- PR title: `argo-config: OS keyring backend for credentials (macOS Keychain / Linux Secret Service / Windows Credential Manager) + migrate-to-keyring CLI (mission-system-keyring-integration)`

## Notes for the implementer

- The `keyring` v3 crate is the canonical cross-platform abstraction. Avoid platform-specific code paths in the connector; let `keyring` handle the OS-specific dispatch.
- Service-name hashing: use BLAKE3 of the workspace canonical path (resolve symlinks). 8-byte hex prefix is enough collision-avoidance for any realistic operator workstation. Don't use the workspace name (operators have `~/code/argo` and `~/Desktop/argo` and both should isolate).
- Auto-backend logic: `keyring_available()` should attempt a no-op probe (set / get / delete on key `argo:probe:liveness`). If any step errors, treat keyring as unavailable + fall back to file. Cache the probe result for the process lifetime.
- DO NOT crash on keyring write failure during normal `get`. The keyring backend is read-only in the hot path; writes happen only during `cred set` and `cred migrate-to-keyring`.
- DO NOT log credential values, even at `trace!` level. Log the key + the action (`keyring_get key=OPENAI_API_KEY backend=keyring`), never the value.
- The migration tool's atomicity is critical: if 5/7 keys migrate and the 6th fails, the file should still have all 7. Use a temp-file dance: read all → write all to keyring → on full success, rewrite file with only the failed entries (or delete it if all succeeded).
- Some Linux distros run without an active Secret Service backend (e.g. minimal containers, headless CI). `keyring_available()` returns `false` cleanly; the operator gets a clear message via `argo cred backend keyring`: "Keyring backend not available on this system. Install gnome-keyring or KWallet."
- The runbook should include: how to verify keyring entries exist (`security find-generic-password -s argo:<hash>` on macOS; `secret-tool lookup argo:<hash> OPENAI_API_KEY` on Linux); how to manually delete + re-add an entry; how to migrate BACK to file mode if needed.
- DO NOT auto-migrate on startup. Migration is an explicit operator decision via `argo cred migrate-to-keyring`. The default `auto` backend reads from keyring **only if entries exist there**; otherwise it falls through to file. This makes the upgrade path safe: install new Argo → run migrate command when ready.
- Coordinate w/ any in-flight credential briefs: this brief does NOT change the `credentials.toml` format, does NOT change `cred::set` callers, does NOT change the env-var precedence rule. Surface is purely additive.
