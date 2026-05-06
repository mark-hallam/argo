# V1.1 — Architectural prep: split worker_actions.rs

**Owner**: claude. **Depends on**: nothing (pure refactor).

## Scope

`crates/argo-cli/src/handlers/worker_actions.rs` is at 512
LOC, over the project's 500-LOC-per-file invariant. This
blocks downstream briefs that need to extend the worker-action
flow (codex-multi-step-approval, future signature-flow work).

Pure refactor — no behavior change. Split the file into
focused sibling-file modules, keep the public API stable so
the existing 100+ call sites don't have to change.

## Approach

Inspect existing sibling files in
`crates/argo-cli/src/handlers/` already split off from the
worker_actions surface:
- `worker_actions_dispatch.rs`
- `worker_actions_transitions.rs`
- `worker_actions_emit.rs` / `worker_actions_emit_plan.rs` /
  `worker_actions_emit_tests.rs`
- `worker_actions_executor.rs`
- `worker_actions_followup.rs`
- `worker_actions_items.rs`
- `worker_actions_list.rs`
- `worker_actions_render_tests.rs`
- `worker_actions_auto_approval.rs`

The 512-LOC `worker_actions.rs` itself likely holds: the
ActionRecord plumbing, ledger row helpers, JSON envelope
builders, render utilities, and a handful of utility
functions. Identify ≥3 cohesive groups and extract each into
its own sibling file (~120-180 LOC per slice).

Suggested split (final names at author's discretion):
- `worker_actions_record.rs` — ActionRecord struct + ledger
  row helpers + serde glue
- `worker_actions_render.rs` — table + JSON envelope
  rendering (the human-output paths)
- `worker_actions_query.rs` — single-row lookup + filtering
  helpers used by other handlers

After the split:
- `worker_actions.rs` ≤ 250 LOC; only re-exports + the small
  shared bits that don't fit elsewhere
- Every new sibling ≤ 250 LOC
- All existing tests pass unchanged

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/worker_actions.rs` —
  reduce to ≤250 LOC; keep public re-exports
- `crates/argo-cli/src/handlers/worker_actions_record.rs`
  (NEW) — or whatever names the split chooses
- `crates/argo-cli/src/handlers/worker_actions_render.rs`
  (NEW)
- `crates/argo-cli/src/handlers/worker_actions_query.rs`
  (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: `pub
  mod` declarations for new files
- Any handler file that currently imports symbols from
  `worker_actions::` — narrowly: update import paths if a
  symbol moved (mechanical)

## Acceptance criteria
- [ ] `worker_actions.rs` ≤ 250 LOC
- [ ] Each new sibling file ≤ 250 LOC
- [ ] No public-API change (every existing call site still
      compiles unchanged)
- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` — same set of tests pass as
      before (no test deletions; no skips)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] No changes to `worker_actions_*_tests.rs` files
      (they should keep working unchanged)

## Branch + PR
- Branch: `claude-worker-actions-split`
- PR title: `refactor: split worker_actions.rs under 500 LOC budget (claude-worker-actions-split)`
