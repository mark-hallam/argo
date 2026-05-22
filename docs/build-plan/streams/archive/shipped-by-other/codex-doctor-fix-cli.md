# V1.4 — Doctor auto-fix CLI

**Owner**: codex. **Depends on**: H-doctor-extensions (#808),
codex-doctor-watch-cli (in flight) merged.

## Scope

`argo doctor` reports issues. `argo doctor fix [--auto]
[--issue <id>]` actually applies fixes for the safe subset:

Safe auto-fixes (apply without prompt):
- Missing `~/.argo` config dir → create
- Missing per-tenant manifest dir → create
- Stale lock file > 5min old → remove
- Missing default workspace policy → write template

Confirmation auto-fixes (require `--auto` flag explicitly):
- Migrations behind → run pending migrations
- Vertical missing required fields → patch from registry

Unsafe (never auto-fix; print remediation only):
- Database corruption
- Missing API keys
- Workspace policy bare-mode enabled

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/doctor_fix.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-cli/src/cli_doctor_cmds.rs` — narrowly: add
  `Fix` subcommand variant
- `crates/argo-cli/tests/doctor_fix_smoke.rs` (NEW)

## Acceptance criteria
- [ ] All safe auto-fixes apply without prompt
- [ ] Confirmation fixes require `--auto`
- [ ] Unsafe issues print remediation, exit non-zero
- [ ] Per-fix audit row written
- [ ] `--issue <id>` runs only that fix
- [ ] `--json` envelope
- [ ] 6 unit tests (per category) + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-doctor-fix-cli`
- PR title: `cli: argo doctor fix — auto-remediation (codex-doctor-fix-cli)`
