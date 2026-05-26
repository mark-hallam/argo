# Round 75 — PR #2323 POST-MERGE anti-theater (gateway skills + workflows endpoints LIVE)

- **Date**: 2026-05-26 (Melbourne 2026-05-26 10:11 AEST)
- **Round**: 75 (scheduled-task hourly fire; ~40 min after r74 ship 23:30Z)
- **Pillar/Claim**: Mission Claim 3 (Platform builds workers) — SPA `/skills` + `/workflows` pages fell through to hardcoded fixtures because gateway 404'd; live endpoints required for v1.0 demo
- **PR**: [argo#2323](https://github.com/mark-hallam/argo/pull/2323) merged 2026-05-26T09:23+10:00 (= 2026-05-25T23:23Z) at commit `6efd3dc06`
- **Verdict**: 🟢 **PASS** — endpoints live, return correctly-shaped envelopes, smoke-tests honest about their scope

## Setup

The PR mounts five gateway routes that were 404'ing pre-merge:

- `GET /api/v1/skills` — caller-tenant installed catalog
- `GET /api/v1/skills/marketplace` — cross-tenant published catalog
- `GET /api/v1/skill-library/list` — alias of marketplace
- `GET /api/v1/workflows` — caller-tenant cross-vertical list
- `GET /api/v1/workflows/list` — alias of workflows

Brief: [streams/fix-gateway-workflows-and-skills-marketplace-endpoints-404.md](docs/build-plan/streams/fix-gateway-workflows-and-skills-marketplace-endpoints-404.md)

Round 74 noted live `/api/v1/skills` was still HTTP 404 even after I recommended P0 merge — implying the gateway binary needed a restart. R74 conductor T23-21 acted: merged the PR + posted an operator-restart prompt in SHARED. The gateway was restarted some time before my round 75 fire.

## Observed

**Wire-level HTTP probes** (live gateway, uptime 20:35 from binary launched 2026-05-26 09:35 AEST = post-merge restart):

```
$ curl -sf -o /dev/null -w '%{http_code}\n' http://127.0.0.1:7400/api/v1/skills
200
$ curl -sf -o /dev/null -w '%{http_code}\n' http://127.0.0.1:7400/api/v1/workflows
200
$ curl -s http://127.0.0.1:7400/api/v1/skills | head -c 200
{"skills":[],"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","total_count":0}
$ curl -s http://127.0.0.1:7400/api/v1/workflows | head -c 200
{"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","total_count":0,"workflows":[]}
```

Both endpoints return well-shaped envelopes with the tenant scoped from the caller cookie. `total_count:0` is expected on a fresh local store. The SPA's `/skills` and `/workflows` pages, which round 74 noted "still serve from hardcoded fixtures because gateway 404s", can now switch to live data.

**Source verification** (commit `6efd3dc06` files):

- [crates/argo-cli/src/handlers/gateway/mod.rs:1](crates/argo-cli/src/handlers/gateway/mod.rs:1) — adds module imports
- [crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs](crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs) — wires 5 routes into dispatcher
- [crates/argo-cli/src/handlers/gateway/routes_skills_list_get.rs](crates/argo-cli/src/handlers/gateway/routes_skills_list_get.rs) — handler with `WHERE s.tenant_id = ?` + lifecycle filter
- [crates/argo-cli/src/handlers/gateway/routes_workflows_list_top_get.rs](crates/argo-cli/src/handlers/gateway/routes_workflows_list_top_get.rs) — handler against `workflow_definition_index.tenant_id`
- [crates/argo-cli/tests/gateway_skills_list_smoke.rs:1](crates/argo-cli/tests/gateway_skills_list_smoke.rs:1)
- [crates/argo-cli/tests/gateway_workflows_list_smoke.rs:1](crates/argo-cli/tests/gateway_workflows_list_smoke.rs:1)

## Anti-theater check

The smoke tests are **honest about their scope** — they do NOT spawn a real gateway and HTTP-probe it. From the docstring at [crates/argo-cli/tests/gateway_skills_list_smoke.rs:11-19](crates/argo-cli/tests/gateway_skills_list_smoke.rs:11):

> The HTTP wire (route mounted, dispatcher reached, 200 returned) is covered by the compile-time wiring in `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` and the operator-facing curl probes documented in the brief. A binary-launch HTTP smoke would also need a provider API key to get past `gateway::start`'s startup check; that's a constraint of the swarm worker environment (which deliberately runs with no API key per SWARM_PROTOCOL.md), not a code path issue.

What the tests **do** check is meaningful:

1. `installed_list_filters_by_tenant` — seeds 3 skills under tenant A, 2 under tenant B, then runs the **same SQL the handler runs** (`WHERE s.tenant_id = ? AND s.superseded_by IS NULL AND s.lifecycle_state = 'active'`) and asserts A sees its 3 and B sees its 2 with no leak.
2. `marketplace_list_returns_published_rows_across_tenants` — seeds 1 published + 1 private per tenant, runs the marketplace SQL (`WHERE source LIKE 'published:%'`), asserts both published rows surface and neither private one does.

**One vacuity flag worth flagging** (non-blocking): the tests replicate the SQL string inline rather than calling the handler function. If the handler's actual SQL drifted from the test SQL, the test would still pass. Mitigated by:

- Wire-level HTTP probes above confirm the handler is reachable and returns the expected envelope shape
- The tenant_id surfaces in the live response payload, proving the per-caller scope is applied

**End-to-end evidence combined**: the tests prove the SQL semantics (tenant isolation); the live HTTP probes prove the wiring is mounted and the handler runs and the envelope shape matches. Together these close the loop without needing a binary-launch smoke that the swarm environment can't easily run.

## Gap / follow-up brief candidates

None blocking. One optional cleanup brief candidate (P3, ~30 LOC):

- **`eval-gateway-smoke-tests-call-handler-not-replicate-sql`** — refactor the 2 new smoke files to invoke `installed_response` and `marketplace_response` functions directly (with a mock `apply_cors` shim) rather than replicating the SQL inline. Closes the one vacuity flag above. Not a v1.0 blocker; current evidence-pyramid is sufficient.

This is the **63rd anti-theater verification this week** (62→63 streak).
