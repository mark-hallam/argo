# V1.4 — Setup wizard CLI

**Owner**: codex. **Depends on**: G-genesis (#790),
H-doctor-extensions (#808) merged.

## Scope

`argo setup` is currently a single flat handler. Operators
new to Argo need a guided walk-through: install verification
→ workspace bootstrap → first worker spawn → first action
emit → approval reply. This brief turns `argo setup` into an
interactive wizard.

- Sub-steps printed as numbered checklist; each step prints
  `OK ✓` / `FAIL ✗ <reason>` / `SKIP <reason>`
- Steps:
  1. `argo doctor` invariants (re-uses existing JSON
     envelope; SKIP if already passing)
  2. Workspace dir present + `.argo/manifests/` writeable
  3. At least one `vertical.toml` registered
  4. At least one Worker bootstrapped (offers `argo worker
     spawn ...` if zero)
  5. At least one NotifyChannel manifest validated
  6. Genesis archetype suggestion table printed
  7. Sample `argo audit query` succeeds against a fresh DB
- `--non-interactive` flag skips prompts (CI mode)
- `--json` envelope: `{steps: [{name, status, detail}],
  ready: bool}`

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/setup.rs` — narrowly:
  refactor into thin top-level + delegate to step-specific
  sibling fns OR break into sibling files if it's already
  large
- `crates/argo-cli/src/handlers/setup_steps.rs` (NEW —
  step implementations)
- `crates/argo-cli/src/handlers/setup_render.rs` (NEW —
  table + JSON envelope formatting)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 2 new
  `pub mod` lines
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: add
  `--non-interactive` and `--json` flags to the `Setup`
  variant if not already present

## Acceptance criteria
- [ ] All 7 steps execute and report status
- [ ] `--non-interactive` mode runs without TTY prompts
- [ ] `--json` follows universal envelope shape
- [ ] Table output uses ASCII checkmarks (✓ / ✗)
- [ ] 6 unit tests (per step + JSON envelope) + 1 CLI
      smoke test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-setup-wizard-cli`
- PR title: `cli: argo setup interactive wizard (codex-setup-wizard-cli)`
