# mission-genesis-gateway-route-streaming-api

**Owner**: claude.
**Authored against**: origin/main `<latest>` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-gateway-route-streaming-api`.
**Blocks**: Genesis (MISSION claim 3) SPA substantiation — see audit `docs/skycot/ux-audits/2026-05-25-1100-whole-site-mvp-claims-coverage.md` finding "Claim 3: NOT SUBSTANTIATED in SPA — `/api/v1/genesis = 404`".

🔴 **P0 MISSION CLAIM 3 SUBSTANTIATION — investor-narrative load-bearing.**

Per Agent-5 whole-site audit: `/landing` HEADLINES "Describe the job in plain English" as the Skycot product promise, but `curl /api/v1/genesis = 404`. No SPA surface fires Genesis. Onboarding step 4 (`/onboarding/pick-vertical`) presents 5 prefab roles, not Genesis-from-English. **The product's most-prominent marketing claim has no implementation a user can reach.**

The `argo-genesis` crate already exists + ships a working `plan + write_vertical` pipeline (per Phase-A round-3 audit + the existing `eval-genesis-pipeline-end-to-end-smoke` brief — argo-genesis is at ~8881 LOC + has 20+ pub modules). The gap is the GATEWAY ROUTE: argo-genesis is not exposed via an HTTP endpoint the SPA can call.

## Scope

Add a gateway route `POST /api/v1/genesis` that:

1. Accepts a JSON body: `{ "role_description": "plain English", "tenant_id"?: TenantId, "context"?: { sandbox: bool, sample_kpis: bool } }`.
2. Streams Server-Sent Events (SSE — a one-way push protocol over HTTP — used so the SPA can render live progress without polling) responses:
   - `event: progress` payloads — `{ phase: "drafting_persona" | "compiling_charter" | "generating_skill_bundle" | "validating_eval_seeds" | "ready", message, percent }`
   - `event: vertical_ready` payload (final): `{ vertical_dir_path, manifest, prompts: [], skills: [], eval_seed_count, judge_verdict }`
   - `event: error` payload (on fail): `{ phase, message, recoverable: bool }`
3. Tenant-scoped: every generated vertical is bound to the caller's tenant_id; cross-tenant isolation tested.
4. Cost guardrail: same as `mission-demo-seed-real-llm-driver` — require `ARGO_GENESIS_PROVIDER_KEY` env var; fail closed if unset; never fall back to canned responses.

## Acceptance criteria

- [ ] `POST /api/v1/genesis` with `{"role_description": "I need a customer success lead who triages tier-1 tickets and escalates refunds over $500"}` returns 200 + SSE stream + final `vertical_ready` event within ~30-60s.
- [ ] Generated vertical round-trips through `argo_vertical::load_dir` (per the existing `eval-genesis-output-loads-cleanly-through-argo-vertical-v2` brief pattern).
- [ ] Cross-tenant negative test: tenant A's generated vertical NOT visible to tenant B's `GET /api/v1/verticals`.
- [ ] PR body includes a 20-line SSE-trace sample showing progress events + final vertical_ready.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_genesis.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch.rs` (narrow: register the route)
- `crates/argo-cli/src/handlers/gateway/boot_serve.rs` (narrow: SSE middleware if not already present)
- `crates/argo-cli/tests/genesis_route_e2e.rs` (NEW)

## Out of scope

- The SPA-side `/genesis` page (separate brief: `mission-genesis-spa-page-describe-and-watch`)
- Onboarding integration (separate brief: `mission-genesis-onboarding-pick-vertical-add-describe-own`)
- Touching `argo-genesis` internals (consume existing public API only)

## Effort

~300-500 LOC (gateway route + SSE plumbing + cross-tenant test + e2e). ~3-5 hours.
