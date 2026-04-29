# Contributing to Argo

Read `docs/plan/argo-build-plan.md` first. Especially **§1.2 — the five invariants**. Every PR is reviewed against them:

1. **Polyglot core.** Rust core + JSON-RPC plugin ABI. Plugins are subprocesses.
2. **Multi-tenant by construction.** `tenant_id` plumbed through every primitive.
3. **Capability-based security with first-class RBAC.** Every dispatch checks declared capabilities.
4. **Provider manifests, not adapters.** Quirks live in TOML, not Rust.
5. **MCP-first integration.** 8 in-core tools; everything else is an MCP server.

## Hard rules — CI rejects PRs that violate

- No file > 500 LOC
- No fn > 80 LOC
- No struct impl block > 200 LOC
- No `unsafe` Rust outside marked FFI shims
- No `unwrap`/`expect` outside `#[cfg(test)]`
- No global mutable state. `AgentDeps` is the only injection point.
- Every public Rust API has rustdoc with at least one example.
- Every spec section ends in acceptance criteria; every PR claiming to implement a section MUST tick those boxes in the PR description.

## PR title format

`<phase> — <section> — <one-line summary>`

Example: `Phase 1 — §5 — argo-types: capabilities, budgets, errors`.

## Workflow

1. Find the section of `docs/plan/argo-build-plan.md` you are implementing.
2. Reproduce its acceptance criteria in your PR description.
3. Write code + tests until each acceptance box is checked.
4. Run `cargo nextest run` and `cargo clippy --all-targets -- -D warnings` locally.
5. Open the PR with the title format above.

## When in doubt

- **Subtract before adding.** The Idiot Index is solution complexity ÷ problem complexity.
- **Question the requirement.** If a spec section doesn't make sense, raise an issue against the spec; don't invent.
- **Ship the 80% solution.** Working software beats the perfect solution that arrives late.

If you cannot complete a task, do NOT mark it complete. Open an issue documenting the blocker. Stay on it; do not move sideways to a different task.
