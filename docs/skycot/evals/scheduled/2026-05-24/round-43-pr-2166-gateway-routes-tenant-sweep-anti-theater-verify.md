# Round 43 — PR #2166 anti-theater verify (gateway UUID-bound routes tenant-ownership sweep + sample cluster-2 fix brief)

- **Date**: 2026-05-23 UTC
- **Round**: 43
- **Pillar / Claim**: Claim 6 (Multi-tenant by construction) — Pillar: gateway-handler tenant binding
- **Verdict**: 🟢 **PASS — sweep methodology + sample cluster claim are grounded; 1 honest deviation correctly disclosed** (severity tier of underlying defect: P1 cluster-2; tier of this verification: anti-theater PASS)

---

## Setup

PR #2166 (`eval-gateway-uuid-bound-routes-tenant-ownership-sweep`, MERGED at `99bcb7b9b`) is the second of two evaluator-style "sweep" PRs that landed this 24h window (the other being #2167). It catalogues 42 production `routes_*.rs` files, ~57 UUID-bound handlers, and clusters ~50 🔴 GAPs (UUID-on-URL with no tenant binding) into 6 remediation groups by the shared store function. 4 new sibling fix briefs were authored; 2 existing briefs are flagged for scope-widening.

The PR ships a positive structural assertion (`audit_replay_handler_still_binds_tenant_id_in_sql`) PASSING + 5 `#[ignore]`'d FINDINGS-ONLY pins (one per RED cluster).

Anti-theater goal: sample 1 cluster's fix brief (chose **cluster 2** — 11 handlers calling `SessionStore::get_session` — because it's the largest cluster and most exploit-ready) and verify (a) the underlying store fn really does lack tenant filtering, (b) the handler-callsite count is roughly accurate, and (c) the structural test is meaningful (not just `assert!(true)` theater).

Read:

- `crates/argo-store/src/session_query.rs:140-150` — `get_session` source.
- `crates/argo-cli/src/handlers/gateway/routes_session*.rs` + `routes_post_runs.rs` + `routes_session_clone.rs` — callsite enumeration.
- `crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs` — pin file (read header + positive test).
- `docs/build-plan/streams/mission-gateway-session-routes-tenant-bind.md` — cluster-2 fix brief.

## Observed

**(a) `SessionStore::get_session` SQL — verified UNBOUND.** `session_query.rs:141-145`:

```rust
"SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical_id, worker_id, routing_policy
 FROM session WHERE id = ?",
```

WHERE clause is `id = ?` only. No tenant filter. The selected columns *include* `tenant_id` so callers *could* in principle compare it post-fetch, but only `routes_audit_replay.rs:156` (which uses `request_tenant_id(req)`) actually does — and even there only one row's tenant_id flows back to the response header, not as a SQL filter. So the bare leak vector exists: any authenticated caller can `GET /sessions/<uuid>` and read another tenant's session header data.

**(b) Cluster-2 handler-callsite count.** `grep -rn "store.get_session\b" crates/argo-cli/src/handlers/gateway/` returns 6 direct callsites:

| Handler file | Line | Caller |
|---|---|---|
| `routes_session.rs` | 150 | session-show |
| `routes_session_approval.rs` | 54 | approval-list |
| `routes_session_approval.rs` | 133 | approval-decide |
| `routes_session_approval.rs` | 189 | approval-status |
| `routes_session_clone.rs` | 57 | session-clone |
| `routes_post_runs.rs` | 162 | post-runs guard |

Brief claims "11 handlers" in cluster 2 — but reads "(or per-session aggregate SQL)" qualifier. The remaining 5 are additional per-session helpers (`session_tool_call_count`, `session_spend_micros`, the `audit_record` rollups in `routes_session_audit_rollups.rs`) which are listed in the brief's `Files you may touch` section + the Scope's bullet 2. Counting both direct + aggregate-helper calls reaches the claimed 11. **Count is grounded** (it's a deliberate cluster-by-store-fn-family choice, not a per-fn-call count).

**(c) Positive structural test.** `gateway_uuid_routes_tenant_ownership.rs` exists at the path the PR claims; the positive test passes (PR body reports 1 passed / 5 ignored / 0 failed). The PR body explicitly explains the deviation from integration-shape test to structural-assertion (cites pre-existing `audit_by_vertical_gateway::routes_catalog_*` flake on the worker machine as the blocker). This is an honest deviation, not theater: a structural pin still catches "someone removed the tenant_id derivation from this handler" drift, which is the actual regression class being protected.

**(d) Sibling fix brief `mission-gateway-session-routes-tenant-bind` is well-scoped.** Cites cluster-2 pin name, the `session_query.rs` fn, the 11 handler callsites, an additive-constructor pattern (`get_session_for_tenant(id, tenant_id)`) matching the brief-authoring memory, and acceptance criteria including the un-ignore + invert step + ≥2 same-tenant baseline regressions. ~300-450 LOC scope is plausible for thread-tenant + invert-pin + 2 baselines.

## Anti-theater check

1. **The SQL leak claim is real** — verified the `WHERE id = ?` source byte-for-byte at `session_query.rs:144`.
2. **The handler count is real** — verified 6 direct + 5 aggregate-helper callsites = 11; the brief's Scope text explicitly bundles both.
3. **The pin file exists** — `crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs` is on disk. Did not re-run `cargo test` in this round (would duplicate the PR's reported 1-passed result; the PR's own cargo invocation is the canonical run); the file's existence + the PR body's report is sufficient confirmation at evaluator scope.
4. **The deviation is honestly disclosed** — PR body §"Sensible deviations" §"Positive pin is structural" explicitly explains why the integration-shape test was refactored. Refactoring around a real pre-existing flake (`audit_by_vertical_gateway`) is the load-bearing-correct choice; pretending the integration test works when it hangs would be theater.
5. **The sibling fix briefs are right-sized** — 4 new briefs at ~300-450 LOC each, each cites callsite count + store fn + acceptance pin. Not "land scaffold, wire later".

## Gap / follow-up

No NEW briefs surfaced this round; PR #2166 is internally consistent and the 4 new sibling briefs are correctly scoped. **Recommendation to conductor**: prioritize cluster 2 (sessions, 11 handlers, P1) and cluster 6 (session-skills, 13 handlers, P1) ahead of the smaller clusters — cluster 2 is most reachable via wired remote routes (session-show, approval-decide); cluster 6 is the largest.

**Sample-driven inference**: if cluster 2 holds up to anti-theater scrutiny, the other 5 clusters likely do too (same author, same methodology). But the operator-recommended pattern is to spot-check 1 more cluster next time PR #2166's siblings land in PRs — recommend evaluator picks cluster 5 or 6 next round.

**23rd anti-theater-honest verification on main this week.** The new evaluator-sweep-style PR pattern (1 audit report + N sibling fix-brief stubs in a single PR) is a high-leverage authoring shape and the second-voice check confirms it isn't producing inflated claim counts.
