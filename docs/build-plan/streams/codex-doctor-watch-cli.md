# V1.4 — Continuous doctor watch CLI

**Owner**: codex. **Depends on**: H-doctor-extensions (#808)
merged.

## Scope

`argo doctor watch` runs the existing doctor invariants on a
loop and prints a live dashboard. Useful for deploy windows
or production troubleshooting where you want to see
status drift in real time.

- `argo doctor watch [--interval <duration>] [--clear]`
- Default interval: 30s
- `--clear` redraws the screen each cycle (curses-style);
  default is append-mode for piping
- Reuses the existing `argo doctor --json` envelope under
  the hood — no new check logic, just the watch loop

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/doctor_watch.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-cli/src/cli_doctor_cmds.rs` — narrowly: `Watch`
  subcommand variant
- `crates/argo-cli/tests/doctor_watch_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo doctor watch` runs in a loop until Ctrl-C
- [ ] `--interval 5s` honored
- [ ] `--clear` redraws cleanly without flicker
- [ ] Append-mode emits one envelope per cycle
- [ ] 3 unit tests + 1 CLI smoke test (3-iteration capture)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-doctor-watch-cli`
- PR title: `cli: argo doctor watch — continuous health monitoring (codex-doctor-watch-cli)`
