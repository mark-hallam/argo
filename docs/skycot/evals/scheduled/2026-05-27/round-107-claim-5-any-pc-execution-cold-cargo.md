# Round 107 — Claim 5 (Any-PC execution) cold cargo

- **Date / fire**: 2026-05-27T19:26Z (06:26 AEDT 2026-05-28)
- **Round**: 107
- **Pillar / claim**: Claim 5 — Any-PC execution (MCP cap-audit + pool routing substrate; macOS cu smoke documented skip-path)
- **Origin/main SHA at run**: `577827919`

## Verdict

🟢 **PASS** — both substrate-level Claim 5 LIVE tests pass cold against current main. Last touched ≥6 rounds back (R101); rotation per R106 "track for R107" item satisfied. macOS cu smoke test skipped this round (operator-granted-binary TCC scoping requires `argo dev claim-5-cu-smoke` subcommand which doesn't ship yet — same documented skip-path that prior rounds noted; not a regression).

## Setup

```bash
export CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target
git rev-parse --short origin/main  # → 577827919

time cargo test -p argo-workflow --test claim_5_live_mcp_cap_audit_test --release
time cargo test -p argo-workflow --test claim_5_live_mcp_pool_routing_test --release
```

Substrate-level tests run against the `FakeMcpServerPool` witness — they exercise the runtime's cap-check / audit-row / tenant-keyed-pool wiring without needing live MCP servers or operator TCC grants.

## Observed

```
running 4 tests
test mcp_fake_server_dispatch_error_is_surfaced_not_swallowed ... ok
test mcp_fake_server_audit_row_carries_tenant_and_principal ... ok
test mcp_fake_server_cap_check_fires_before_dispatch ... ok
test mcp_fake_server_transport_500_takes_same_failure_path ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
                finished in 0.00s
```

Compile time: 58.41s cold (first-time release build for this target after CARGO_TARGET_DIR switch).

```
running 2 tests
test mcp_fake_server_call_is_keyed_by_tenant_and_server ... ok
test mcp_fake_server_two_tenants_route_independently ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
                finished in 0.00s
```

Compile time: 9.03s warm (cap-audit deps cached the workspace).

LOC for the two test files: 286 + 191 = 477 LOC. Both are substrate-level (use `FakeMcpServerPool` rather than mocking out the runtime); production code paths exercised include `WorkerSessionRunner::new`, `Executor::execute`, `TableCapabilities::granting`, audit-row tenant-binding, and per-tenant MCP pool keying.

## Anti-theater check

The cap-check test (`mcp_fake_server_cap_check_fires_before_dispatch`) has a load-bearing assertion that distinguishes it from a vacuous pass. It sets up:

1. A `FakeMcpServerPool` that has `route_ok` configured for the `github-mcp` server (so a dispatch *would* succeed if it reached the pool).
2. A `TableCapabilities` table that grants `create_issue` to `alice-has-grant` only.
3. A `WorkerSessionRunner` constructed with `principal_without_grant = bob-no-grant`.
4. Executes a one-step workflow requesting `create_issue`.
5. The load-bearing assertion: `assert!(pool.calls().is_empty(), "cap-check must refuse before MCP dispatch; got calls={:?}", pool.calls())`.

If the runtime ever skipped the capability gate, the fake server would record the call — `pool.calls()` would not be empty. The test cannot self-rewrite green because the fake pool is the structural witness, not a configurable mock-return. Same shape for the tenant-routing test: it sets up two tenants, dispatches one call per tenant, and asserts each fake-pool slot received exactly its tenant's call — cross-tenant leakage would surface as a non-empty `calls()` on the wrong slot.

The pool-routing test is what closed claim 6 + claim 5's overlapping multi-tenant surface; the cap-audit test is the load-bearing claim 5 cap-gate witness. Both pass cold on current main.

## Gap / follow-up brief candidates

None this round.

Track for R108-R109:

- **macOS cu smoke test rotation**: the third claim-5 file (`claim_5_live_macos_cu_smoke_test.rs` in argo-executor-computer-use-macos) is the operator-perms-dependent one. Round 95 evaluator already flagged + operator merged with skip-path acknowledged; the `ops-claim-5-add-canonical-argo-binary-cu-smoke-subcommand` follow-up brief was *not* authored (round-95 surfaced for operator decision; still pending). Re-surface only if Claim 5 substrate ever regresses or operator wants to close the cu-smoke evidence gap.
- **Claim 2 (Delegation-as-unit) rotation**: hasn't been touched in 3 rounds (R104 was claim 1, R105 was claim 4, R106 was claim 3, R107 is claim 5). R108 should cover claim 2 — `delegate-task fallback-chain inheritance` via the live runner dispatch + `ToolContext.current_handoff_id` propagation.
