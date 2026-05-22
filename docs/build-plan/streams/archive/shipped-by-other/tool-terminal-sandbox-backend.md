# tool-terminal-sandbox-backend — terminal `backend: local|sandbox` arg

**Owner**: claude. **Depends on**: nothing (`argo-sandbox` crate confirmed at `crates/argo-sandbox/` with `Sandbox` trait + `LocalSandbox` and stub `DockerSandbox`). **Tier B — CLI execution depth.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

The `terminal` tool currently shells out via `tokio::process::Command` directly,
bypassing the existing `argo-sandbox` seam. That's fine for trusted workspaces
but rules out agent-driven `npm install`, `pip install`, or arbitrary user code
without polluting the host. Wiring `terminal` through the existing `Sandbox`
trait — with a `backend` arg — gives us host-clean execution today (via the
sandbox crate's existing `LocalSandbox`) and a drop-in path for `DockerSandbox`
/ E2B microVM dispatch tomorrow, without re-implementing capability gating.

## Approach (concrete)

1. Extend `terminal::Args` with `backend: Option<String>` (`"local"` default,
   `"sandbox"` opt-in). Reject any other value with a clear `ArgoError::Tool`.
2. Refactor `run_command` to build an `argo_sandbox::RunRequest` and dispatch
   through a `Box<dyn Sandbox>` instead of calling `tokio::process::Command`
   directly. Map `RunResult` → existing `format_output`.
3. Backend selection: `"local"` → `argo_sandbox::LocalSandbox` (already in
   crate). `"sandbox"` → for now, also `LocalSandbox` but the dispatch logs a
   clear `audit_event` indicating sandbox-mode was requested. Wiring to the
   real Docker/E2B backend is a follow-up brief once `DockerSandbox` lands;
   this brief is the seam, not the microVM.
4. Capability differentiation: `required_capabilities` returns
   `Spawn::Subprocess` for `local`, **and a NEW `SpawnKind::Container` arm**
   for `sandbox` (variant exists already at `crates/argo-types/src/capability.rs:83`).
5. Update the parameter JSON schema to expose the new `backend` enum field
   with `default: "local"`.
6. Add the `argo-sandbox` workspace dependency to `crates/argo-tool/Cargo.toml`
   (already a workspace member, so `argo-sandbox = { workspace = true }`).
7. Add tests + rustdoc example. Update vertical-manifest allow-list for at
   least one engineering-style vertical that wants `terminal` access (see
   acceptance below).

## Files you may touch

- `crates/argo-tool/src/tools/terminal.rs` — primary target, currently 238 LOC; expected to land ~330 LOC; **must stay <500.**
- `crates/argo-tool/Cargo.toml` — add `argo-sandbox` dep.
- `verticals/gtm-engineer/vertical.toml` — move `terminal` from `denied` to `allowed` for the GTM engineer (intentional engineering vertical), OR leave verticals untouched and document the decision in the PR body. Brief author preference: keep verticals untouched in this PR — separate brief should re-tier vertical permissions once all three Tier B tools land.

## Files you must NOT touch

- `crates/argo-tool/src/tools/mod.rs` — `Terminal` is already registered there.
- Any other tool implementation.
- `crates/argo-sandbox/` — consume only the public API; no edits.
- `crates/argo-types/src/capability.rs` — `SpawnKind::Container` already exists.

## Acceptance criteria

- [ ] `backend: "local"` default preserves existing behavior byte-for-byte
- [ ] `backend: "sandbox"` dispatches via `Sandbox` trait (even if `LocalSandbox` under the hood for now)
- [ ] `required_capabilities` returns `Spawn::Subprocess` for local, `Spawn::Container` for sandbox
- [ ] At least one vertical's manifest allows `terminal` (or PR body explains deferral to follow-up)
- [ ] 3+ unit tests: backend default, sandbox dispatch path, capability differentiation
- [ ] 1 integration test (in `crates/argo-tool/tests/`) end-to-end through the registry with both backends
- [ ] Rustdoc with example showing both `backend` values
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool` green
- [ ] `terminal.rs` stays under 500 LOC

## Branch + PR

- Branch: `tool-terminal-sandbox-backend`
- PR title: `Tier B — terminal — add sandbox backend dispatch via argo-sandbox`
