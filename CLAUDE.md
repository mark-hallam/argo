You are a coding agent on the Argo project. The master spec is
`docs/plan/argo-build-plan.md` (v0.2). Read it before any non-trivial change.

Five non-negotiable invariants — every PR is checked against them:
  1. Polyglot core (Rust core + JSON-RPC plugin ABI; plugins are subprocesses)
  2. Multi-tenant by construction (tenant_id plumbed through every primitive)
  3. Capability-based security with first-class RBAC
  4. Provider manifests, not adapters (TOML, not Rust)
  5. MCP-first integration (8 in-core tools; everything else MCP)

Hard rules — CI rejects PRs that violate any of these:
  - No file > 500 LOC
  - No fn > 80 LOC
  - No struct impl block > 200 LOC
  - No `unsafe` Rust outside marked FFI shims
  - No `unwrap`/`expect` outside `#[cfg(test)]`
  - No global mutable state. AgentDeps is the only injection point.
  - Every public Rust API has rustdoc with at least one example
  - Every spec section ends in acceptance criteria; every PR claiming to
    implement a section MUST tick those boxes in the PR description.

When in doubt:
  - Subtract before adding. Idiot Index is solution complexity ÷ problem complexity.
  - Question the requirement. If a spec section doesn't make sense, raise an
    issue against the spec; do not invent.
  - Ship the 80% solution. Working software beats the perfect solution that
    arrives late.

PR title format: "<phase> — <section> — <one-line summary>"
