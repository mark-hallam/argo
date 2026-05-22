# Cross-tenant endpoint coverage audit

**Generated**: 2026-05-22 against `origin/main` `917290b98`.
**Brief**: `mission-cross-tenant-audit-sweep` (P2 SECURITY HYGIENE, tick-41).
**Source script**: `scripts/audit-cross-tenant-coverage.sh` (idempotent;
re-run regenerates the test-file flag table).
**Method**: see "Heuristic + known limits" below.

## Summary

- **88** gateway endpoint families enumerated across
  `crates/argo-cli/src/handlers/gateway/` (one row per coherent handler
  module — e.g. `routes_worker_memory` covers the four worker-memory
  HTTP verbs as one family because they share the same test file).
- **9** endpoint families have a same-file cross-tenant negative test
  in their sibling `*_tests.rs`. 7 additional CLI-integration tests
  carry cross-tenant assertions but exercise CLI/store paths rather
  than HTTP handlers.
- **20** tenant-scoped endpoint families have **NO** cross-tenant test
  in any reachable test file — see "Gap inventory" below.
- **59** endpoint families are tenant-agnostic by design (auth,
  health, billing webhook HMAC, peer mesh, SSO callbacks, marketplace
  catalog browsing, dashboards aggregating across tenants) — cross-
  tenant tests are not applicable.

The follow-up briefs target the **3 highest-data-exposure gaps**:

1. `mission-cross-tenant-test-audit-export` — `POST /api/v1/audit/export`
   (signed bundle download covering an arbitrary `[from, to]` window).
2. `mission-cross-tenant-test-worker-memory` — `GET|POST|PUT|DELETE
   /api/v1/workers/<id>/memory{,/facts/<fact_id>}` (worker memory
   facts; can contain PII / contact data).
3. `mission-cross-tenant-test-workspace-members` — `GET
   /api/v1/workspace/members` + invite/accept companions (IAM-grade —
   workspace member list + pending invitations).

> **Next conductor action**: insert TODO rows for the 3 follow-up
> briefs into `docs/build-plan/BUILD_STATUS.md` in the next
> replenishment commit.

## Heuristic + known limits

The script in `scripts/audit-cross-tenant-coverage.sh` flags a test
file as having a cross-tenant negative test when **all three** of:

1. Mentions `tenant_a` / `tenant-a` / `"t1"` somewhere in the file.
2. Mentions `tenant_b` / `tenant-b` / `"t2"` somewhere in the file.
3. Mentions one of `leak`, `403`, `Forbidden`, `excluded`,
   `isolation`, `cross-tenant`, `cross_tenant`, `other_tenant`,
   `other-tenant`.

This is intentionally generous on (1)+(2) and strict on (3) so the
report errs on the side of false negatives (the costly direction —
the brief states "false negatives are more costly than false
positives"). Known limits:

- **Random `TenantId::new()` patterns** (e.g.
  `tests/audit_cross_tenant_smoke.rs`) seed two tenants from random
  UUIDs rather than the literal `tenant-a` / `tenant-b` — the heuristic
  flags these as NO. They are reclassified by manual review in the
  table below (column: *Cross-tenant test? (manual)*).
- **Shared helper modules** can hold the seed/assert pair in a
  helper that the test file `use`s — the heuristic does not chase
  re-exports. None observed in this sweep, but the manual review
  column carries the same flag if discovered.
- **CLI integration tests** in `crates/argo-cli/tests/` are scanned
  by the script but most do not drive `/api/v1/` HTTP handlers — they
  cover CLI subcommands or store-layer paths. They count toward
  defense-in-depth but cannot substitute for a same-file gateway
  handler test, so the gap inventory does not credit them.
- The script does NOT prove the matched fn actually asserts
  isolation — only that the file contains the marker words. Spot-
  checks of all 9 YES gateway files confirmed real coverage in this
  audit; the conductor should re-spot-check on re-runs.

## Endpoint coverage

One row per **handler family** (handler module file). Risk class
heuristic:

- **HIGH** — endpoint returns or mutates rows that belong to a tenant
  identity (workers, skills, audit, identity, session contents,
  approvals, memory, retros, notifications, billing rows, costs,
  workspace members, scim provisioning, evidence bundles).
- **MED** — endpoint returns aggregate / KPI / config data scoped by
  tenant where leakage would expose strategic shape but not raw
  per-row PII.
- **LOW** — endpoint is anonymous-by-design (auth signup/verify,
  CSRF token, health, version, marketplace catalog), bounded by a
  separate HMAC (billing webhook, audit export download), or
  cross-tenant by intent (admin fleet dashboard, peer mesh).

### Endpoints WITH a cross-tenant test (✅)

| Endpoint family | Handler file | Test file | Risk |
|---|---|---|---|
| `GET /api/v1/audit` (feed) | `routes_audit_feed.rs` | `routes_audit_feed_tenant_tests.rs` | HIGH |
| `GET /api/v1/audit/<id>` (detail) | `routes_audit_detail.rs` | `routes_audit_detail_tests.rs` | HIGH |
| `POST /api/v1/audit/<id>/replay` | `routes_audit_replay.rs` | `routes_audit_replay_tests.rs` | HIGH |
| `GET/POST /api/v1/notifications*` | `routes_notifications.rs` | `routes_notifications_tests.rs` | HIGH |
| `GET /api/v1/retros` | `routes_retros.rs` | `routes_retros_tests.rs` | HIGH |
| `GET /api/v1/skills/<id>/ir` | `routes_skill_ir.rs` | `routes_skill_ir_tests.rs` | HIGH |
| `POST /api/v1/workers/<id>/freeze` | `routes_worker_freeze.rs` | `routes_worker_freeze_tests.rs` | HIGH |
| `GET /api/v1/workers/<id>/trust-grade` | `routes_worker_trust_grade.rs` | `routes_worker_trust_grade_tests.rs` | HIGH |
| per-route rate limiter (unit) | `rate_limit_tenant.rs` | `rate_limit_tenant_tests.rs` | MED |

CLI-integration tests with cross-tenant assertions (defense-in-depth,
not handler-level coverage):

- `tests/action_history_smoke.rs` (CLI `action recent --tenant`)
- `tests/audit_chain_replay_smoke.rs`
- `tests/audit_cross_tenant_smoke.rs` (random-UUID two-tenant —
  manual-review YES; script flags NO)
- `tests/cross_tenant_skill_discovery_negative.rs`
- `tests/gateway_tenant_rate_limit_smoke.rs`
- `tests/multi_tenant_isolation_load.rs`
- `tests/skill_loop_closure_smoke.rs`
- `tests/skill_runs_smoke.rs`

### Endpoints WITHOUT a cross-tenant test (❌, tenant-scoped → gap)

Sorted by risk class (HIGH first). The "Recommend" column is the
shape of the test that would close the gap.

| Endpoint family | Handler file | Sibling test file | Risk | Recommend |
|---|---|---|---|---|
| `POST /api/v1/audit/export` (+ preview + signed GET) | `routes_audit_export.rs` | `routes_audit_export_tests.rs` | **HIGH** | Tenant A POSTs export; tenant B requests overlapping window — bundle contains zero tenant-A rows. **Top-1 follow-up.** |
| `GET|POST|PUT|DELETE /api/v1/workers/<id>/memory{,/facts/<fact_id>}` | `routes_worker_memory.rs` | `routes_worker_memory_tests.rs` | **HIGH** | Tenant B GET on tenant-A worker memory → 404 (handler scopes `resolve_worker_ctx`); same for POST/PUT/DELETE. **Top-2 follow-up.** |
| `GET /api/v1/workspace/members` (+ invite, accept) | `routes_workspace_members.rs` + `_invite.rs` + `_accept.rs` | `routes_workspace_members_tests.rs` | **HIGH** | Tenant B caller GET → only B's members; invite from B for an email already pending in A → no leak. **Top-3 follow-up.** |
| `POST /api/v1/workers` (hire) | `routes_workers_hire.rs` | `routes_workers_hire_tests.rs` | HIGH | Caller in tenant B cannot create a worker that lands under tenant A's scope. |
| `GET/POST /api/v1/workers/<id>/settings` + `PATCH .../settings/<facet>` | `routes_worker_settings.rs` | `routes_worker_settings_tests.rs` | HIGH | Settings include policy thresholds + sensitive config; tenant B cannot read/write tenant A's worker settings. |
| `GET/POST/DELETE /api/v1/workers/<id>/skills` | `routes_worker_skills.rs` | `routes_worker_skills_tests.rs` | HIGH | Tenant B cannot add or remove skills on tenant A worker (active `web-cohort-skill-removal` brief adds the DELETE invariant — that brief covers this gap). |
| `GET /api/v1/workers/<id>/overview` | `routes_worker_overview.rs` | `routes_worker_overview_tests*.rs` | HIGH | Tenant B GET on tenant-A worker → 404, no KPI bleed. |
| `GET /api/v1/workers/<id>/activity` | `routes_worker_activity.rs` | `routes_worker_activity_tests.rs` | HIGH | Activity log carries action payloads; isolation must be enforced. |
| `GET /api/v1/workers/<id>` (detail) | `routes_worker_detail.rs` | `routes_worker_detail_tests.rs` | HIGH | Cross-tenant detail fetch → 404. |
| `GET/PATCH /api/v1/workers/<id>/policies` | `routes_worker_policies.rs` | `routes_worker_policies_tests.rs` | HIGH | Policy thresholds drive auto-approval — cross-tenant write would be P0. |
| `POST /api/v1/audit` (record) | `routes_audit_record.rs` | `routes_audit_record_tests.rs` | HIGH | Hand-rolled `tenant_id` in body must be ignored — handler stamps caller tenant; verify a B-caller cannot mint an A-tenant row. |
| `GET /api/v1/approvals` | `routes_approvals_json.rs` | `routes_approvals_json_tests.rs` | HIGH | Approval queue per tenant; B cannot see A's pending approvals. |
| `GET/POST /api/v1/approvals/bundles*` | `routes_approval_bundles.rs` | `routes_approval_bundles_tests.rs` | HIGH | Bundles carry approval payloads; B cannot read A's bundle ids. |
| `GET /api/v1/cohorts` (+ `<id>`) | `routes_cohorts.rs` | `routes_cohorts_tests.rs` | HIGH | Cohort defs include skill assignments + per-worker policy; B cannot read A's cohort. |
| `POST /api/v1/audit/elevated-view-request` + `GET/DELETE .../elevated-view-grants*` | `routes_elevated_view.rs` | `routes_elevated_view_tests.rs` | HIGH | Elevated-view is the admin override channel — cross-tenant grant minting must 403. |
| `GET /api/v1/orchestration/runs` + `POST .../runs/<id>/...` | `routes_orchestrator.rs` + `_slas.rs` + `_handoffs.rs` + `_sessions.rs` | `routes_orchestrator_tests.rs` | HIGH | Orchestrator runs carry inter-worker handoffs; cross-tenant leak = strategy disclosure. |
| `POST /api/v1/workspace/members/invite` | `routes_workspace_members_invite.rs` | covered by `routes_workspace_members_tests.rs` follow-up | HIGH | Invite token must not be usable by another tenant's session. |
| `POST /api/v1/workspace/members/accept` | `routes_workspace_members_accept.rs` | covered by `routes_workspace_members_tests.rs` follow-up | HIGH | Accept-flow tenant binding must follow the invite, not the caller's existing tenant. |
| `GET/POST/PATCH /api/v1/support/tickets*` | `routes_support_tickets.rs` | `routes_support_tickets_tests.rs` | HIGH | Support tickets contain customer-side context; isolation required. |
| `GET /api/v1/team` | `routes_team_list.rs` | `routes_team_list_tests.rs` | HIGH | Team roster (IAM-adjacent) per tenant. |
| `GET/POST /api/v1/me/export*` | `routes_me_export.rs` | `routes_me_export_tests.rs` | HIGH | Own-data export — caller email must bind to own tenant; cannot be tricked into A's export. |
| `POST /api/v1/genesis/{classify,draft-worker}` | `routes_genesis.rs` | `routes_genesis_tests.rs` | MED | Draft-worker writes a tenant-scoped seed. Cross-tenant draft would be confusing not catastrophic, but worth a test. |
| `POST /api/v1/onboarding/{first-hire,install-vertical,provider-key,provision}` | `routes_onboarding.rs` + `onboarding_provision.rs` | `routes_onboarding_tests.rs` + `onboarding_provision_tests.rs` | MED | Tenant binding flows; if reusable by a second caller, could rebind. |
| `GET/POST /api/v1/wizard*` | `routes_wizard.rs` | _(none — `routes_dispatch_post_tests.rs` covers dispatch)_ | MED | Wizard state per tenant. |
| `GET /api/v1/dashboards/{error-rate,sla}` | `routes_dashboard.rs` + dispatch | _(none)_ | MED | Aggregate dashboards — cross-tenant numbers could leak shape. |
| `GET /api/v1/tenant/usage` | `usage_endpoint.rs` | `usage_endpoint_tests.rs` | MED | Usage numbers per tenant; B cannot read A. |
| SCIM users | `routes_scim_users.rs` + `_envelope.rs` | `routes_scim_users_tests.rs` | MED | SCIM token is per-tenant; verify tokens bind correctly. |
| `GET /api/v1/meta/error-reporting` | `routes_meta_error_reporting.rs` | _(none)_ | LOW-MED | Reporter config per tenant. |
| `GET /api/v1/workers/<id>/trust-streaks` | `routes_trust_streaks.rs` | `routes_trust_streaks_tests.rs` | MED | Trust streaks are per-worker; same shape as trust-grade (already covered). |
| Worker skill discovery via SkillGraph | covered by `routes_skill_ir.rs` ✅ | — | — | (Already has cross-tenant test.) |
| Auth logout | `routes_auth_logout.rs` | `routes_auth_logout_tests.rs` | LOW | Stateless cookie clear; cross-tenant is not the threat model. |
| Channels config (`/channels/discord|email|imessage|push|slack|voice|webinbox|whatsapp`) | `routes_channels*.rs` | `routes_channels_voice_tests.rs` (voice only) | MED | Channel configs are per-tenant; voice has the cross-tenant gap explicitly. |
| `POST /api/v1/skills/draft-from-prompt` | `routes_skills.rs` | `routes_skills_tests.rs` | MED | Draft skill bound to caller tenant. |
| `GET /api/v1/skills/quality*` | `routes_skills_quality.rs` | `routes_skills_quality_tests.rs` | MED | Quality metrics per skill per tenant. |
| Workflow defs/runs | `routes_workflows.rs` + `_handlers.rs` | `routes_workflows_tests.rs` | HIGH | Workflow JSON defines runtime behavior; cross-tenant read/write would be P1. |
| Duties scheduler | `routes_duties.rs` + `_helpers.rs` | `routes_duties_tests.rs` | MED | Duty rotations per tenant. |
| Retrain | `routes_retrain.rs` + `_helpers.rs` | `routes_retrain_tests*.rs` | MED | Retrain inputs include feedback rows; isolation required. |

### Endpoints with NO cross-tenant test by design (LOW)

These endpoints either run anonymously (auth bootstrap, marketplace
catalog), are bounded by an independent HMAC (billing webhook,
signed export download), or aggregate across tenants by intent
(admin fleet dashboard, peer mesh). They are listed for completeness;
no follow-up brief is recommended.

| Endpoint family | Handler file | Reason |
|---|---|---|
| `POST /api/v1/auth/signup` | `auth_magic_link_signup.rs` | Anonymous; rate-limit + signup-protection cover the threat. |
| `POST /api/v1/auth/verify`, `GET /api/v1/auth/verify` | `auth_magic_link_verify.rs` | Anonymous; token binds tenant. |
| `GET /api/v1/auth/csrf` | `routes_auth_csrf.rs` | Anonymous double-submit token mint. |
| `POST /api/v1/billing/webhook` | `billing.rs` | Stripe HMAC over body — not session-scoped. |
| `POST /api/v1/billing/{checkout,portal}` | `billing.rs` | Bearer-gated + binds session tenant; cross-tenant write surface is the customer-id lookup which is per-tenant by construction. |
| Marketplace catalog GETs | `marketplace.rs` + `marketplace_render.rs` | Anonymous browsing surface. |
| `marketplace_authz_tests.rs` install authz | `marketplace.rs` | Install path is authz-gated; existing tests cover bearer/JWT acceptance. |
| `peer/*` mesh | `routes_peer.rs` | Own bearer secret (`ARGO_PEER_SECRET`); cross-tenant is the point of mesh sync. |
| SSO callbacks (OIDC, SAML, ACS, XMLSIG) | `sso_*.rs` | Identity-provider bound; tenant resolved from IdP response. |
| Dashboard fleet | `dashboard_fleet.rs` | Admin global view by intent. |
| Health, version, routes catalog | `routes_health.rs` + admin dispatch | Anonymous liveness. |
| OpenMetrics, metrics JSON/text | `routes_openmetrics.rs` + `metrics*.rs` | Operator-side metrics; aggregate. |
| WS upgrade + WS inbox/skill-trace | `ws_*.rs` | WS auth gate covers cross-tenant subscribe. |
| Streaming helper | `streaming.rs` | Plumbing. |
| All middleware (`middleware_*`, `rate_limit_*`, `role_gate`, `quota_middleware`, `proxy_guard`, `security_headers`, `tier_capability_filter`, `signup_protection`, `request_audit`, `region_gate`, `maintenance_middleware`, `ip_allowlist`, `hipaa_startup_check`, `service_token_binding_sweeper`, `approval_expiry_sweeper`, `approval_reminder`, `email_outbound`) | various | Middleware/sweepers — not request handlers; tested independently. |
| JWT, session_jwt | `jwt.rs`, `session_jwt.rs` | Auth primitives — covered by their own tests. |
| Boot / boot_router / boot_bind | `boot*.rs` | Startup. |
| Maintenance, lifecycle | `lifecycle.rs`, `maintenance_middleware.rs` | Op surfaces. |
| Bootstrap token (wizard) | `bootstrap_token.rs` | One-shot install token. |

## Gap inventory (HIGH risk, no test)

The HIGH-risk gaps above number **17**. The 3 highest-data-exposure
gaps get follow-up briefs in this PR; the remaining 14 are tracked
inline in the table above for a future replenishment wave to pick
up. They are ordered roughly by data-exposure severity:

| Rank | Endpoint | Why HIGH |
|---|---|---|
| 1 | `POST /api/v1/audit/export` | Signed download URL — one cross-tenant leak = full audit log dump. → `mission-cross-tenant-test-audit-export` |
| 2 | `/api/v1/workers/<id>/memory*` | Memory facts can contain PII / customer data. → `mission-cross-tenant-test-worker-memory` |
| 3 | `/api/v1/workspace/members*` | IAM-grade — list + invite + accept. → `mission-cross-tenant-test-workspace-members` |
| 4 | `POST /api/v1/workers` (hire) | Worker creation under wrong tenant = identity confusion. |
| 5 | `/api/v1/workers/<id>/settings` | Settings include policy thresholds + sensitive config. |
| 6 | `POST /api/v1/audit` (record) | Hand-rolled body could attempt to mint cross-tenant row. |
| 7 | `/api/v1/approvals` + `/approvals/bundles` | Pending approvals carry sensitive payloads. |
| 8 | `/api/v1/cohorts` | Cohort defs include skill+policy. |
| 9 | `/api/v1/audit/elevated-view-*` | Admin override channel. |
| 10 | `/api/v1/orchestration/runs` | Orchestrator handoffs disclose strategy. |
| 11 | `/api/v1/workers/<id>/overview` | KPI bleed. |
| 12 | `/api/v1/workers/<id>/activity` | Action payloads. |
| 13 | `/api/v1/workers/<id>` (detail) | Worker card contents. |
| 14 | `/api/v1/workers/<id>/policies` | Policy thresholds. |
| 15 | `/api/v1/workflows*` | Workflow JSON = runtime behavior. |
| 16 | `/api/v1/me/export` | Own-data export coercion. |
| 17 | `/api/v1/team`, `/api/v1/support/tickets` | Roster + ticket content. |

## How to re-run

```bash
bash scripts/audit-cross-tenant-coverage.sh \
  > docs/skycot/audit/cross-tenant-endpoint-coverage.md.script
# diff against the "Test-file flag" check below — drift means a new
# *_tests.rs file landed or an existing one removed cross-tenant
# coverage. Re-author this report when drift > 0 rows.
```

The script's output is a flat `(YES|NO, gateway|cli, filename)`
table, deliberately narrower than this report (the manual risk
classification + endpoint-family grouping cannot be inferred from
filenames alone). Keep the script and the report **separate
artefacts**: the script is the regenerable signal, the report is the
human-authored interpretation.
