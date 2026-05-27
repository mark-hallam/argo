# Round 108 — Claim 2 (Delegation-as-unit) cold cargo

- **Date / fire**: 2026-05-27T20:26Z (07:26 AEDT 2026-05-28)
- **Round**: 108
- **Pillar / claim**: Claim 2 — Delegation-as-unit (live runner handoff-chain inheritance through `ToolContext.current_handoff_id`)
- **Origin/main SHA at run**: `e65328c5d`

## Verdict

🟢 **PASS** — both substrate-level Claim 2 live-runner tests pass cold against current main. Rotation per R107 "track for R108: rotate to Claim 2 (Delegation-as-unit) cold cargo" satisfied. Last touched at R102; the canonical wire-up shipped via PR #2256 + #2257 is still intact and exercised end-to-end through `run_turn_with_events`.

## Setup

```bash
export CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target
git rev-parse --short origin/main  # → e65328c5d

cargo test -p argo-agent-runner --test delegate_task_handoff_chain_runtime --release
```

The test exercises the live `run_turn_with_events` runner against a `wiremock::MockServer` provider, with a real `OrchestratorStore` backed by `SessionStore` in a `TempDir`. It is the canonical end-to-end pin for Claim 2.

## Observed

```
   Compiling argo-agent-runner v0.0.1 (~/code/argo/crates/argo-agent-runner)
    Finished `release` profile [optimized] target(s) in 2m 58s
     Running tests/delegate_task_handoff_chain_runtime.rs

running 2 tests
test delegate_task_without_inbound_handoff_leaves_chain_unset ... ok
test delegate_task_inherits_active_inbound_handoff_chain ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
```

2/2 PASS. 178s cold compile, 0.16s runtime.

## Anti-theater check

Three independent structural witnesses, not just an assertion that rewrites itself green:

1. **Real provider witness.** The test installs a `wiremock::MockServer` and routes a real HTTP-shaped provider request through `run_turn_with_events`. The `DelegateSequence::respond` impl asserts call #1 returns a `delegate_task` tool_use, and call #2's `latest_tool_result` is non-empty — i.e. the runner actually emitted a handoff id back to the LLM turn. Bypassing the chain wire-up would still let call #0 succeed but break the body shape on call #1.
2. **Production-code wire confirmed.** `rg current_handoff_id crates/argo-tool crates/argo-agent-runner` shows the consumption chain end-to-end:
   - [crates/argo-tool/src/tool.rs](crates/argo-tool/src/tool.rs): `pub current_handoff_id: Option<HandoffId>` on `ToolContext`
   - [crates/argo-agent-runner/src/turn_helpers_run.rs](crates/argo-agent-runner/src/turn_helpers_run.rs): runtime computes `current_handoff_id = current_inbound_handoff_id(...)` and threads it into dispatch
   - [crates/argo-agent-runner/src/dispatch.rs](crates/argo-agent-runner/src/dispatch.rs): `ctx.current_handoff_id = current_handoff_id` set per dispatch call
   - [crates/argo-tool/src/tools/delegate_task.rs](crates/argo-tool/src/tools/delegate_task.rs): `let Some(active_id) = ctx.current_handoff_id.as_ref() else { ... }` — the consumer
3. **Both polarity tests present.** `delegate_task_without_inbound_handoff_leaves_chain_unset` AND `delegate_task_inherits_active_inbound_handoff_chain` — i.e. the chain must extend when servicing an inbound handoff, and must NOT spuriously initialize otherwise. A single-direction test could pass on broken code; the pair pins both branches.

## Gap / follow-up brief candidates

None. Claim 2's substrate pin is dispositive and the production-code wire is intact across 4 files. Continue the rotation: R109 should rotate to **Claim 6 (multi-tenant by construction)** cold cargo — Claim 6 last touched at R103; R107 covered Claim 5, R108 covers Claim 2.

## Round 108 anti-theater verification count

122nd anti-theater verification this week (R102 + R103 + R104 + R105 + R106 + R107 = 121 prior; this round adds 1).
