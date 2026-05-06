# V1.4 — Self-update CLI

**Owner**: codex. **Depends on**: claude-installer-macos
(in flight) merged.

## Scope

`argo self-update` checks GitHub Releases for a newer version,
downloads the appropriate binary, replaces the running one,
verifies signature, restarts the daemon if running.

- `argo self-update [--check] [--channel stable|beta]
  [--force]`
- `--check` reports the available version without installing
- Downloads from `https://github.com/mark-hallam/Argo-Agent/
  releases/latest` (or the GitHub API) the asset matching
  the current platform (e.g. `argo-darwin-aarch64.tar.gz`)
- Verifies signature against committed public key
  (cosign or minisign)
- Atomic replace: download to temp, verify, then `mv`
- If gateway daemon was running, gracefully restart it after
  swap

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/self_update.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register
  `SelfUpdate` subcommand
- `crates/argo-cli/Cargo.toml` — narrowly: add `self_update`
  crate dep (well-known Rust crate that handles the heavy
  lifting)
- `assets/release-pubkey.pem` (NEW — embedded via include_bytes!)
- `crates/argo-cli/tests/self_update_smoke.rs` (NEW — gated on
  `ARGO_SELF_UPDATE_INTEGRATION`)

## Acceptance criteria
- [ ] `--check` reports version without installing
- [ ] Real download + replace works on macOS + Linux
- [ ] Signature verification mandatory (no opt-out)
- [ ] Daemon restart works if running
- [ ] `--json` envelope reports version + signature status
- [ ] 5 unit tests + 1 integration test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-self-update-cli`
- PR title: `cli: argo self-update — verified binary upgrade (codex-self-update-cli)`
