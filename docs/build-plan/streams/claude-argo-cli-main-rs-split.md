# claude-argo-cli-main-rs-split — split 525-LOC `main.rs` dispatch into per-domain handlers

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `f252a46b` on `2026-05-14`.

> **URGENT split-prep — blocker for `claude-config-loader-migrate-callsites-v2` + `mission-operator-feedback-dreams-curation-v2`.** `crates/argo-cli/src/main.rs` is at 525 LOC (cluster-5 audit; over the 500 ceiling). The bulk is the giant `match Command::...` dispatch.

## Scope

Pure refactor. Extract dispatch arms into per-domain handler modules. Pattern: keep the top-level `Command` match in main.rs but each arm calls a `dispatch_<domain>(cmd)` async fn that lives in a sibling. The dispatch sub-fns return `ArgoResult<()>`.

Natural split (8 sub-dispatchers, ~30-60 LOC each):
- `main_dispatch_skill.rs` — Skill / Memory / Trust subcommands
- `main_dispatch_session.rs` — Session / Chat / Resume / Replay
- `main_dispatch_worker.rs` — Worker / Persona / Charter
- `main_dispatch_audit.rs` — Audit / Compliance / Forge
- `main_dispatch_gateway.rs` — Gateway / Health / Doctor
- `main_dispatch_setup.rs` — Setup / Onboard / Wizard / Provider
- `main_dispatch_vertical.rs` — Vertical / Genesis / Marketplace / Cron
- `main_dispatch_dev.rs` — TUI / Voice / ACP / DevTools

After split: `main.rs` ≤ 250 LOC (top-level match + arg parsing + global init).

## Files you may touch

- `crates/argo-cli/src/main.rs` — slim from 525 → ~250 LOC.
- 8 NEW `crates/argo-cli/src/main_dispatch_*.rs` siblings.
- `crates/argo-cli/src/lib.rs` — narrowly: declare the 8 new sibling modules.

## Files you must NOT touch

- Any `handlers/` file — locked.
- The `Command` enum or any `*Cmd` enum — locked.
- The dispatch semantics (everything still exits the same way).

## Acceptance

- [ ] `main.rs` ≤ 300 LOC (target ~250).
- [ ] 8 sibling files each ≤ 200 LOC.
- [ ] `cargo build --workspace` clean.
- [ ] Every `argo <subcommand>` produces byte-identical output.
- [ ] Workspace clippy clean.

## Branch
- `claude-argo-cli-main-rs-split`

## Notes
- `-split` suffix → ceiling-check auto-skips.
- The match-arm-to-sub-fn mechanical transform is well-precedented (see `boot.rs` split #1544).
