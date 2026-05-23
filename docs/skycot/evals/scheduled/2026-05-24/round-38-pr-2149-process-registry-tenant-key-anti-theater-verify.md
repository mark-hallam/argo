# Round 38 — PR #2149 `mission — argo-tool — tenant-key ProcessRegistry` anti-theater verify

- **Date**: 2026-05-24
- **Round**: 38
- **Pillar / Claim**: Claim 6 — Multi-tenant by construction (per-tenant ProcessRegistry isolation closes F2 from #2117 audit)
- **PR**: [#2149](https://github.com/mark-hallam/argo/pull/2149) — MERGED at `6291ce3e9`
- **Verdict**: 🟢 PASS (no severity — closes a P1 from #2117)

## Setup

Production source — `grep -n tenant_id crates/argo-tool/src/process_registry.rs` + substrate-pin file inversion check + new integration file + cold cargo.

## Observed

### Production seam — `crates/argo-tool/src/process_registry.rs`

The PR threads `tenant_id: TenantId` through **every** public method, not just `spawn` / `kill`:

| Line | Method | Tenant enforcement |
|---|---|---|
| `:128` | `ProcessHandle` struct | new `tenant_id: TenantId` field |
| `:157-191` | `spawn(req, tenant_id)` | stamps tenant on handle at `:191` |
| `:200-206` | `list(tenant_id)` | `.filter(|(_,h)| h.tenant_id == tenant_id)` |
| `:221-222` | `status(id, tenant_id)` | delegates to `lookup_for_tenant` |
| `:248-249` | `tail(id, tenant_id)` | delegates to `lookup_for_tenant` |
| `:263-290` | `kill(id, tenant_id, grace)` | `lookup_for_tenant` + `remove_if_tenant_matches` |
| `:298-324` | `reap_finished(tenant_id)` | double-check `handle.tenant_id != tenant_id` before removing |
| `:331-340` | `len/is_empty(tenant_id)` | counts only own tenant's entries |
| `:350` | `total_len()` | new admin-only aggregate (no tenant arg) — explicitly scoped non-tenant escape hatch |

The `kill` surface returns the same `no such process` text for both "unknown pid" and "wrong tenant" — the PR body promises this and `:267-290` confirms (function calls `lookup_for_tenant` first and short-circuits with `ArgoError::Tool { msg: "no such process: …" }` on `None`, identical to a genuinely-unknown pid). **No existence leak.**

### Substrate pin — inverted F2 from #2117

`crates/argo-tool/tests/argo_tool_substrate_pin_test.rs:409` renames `process_registry_kill_is_tenant_blind_today` → `process_registry_kill_rejected_across_tenants`, removes `#[ignore]`, asserts the opposite:

- Two `ToolContext`s share **one** `Arc<ProcessRegistry>` but carry distinct `tenant_id`s (`0xA` vs `0xB`).
- Tenant A spawns a real `sleep 60` background process via `Terminal::default().dispatch(...)` — production dispatcher path.
- Tenant B calls `Process.dispatch({"action":"kill","pid":pid_a}, &ctx_b)` — production dispatcher path again.
- The PR-changed code path is exercised end-to-end: `process.rs:action_kill` reads `ctx.tenant_id`, calls `registry.kill(&id, tenant_id, grace)`, which calls `lookup_for_tenant(id, tenant_b)` and returns `None` because the handle was minted with `tenant_a`.
- Assertion 1 (`:452-455`): `kill_result.is_error == true`.
- Assertion 2 (`:457-463`): error text contains `"no such process"` — proves the error mirrors the unknown-pid path. Existence not leaked.
- Assertion 3 (`:467-477`): `registry.len(tenant_a) == 1` AND `registry.len(tenant_b) == 0` — tenant A's entry **survives** the cross-tenant kill attempt.

### New integration file — 6 tokio::tests

`crates/argo-tool/tests/process_registry_tenant_isolation.rs` exhaustively covers all 7 tenant-scoped methods:

1. `list_returns_only_own_tenants_entries` — spawn for A and B, `list(tenant_a)` returns A's id only.
2. `status_returns_none_across_tenants` — `status(id_a, tenant_b) == None`.
3. `tail_returns_none_across_tenants` — **anti-theater killer test**: tenant A runs `printf "tenant-a-secret\n"` (line 99) which lands in registry stdout; owner reads it back, non-owner gets `None`. Proves the gate is "no stdout bytes for B" not just "different error code".
4. `kill_across_tenants_errors_like_unknown_pid` — `ArgoError::Tool { tool: "process_registry", msg: contains "no such process" }` shape match.
5. `reap_finished_only_touches_own_tenant` — A's reap drops A's exited entry, B's identical entry untouched (`reg.len(tenant_b) == 1` after A's reap).
6. `len_and_is_empty_are_per_tenant` — interleaved spawns/kills, per-tenant counts stay independent.

### Cold-run cargo

```
cargo test -p argo-tool --test argo_tool_substrate_pin_test process_registry_kill_rejected_across_tenants
  → test process_registry_kill_rejected_across_tenants ... ok
  → test result: ok. 1 passed; 0 failed; 0 ignored

cargo test -p argo-tool --test process_registry_tenant_isolation
  → 6 tests all ok
  → test result: ok. 6 passed; 0 failed; 0 ignored
```

**7/7 green cold from `origin/main`.**

## Anti-theater check

Three angles that would have caught a "test rewrites itself green" approach:

1. **Inverted-pin matches production behaviour.** The F2 pin at `:409` does NOT assert against `registry` directly — it routes through `Terminal::default().dispatch(...)` (real production tool-dispatch entry) and `Process.dispatch(...)` (real production process-tool dispatch entry). If the PR had only fixed `ProcessRegistry::kill` but forgot to thread `tenant_id` through `process::action_kill`, the test would fail because `ctx.tenant_id` wouldn't reach the registry call. Production grep at `crates/argo-tool/src/tools/process.rs` confirmed (`:71-92` for `action_kill` pulling `ctx.tenant_id`).
2. **Stdout-bytes leak test in #3** — `tail_returns_none_across_tenants` doesn't just check that `tail` returns a different error; it spawns a process that writes a distinct secret string to stdout, then asserts the non-owner gets `None`. A future regression that "returns dummy TailOutput for cross-tenant" would surface here.
3. **Production caller audit.** `rg "ProcessRegistry::(spawn|list|status|tail|kill|reap_finished|len|is_empty)" crates/` is the natural follow-up — `ProcessRegistry` is currently latent in production (per the F1 finding still `#[ignore]`'d at `:366`). The brief explicitly defers production wire-up to the sibling `mission-agent-runner-wire-process-registry-per-session`. **This PR closes the F2 (data layer) but the surface stays dead-in-production until F1's wire-up brief lands.** The PR body is honest about this scope split.

## Gap / follow-up brief candidates

**NONE NEW.** Pre-existing follow-up:

- `mission-agent-runner-wire-process-registry-per-session` — **P1, ~150-250 LOC**. The F1 sibling. Wires a per-`AgentSession` `Arc<ProcessRegistry>` into `ToolContext::background_processes` so production dispatches reach a registry. Then un-ignore + flip the F1 pin (`production_tool_context_lacks_process_registry_today` at `argo_tool_substrate_pin_test.rs:366`). With F2 already landed at the data layer, the natural wire-up path can't accidentally reopen the cross-tenant leak.

`total_len()` admin escape hatch at `:350` is currently uncalled. If left uncalled for >2 rounds, recommend deletion as latent-surface drift.

## Closing

The 19th anti-theater-honest substrate contract on `main` this week. ProcessRegistry is now tenant-keyed at the data layer; surface remains latent in production callers (dead-code by design, activated by F1's sibling brief).
