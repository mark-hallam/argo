# fix-genesis-hired-worker-run-path-tenant-scoped-vertical-resolve

**Owner**: claude or codex.
**Depends on**: none (the tenant-scoped resolver already exists; this wires it in).
**Authored against**: origin/main `6e8149f19` on `2026-06-10`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-genesis-hired-worker-run-path-tenant-scoped-vertical-resolve

## Goal

Make a **Genesis-hired teammate actually runnable end-to-end** — "Run now",
scheduled/cron runs, AND the turn-time charter/policy load — by teaching the
run path + agent runtime to resolve a worker's vertical from the
tenant-scoped `.genesis/<tenant>/<vid>/` tree, not only the flat `verticals/`
catalog.

## Why now (live e2e finding, 2026-06-10)

A teammate built through the Genesis onboarding studio gets a `vertical_id`
like `genesis-personal-019eaf2871121498` and is materialized to
`verticals/.genesis/<tenant-id>/<vertical-id>/vertical.toml`. Clicking
**"Run now"** on that teammate returns HTTP 409 **"this teammate has no
runnable role yet"**. A template-based teammate (vertical = flat
`customer-support`) runs fine. So the platform's headline flow ("describe in
plain English → hire → it works") produces a teammate that **cannot be run at
all**. This blocks the v1.0 demo and the DEC-042 product loop.

## Root cause (VERIFIED by reading the code)

1. **The run gate is genesis-blind.**
   `routes_worker_run_exec::vertical_loads(vertical_id)`
   (`crates/argo-cli/src/handlers/gateway/routes_worker_run_exec.rs:204-210`)
   calls `crate::handlers::vertical::resolve_verticals_dir()` →
   `argo_vertical::default_dir()` (flat `verticals/`) →
   `argo_vertical::load_dir()`
   (`crates/argo-vertical/src/loader.rs:35-79`), which does a **single-level**
   `read_dir` for `<sub>/vertical.toml` and never descends into
   `.genesis/<tenant>/<vid>/`. So `registry.get(vertical_id)` is `None` →
   `RunBlocked::NoVertical` → 409 `no_runnable_role`
   (`routes_worker_run_exec.rs:164-166`), before any session is created.

2. **The scheduled-run path has the same blindness.** `argo work serve`
   validates via the same flat loader
   (`crates/argo-cli/src/handlers/work.rs` `validate_vertical`, ~303-313), so
   cron/scheduled runs of a Genesis teammate are blocked too.

3. **Even past the gate, the turn-time charter/policy load is flat-only — a
   NO-THEATER trap.** `crates/argo-agent-runner/src/turn_vertical.rs` has three
   functions that each take `vertical_id: Option<&str>` only and use
   `argo_vertical::default_dir()`/`load_dir()`:
   - `build_vertical_deny_set` (lines 42-62) — tool deny-set,
   - `build_vertical_approval_set` (lines 80-100) — approval gates,
   - `resolve_vertical_for_session` (lines 115-123) — system prompt.
   Called for every worker at `turn_run_loop.rs:102-104`. For a Genesis worker
   these all return empty → it would run **with no system prompt, no tool
   deny-set, and no approval gates** (i.e. ungoverned). And
   `work_dispatch_phases.rs` `create_session_with_bundle` (~line 71) installs
   skills via the flat loader → Genesis skills don't attach. Fixing only the
   gate would let the worker "run" but ignore its own charter — that is theater,
   not a fix.

4. **The correct resolver already exists.**
   `genesis_tenant::genesis_vertical_dir_for_tenant(verticals_root, tenant_id,
   id)` (`crates/argo-cli/src/handlers/gateway/genesis_tenant.rs:60-68`)
   resolves the `.genesis/<tenant>/<vid>/` path and is **fail-closed
   cross-tenant** (returns `None` for another tenant's pack). `tenant_id` IS
   available at run time (`routes_worker_run_exec.rs` passes `worker.tenant_id`
   into `process_job`, ~line 180). BUT it is `pub(super)` to the gateway module,
   and `argo-agent-runner` has **no dependency on `argo-cli`** (it already
   duplicates `sanitize_tenant` locally at `turn_vertical_fence.rs:318-335`), so
   `turn_vertical.rs` cannot call it.

## Fix (first-principles: fix the resolver, not just the gate)

The root is that `argo-vertical` — the crate that owns vertical loading — is
genesis-blind. Add ONE canonical tenant-aware resolver there so every caller
uses it with `tenant_id`, with no cross-crate dep on `argo-cli` and no third
copy of the tenant-slug logic.

1. **Add a tenant-aware resolver to `argo-vertical`** (grep-confirm the name is
   free first; suggested `load_for_worker(root, tenant_id, vertical_id)` or
   `resolve_runnable_vertical(...)`): returns the loaded vertical whether it
   lives flat in `verticals/<vid>/` OR under `verticals/.genesis/<tenant>/<vid>/`;
   **fail-closed cross-tenant** (mirror `genesis_vertical_dir_for_tenant`
   semantics — never resolve another tenant's genesis pack). Centralize
   `sanitize_tenant` so `genesis_tenant.rs` and `turn_vertical_fence.rs`
   converge on it rather than adding a 3rd copy.
2. `routes_worker_run_exec::vertical_loads` → take `tenant_id`, call the new
   resolver. (`genesis_tenant.rs`'s helper becomes a thin wrapper or delegates.)
3. `work.rs::validate_vertical` (serve path) → same (see open Q on serve's
   `None` worker_id).
4. `work_dispatch_phases.rs::create_session_with_bundle` skills install → resolve
   via the tenant-aware path.
5. `turn_vertical.rs` `build_vertical_deny_set` / `build_vertical_approval_set` /
   `resolve_vertical_for_session` → accept `tenant_id` (thread from
   `session.tenant_id` at the `turn_run_loop.rs:102-104` call sites) and use the
   new resolver. NB: `turn_vertical.rs` is 360 LOC — adding logic may require a
   split (`turn_vertical_genesis.rs`) to stay <500.

## May touch

- `crates/argo-vertical/src/loader.rs` (add resolver; small additive)
- `crates/argo-cli/src/handlers/gateway/routes_worker_run_exec.rs` (283 LOC)
- `crates/argo-cli/src/handlers/work.rs` (313 LOC) `validate_vertical`
- `crates/argo-cli/src/handlers/work_dispatch_phases.rs` `create_session_with_bundle`
- `crates/argo-agent-runner/src/turn_vertical.rs` (360 LOC — watch ceiling) +
  `turn_run_loop.rs` call sites
- `crates/argo-cli/src/handlers/gateway/genesis_tenant.rs` (109 LOC — converge resolver)
- their `_tests` siblings

## Acceptance / verifier

1. New test: a Genesis-tenant-scoped vertical (materialized under
   `verticals/.genesis/<tenant>/<vid>/`) resolves as **runnable for its OWN
   tenant** and **NOT for another tenant** (cross-tenant fail-closed). Mirror
   `routes_worker_run_exec_tests.rs:178-258` but with a genesis pack (the
   existing test uses a genuinely-absent vertical + flat `customer-support`, so
   it never exercised the genesis path).
2. Gateway "Run now" on a Genesis-hired worker returns 200 + an action/session
   id (not 409 `no_runnable_role`).
3. The run loads the genesis vertical's **system prompt + deny-set +
   approval-set**: assert a charter-denied tool is actually blocked and an
   approval-gated action requires approval (proves NO-THEATER — the worker runs
   WITH its charter, not ungoverned).
4. Scheduled run (`argo work serve`) resolves the genesis vertical too, OR the
   PR documents why serve stays system/cwd-scoped.
5. `cargo test -p argo-cli -p argo-agent-runner -p argo-vertical` green; clippy
   clean; LOC budget (<500/file, <80/fn) respected.

## Size / owner / priority

~150-300 LOC across 2-3 crates (codex-suitable, or claude). **🔴 P1** —
Genesis "describe → hire → it works" is the platform headline
([DEC-010] Genesis, the NO-THEATER demo mandate); today **every** Genesis-hired
teammate is un-runnable. Blocks the v1.0 demo + the DEC-042 product loop.

Related (verify-before-fix done — none of these fix this gap):
`eval-argo-vertical-loader-tenant-isolation` (#2386, the read-only audit that
pinned this surface), `eval-claim-3-live-genesis-to-workflow-run-test`,
`fix-genesis-intake-tenant-resolution` (#2777, sibling tenant-resolve in the
Genesis *intake* path — this brief is the *run* path). Sibling e2e findings:
`fix-worker-run-memory-namespace-unbound-team-actions` and
`fix-agent-runtime-memory-tool-tenant-and-vertical-scope`.

## Open questions (resolve during implementation)

- Where to host the canonical resolver + `sanitize_tenant`. `argo-vertical` is
  recommended (owns loading; reachable from both `argo-cli` and
  `argo-agent-runner` with no `argo-cli` dep). Confirm crate-dep direction.
- `work.rs` serve passes `None` worker_id (`work.rs:229`) — is serve
  system/cwd-scoped by design, or should cron runs attribute to the worker and
  resolve its tenant?
- Verify `tenant_id` is plumbed to **every** turn-time asset read (prompts,
  workflows, tool policies), not just the gate, or the fix is incomplete.
