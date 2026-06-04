# SKYCOT Day-0 Gap-Audit — POC → Public Self-Serve SaaS + Enterprise Tier

**Date:** 2026-06-04 · **Basis:** read-only against `~/code/argo`,
live gateway probed on `:7400` (schema v87, build `da5093e`, healthy).
**Author:** Day-0 gap-audit agent (READ-ONLY), synthesised by claude conductor.
**Companion playbook:** [`docs/skycot/SPRINT-paying-ready-2026-06-04.md`](../SPRINT-paying-ready-2026-06-04.md).

> This doc is the evidence base for the **SPRINT TO PAYING-READY** queue in
> `docs/build-plan/BUILD_STATUS.md`. Tier-1/Tier-2 briefs were authored Day-0;
> **Tier-3/4/5 briefs are authored from THIS doc as the swarm lanes free up**
> (conductor Step-8 replenishment each tick). All "works" claims cite file/route
> evidence; anything unverified is flagged in the caveats section.

## Headline finding

This is **much further along than "a POC with mock surfaces."** Magic-link auth,
per-tenant provisioning, a full 3-tier Stripe billing system, k8s/Docker deploy
manifests, and a rich gateway (100+ routes) all EXIST. **The single biggest
blocker is a tenant-resolution architecture split** (WS1) that makes every user
share one workspace. Fix that and most of the rest is wiring, not building.

---

## WS1 — Real front door + per-user tenant (THE foundation)

**Works:**
- Magic-link signup + verify is LIVE. `POST /api/v1/auth/signup` + `POST /api/v1/auth/verify` mounted. Signup mints a single-use hashed token (`auth_magic_link_signup.rs`, table `magic_link_tokens` migration 115) with IP rate-limiting + Turnstile/honeypot (`signup_protection.rs`). Verify consumes the token, upserts a `user_account`, sets `__Host-session` JWT + CSRF double-submit cookies (`auth_magic_link_verify.rs`).
- Per-user tenant provisioning EXISTS and is correct: `argo_identity::tenant_provision::provision_for_user` idempotently creates a `tenant` + `workspace` + `user_tenant` link (`role=owner`) on first call. Mounted as `POST /api/v1/onboarding/provision`. Verify returns `needs_onboarding: true` when a user has no tenant (`auth_magic_link_verify.rs:108-128`).
- Production auth is real: `gateway.auth_bypass` forced off in production mode; `ARGO_GATEWAY_BEARER_TOKEN` required for non-loopback bind (`routes_auth.rs:58-87`, `routes_auth_production_mode.rs`).

**Missing / broken — the critical architecture split (the "ghost worker" bug):**
- Two parallel tenant-resolution systems that don't connect (documented in-code at `routes_onboarding_helpers.rs:19-40`, `routes_onboarding.rs:286-294`):
  1. **User-bound tenant** (from `provision_for_user`): `root_path = tenant://<id>/<ws>` — the correct per-user tenant.
  2. **CWD tenant** (`ensure_workspace(std::env::current_dir())`): the gateway process's working directory. **59 gateway route files** resolve tenant this way (`resolve_workspace_tenant` duplicated 10+ times — `usage_endpoint.rs:64`, `routes_team_list.rs`, `routes_cohorts.rs`, `routes_worker_*.rs`, etc.).
- Consequence: every read surface (`/api/v1/team`, `/workers`, `/workers/{id}/*`, `/cohorts`, `/usage`) ignores the signed-in user's JWT tenant and queries the ONE cwd tenant. `first-hire` deliberately writes to the cwd tenant too (`routes_onboarding.rs:298`, `cwd_write_tenant`) so reads can see the worker — meaning **all users currently share the single cwd workspace, which is the seeded demo tenant.** A stranger signing up lands in shared demo data, not an empty private workspace.
- No route reads tenant from the session JWT. `routes_auth_principal.rs` carries the principal only into audit payloads, not into tenant scoping. The JWT mint path threads `tenant_id` (`session_jwt.rs`), so the plumbing exists at the edge — it just isn't consumed by the data routes.

**Briefs:** WS1-1 (keystone) .. WS1-5. WS1-1 is the critical path for the whole sprint; WS1-2/3/4/5 depend on it.

---

## WS2 — The platform loop in a FRESH tenant (Genesis → hire → real work → approval → audit)

**Works:**
- Genesis is a full subsystem: crate `argo-genesis` + ~20 gateway handler files. Live: `POST /api/v1/genesis/classify`, `/genesis/draft-worker`, SSE streaming (`routes_genesis_stream*.rs`), a fail-closed safety gate, rubric/comprehension, SOP ingestion, and flag-gated LLM-authoring (`author.rs`, commits `c74da4bf8`/`f5674f559`/`da5093e96`).
- First-hire wires Genesis → live worker: `POST /api/v1/onboarding/first-hire` calls `argo_genesis::plan`, enforces the safety gate before activation, with a deterministic fallback (`routes_onboarding_genesis.rs`, `routes_onboarding.rs:335`).
- Executors are real: `argo-executor-computer-use-macos` posts real `CGEvent`s + AXUIElement calls with a permission preflight; plus document / CSV / gsuite / filesystem / Linux / Windows / generic CU executors.
- Workflow execution is a real run, audited: `POST /api/v1/verticals/<v>/workflows/<n>/execute` + `/verticals/<id>/run` create a session, attach the skill bundle, run via `vertical_workflow::run_collect` with every tool_call landing in the hash-chained audit feed (`routes_verticals_run.rs:101-168`).
- Approval-first + audit live: `POST /api/v1/approvals/<id>/decide`, `GET /api/v1/audit`, `/audit/<id>`, `/audit/sessions/<id>/verify` (hash-chain verify).

**Missing / silently depends on seeded data:**
- The whole loop runs against the cwd/demo tenant (inherits the WS1 split). In a genuinely fresh user tenant, data wouldn't surface on read pages until WS1-1 lands.
- Workflow execute defaults to `provider: "mock"`. Real provider work needs a per-tenant provider key. `POST /api/v1/onboarding/provider-key` exists + encrypts with a per-tenant master key (`wrap_secret` → `MasterKey::resolve_for_tenant`), but validation is shape-only (`validate_key_shape`, no live token check).
- Empty-state dependence: `/team` hero cards read "No activity yet" on an empty tenant (the known Gap 3; commit `c87629786` seeds activity for the *demo* tenant only). A fresh tenant has no skills/verticals/workflows — onboarding must drive those or pages render empty.
- MasterKey hard dependency: credential storage fails closed if `SKYCOT_WORKSPACE_ROOT_KEY_BASE64`/`ARGO_CREDENTIALS_MASTER_KEY` isn't set — a public deploy MUST provision it or every provider-key save 503s.

**Briefs:** WS2-1 .. WS2-4.

---

## WS3 — De-mock the SPA

**Works — the gating is SAFER than feared:**
- Mock fallbacks gate on `isGatewayAuthBypassed()` = `NODE_ENV !== "production" && NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"` (`web/lib/skycot/demo-mode.ts`). **A production build (`NODE_ENV=production`) automatically ignores the bypass and calls real endpoints** — even a hostile build setting the env var is overridden. So mock data does NOT leak to production users by default.
- Most fetchers already have the live path: of ~40 `web/lib/api/*.ts` modules, the majority carry 3-9 `isDemoLive`/bypass gates AND a real `apiGet`/`apiPost`. Live endpoints confirmed for: workers, worker overview/activity/memory/skills/trust/policies/settings, team, cohorts, audit (+detail/export/replay), approvals (+bundles), skills (+marketplace/IR/usage), workflows (+runs), notifications, evals, knowledge, support tickets, orchestration runs, tenant/usage, provider-keys, SSO/SCIM, me/export.

**Missing / mock / needs a backend:**
- No live endpoint at all (need backend built): `retros.ts`, `retrain.ts`/`retrain-mocks.ts`/`retrain-types.ts`, `workflow-metadata.ts` + `workflows-save-publish.ts` (no save/publish route — only execute/list/runs exist), `duties.ts` (no `/api/v1/duties` mounted; duties seeded via hire only).
- Endpoint exists, gate just needs flipping / empty-state polish: `orchestration.ts` (gate=0 but `/orchestration/runs` is live), `worker-trust.ts`/`workers.ts`/`workflow-runs.ts`/`workplace-knowledge.ts` (high sample-ref counts; endpoints exist — need the prod no-bypass path verified + brand-voiced empty states).
- `web/lib/sample-*.ts` are the fixture source; correctly fenced behind the dev bypass, but every consuming page needs an empty-state that isn't a blank screen.

**Briefs:** WS3-1 .. WS3-4.

---

## WS4 — Get paid (billing) — MUCH more complete than assumed

**Works:**
- A real Stripe billing crate exists + is wired: `crates/argo-billing/` (`checkout.rs`, `portal.rs`, `webhook.rs`, `tier_store.rs`, `stripe_client.rs`, `replay.rs` DLQ recovery).
- Live routes: `POST /api/v1/billing/checkout`, `/billing/portal`, `/billing/webhook` (webhook sits ahead of the bearer gate, authenticates via `Stripe-Signature` HMAC).
- Full 3-tier pricing config (`docs/skycot/pricing/v1-tiers.toml`, compiled via `argo_identity::tier::TierConfig`): Free ($0, 1 worker, 100 actions/mo, mock provider only), Pro ($29, 5 workers, 5k actions/mo, real keys + channels), Enterprise (custom, unlimited, SSO + audit-export + custom-verticals).
- Quota enforcement live: `GET /api/v1/tenant/usage` reads `workspace.tier` + `tenant_usage` vs `max_actions_per_month` (`usage_endpoint.rs`); `quota_middleware.rs` exists.
- Frontend wired: `web/lib/billing/use-checkout.ts`, `use-portal.ts`; `web/app/pricing/page.tsx`; operator CLI `argo billing replay`/`dlq`. Idempotent webhook with `has_processed_event` dedup + DLQ.

**Missing for self-serve pay + enterprise gate:**
- Stripe is config-gated, not provisioned: `StripeClient::from_env()` needs `STRIPE_SECRET_KEY` + price IDs + webhook signing secret in the deploy env — none wired into AWS/prod yet.
- Tier→capability enforcement gap: the tier defines feature flags (`provider_real_keys`, `sso`, `channel_voice`, …) but it is NOT verified that these flags actually gate the runtime features (e.g. does Free really block real provider keys / extra workers?). `max_workers` enforcement at hire time unverified. **FLAGGED: not verified.**
- Pricing→checkout CTA not connected for self-serve: pricing buttons call `persistPlan(tier)` only (`pricing/page.tsx:255`); the Pro CTA does not invoke `useCheckout()`. Enterprise CTA is a `mailto:` (`pricing/page.tsx:106-108`) — no EOI persistence.

**Briefs:** WS4-1 .. WS4-4.

---

## WS5 — Go public on AWS

**Works:**
- Containerization done: root `Dockerfile` (multi-stage, distroless, single `argo` binary, `gateway start --bind 0.0.0.0 --port 8080`), `deploy/docker/Dockerfile.argo` + `Dockerfile.web`, `docker-compose.prod.yml`.
- Full k8s manifests: `deploy/k8s/base/` (deployment, service, ingress, PVC, configmap, secret.example, kustomize overlays). README lists EKS as a target.
- Observability shipped: Grafana dashboards (audit-chain, provider-spend, action-dispatch, overview), Prometheus, alertmanager rules, `GET /openmetrics`.
- DNS-as-code: `deploy/dns/cloudflare-records.tf`, `deploy/flyio/`, `web/vercel.json` pointed at `api.skycot.com` / `app.skycot.com`. Health probes `/health{,/live,/ready}`, `/livez`, `/readyz` live.
- Per-tenant rate-limiting + abuse guards exist: `argo-mcp/src/rate_limit.rs`, signup IP limiter (`rate_limit_ip.rs`), verify rate-limit, Turnstile + honeypot.

**Missing for AWS + anonymous public traffic:**
- No AWS-native IaC. Zero AWS/ECS/Fargate/App-Runner/EKS Terraform or CloudFormation. The k8s manifests are cloud-agnostic and could deploy to EKS, but there's no cluster IaC, no ALB ingress config, no RDS/EBS/ACM wiring.
- SQLite single-binary = single-writer. Fine for a single instance behind one EBS volume; does NOT horizontally scale (k8s deployment is `replicas: 1`, `strategy: Recreate`). Acceptable for launch; flag for scale.
- TLS termination, secrets, public bind need an AWS story: ACM cert + ALB or Cloudflare-in-front; `ARGO_GATEWAY_BEARER_TOKEN`, `STRIPE_*`, `SKYCOT_WORKSPACE_ROOT_KEY_BASE64`, provider keys all need a secrets manager.
- LLM-cost controls for anonymous traffic: Free-tier defaults to mock provider (good), but Genesis LLM-authoring + voice/pitch use the operator's keys. Per-tenant action quota exists, but a global per-IP / per-anon-tenant LLM-spend cap is unverified.

**Briefs:** WS5-1 .. WS5-4.

---

## WS6 — Funnel (landing + waitlist + enterprise EOI)

**Works:**
- Landing exists: `web/app/landing/page.tsx` + `page.css` + OG image + tests.
- Waitlist frontend built + honest: `web/components/marketing/WaitlistForm.tsx` + `web/lib/api/waitlist.ts`. Per NO-THEATER, it does NOT fake success — it surfaces the real 404 as an `ApiError`.
- Contact + pricing pages exist: `web/app/contact/page.tsx`, `web/app/pricing/page.tsx`, `web/app/for/sales/`.

**Missing / broken:**
- `POST /api/v1/waitlist` does NOT exist — confirmed two ways: live probe returns `{"error":"not found"}`, no `routes_waitlist*.rs` handler. The frontend documents the exact brief (`waitlist.ts:1-30`): persist to `waitlist_signups`, public anonymous rate-limit, dedupe on email, 202 `{queued:true}`.
- No enterprise EOI/MOU persistence. Enterprise interest is `mailto:` only. No structured capture, no CRM/email forward, no audit row.

**Briefs:** WS6-1 .. WS6-3.

---

## Truth-bounded caveats

- **Verified this turn:** live gateway routes (via `/routes` + `/health`), waitlist 404, all cited Rust source files (read directly), pricing/contact CTAs, demo-mode gate semantics, the tenant-resolution split (read in 3 files + the in-code "ghost worker" comments).
- **NOT verified this turn (flagged inline):** (1) tier feature-flag → runtime capability enforcement (WS4-3); (2) global anonymous LLM-spend cap (WS5-4); (3) computer-use macOS executor *executing* (relied on CLAUDE.md historical claim-5 cold-pass, not re-run); (4) whether `vertical_workflow::run_collect` invokes a real LLM vs deterministic steps with a real provider key (read the dispatch, not a live run).
- **Scope note:** did not exhaustively diff all ~40 `web/lib/api` modules line-by-line; WS3 briefs target the modules with `demoLiveGate=0` + sample refs (highest-risk-for-empty-UX in a real session).

---

## Full prioritized brief backlog (the SPRINT TO PAYING-READY queue)

Critical-path order: WS1 + WS2 first (the foundation), then WS3/WS6 (de-mock + funnel), then WS4/WS5 (pay + ship).

### Tier 1 — Foundation (unblocks everything)
- **WS1-1** `sprint-ws1-tenant-resolve-from-session-jwt-not-cwd` — L — backend — central helper that reads the user-bound tenant from the `__Host-session` JWT; reconcile the `tenant://` root_path convention so reads+writes agree; migrate the core read routes. KEYSTONE.
- **WS1-2** `sprint-ws1-wire-onboarding-writes-to-user-tenant` — M — backend — first-hire/install-vertical/provider-key write to the provisioned user tenant, not `cwd_write_tenant`. (dep: WS1-1)
- **WS1-3** `sprint-ws1-auto-provision-tenant-on-verify-or-gate-onboarding` — M — backend — call `provision_for_user` on verify when `needs_onboarding`, or force `POST /onboarding/provision` before any data page. (dep: WS1-1)
- **WS1-4** `sprint-ws1-cross-tenant-isolation-test-jwt-path` — S — backend — regression test (mirror `seed_cross_tenant_invisibility.rs`) that user A's JWT can't read user B's workers/audit/skills. (dep: WS1-1)
- **WS1-5** folded into WS1-1 (root_path `tenant://` reconciliation is part of the keystone).

### Tier 2 — Platform loop in a fresh tenant
- **WS2-1** `sprint-ws2-fresh-tenant-seeds-usable-baseline` — M — backend — on provision, install a default skill set + ≥1 vertical so /skills, /workflows, /team aren't empty day one. (dep: WS1-1)
- **WS2-2** `sprint-ws2-brand-voiced-empty-states-fresh-tenant` — M — frontend — real onboarding empty states (not "No activity yet"/blank) on /team, /workers, /skills, /workflows, /audit, /approvals. (parallel — independent UI)
- **WS2-3** `sprint-ws2-live-provider-key-validation` — S — backend — replace shape-only `validate_key_shape` with a real one-token validation call.
- **WS2-4** `sprint-ws2-provision-master-key-in-deploy-env` — S — infra — wire `SKYCOT_WORKSPACE_ROOT_KEY_BASE64`/`ARGO_CREDENTIALS_MASTER_KEY` into AWS secrets so provider-key save doesn't 503.

### Tier 3 — De-mock + funnel
- **WS6-1** `sprint-ws6-waitlist-backend-endpoint` — S — backend — `routes_waitlist.rs`: public/anon, rate-limit + bot guard, persist `waitlist_signups` (additive migration), dedupe on email, idempotent 202. (independent — quick win)
- **WS6-2** `sprint-ws6-enterprise-eoi-capture-endpoint-and-form` — M — backend+frontend — `POST /api/v1/enterprise-inquiry` persisted + audited; form on /pricing + /contact + /for/sales.
- **WS6-3** `sprint-ws6-funnel-e2e-on-prod-build` — S — frontend — after WS6-1, confirm WaitlistForm shows real success + landing CTAs route to /auth/signup; `NEXT_PUBLIC_SKYCOT_DEMO_*` OFF for public build.
- **WS3-1** `sprint-ws3-retrain-retro-duties-endpoints-or-hide` — M — backend — build POST routes for retrain/retros/duties or feature-flag the pages off public nav until real.
- **WS3-2** `sprint-ws3-workflow-save-publish-endpoints` — M — backend — `workflows-save-publish.ts` + `workflow-metadata.ts` need save/publish routes so the builder persists user-authored workflows in their own tenant.
- **WS3-3** `sprint-ws3-verify-prod-no-bypass-fetchers` — S — frontend — audit-confirm orchestration.ts/workers.ts/worker-trust.ts/workflow-runs.ts call live endpoints under `NODE_ENV=production`, no sample data.
- **WS3-4** `sprint-ws3-tree-shake-sample-fixtures-from-public-bundle` — S — frontend — ensure `web/lib/sample-*.ts` + `demo-roster-store.ts` are guarded/tree-shaken so no fixture ships to public users even if a flag is misconfigured.

### Tier 4 — Get paid
- **WS4-1** `sprint-ws4-pricing-pro-cta-to-stripe-checkout` — S — frontend — wire the Pro CTA to `useCheckout()` (→ `/api/v1/billing/checkout`, already live). (parallel — independent)
- **WS4-2** `sprint-ws4-provision-stripe-env-prod` — S — infra — `STRIPE_SECRET_KEY` + per-tier price IDs + `STRIPE_WEBHOOK_SECRET` in AWS secrets; create Stripe products matching `v1-tiers.toml` slugs.
- **WS4-3** `sprint-ws4-enforce-tier-capability-gates` — M — backend — verify+wire that Free blocks real provider keys / >1 worker / paid channels per `v1-tiers.toml`. (FLAGGED unverified in audit)
- **WS4-4** `sprint-ws4-self-serve-billing-page-portal-usage-meter` — S — frontend — surface `GET /api/v1/tenant/usage` as a meter + the Stripe portal on a /billing page.

### Tier 5 — Go public on AWS
- **WS5-1** `sprint-ws5-aws-iac-single-binary-gateway` — L — infra — `infra/terraform`: ECS Fargate or EKS for the argo container + an EBS/EFS volume for the SQLite store (single-writer), ALB + ACM TLS at api.skycot.com.
- **WS5-2** `sprint-ws5-deploy-spa-to-app-skycot-com` — S — infra — confirm Vercel deploy (web/vercel.json → api.skycot.com) + DNS + `NEXT_PUBLIC_*` prod env. (smallest path: SPA on Vercel, gateway on AWS).
- **WS5-3** `sprint-ws5-prod-secrets-bearer-token-bind-hardening` — S — infra — wire bearer token + master key + Stripe/provider secrets into AWS Secrets Manager; confirm `ARGO_GATEWAY_PROFILE=production` forces auth_bypass off.
- **WS5-4** `sprint-ws5-global-anon-abuse-and-llm-cost-cap` — M — backend+infra — global per-IP / per-anon-tenant LLM-spend ceiling + WAF/Cloudflare rules; verify Free tier truly stays on mock provider.
