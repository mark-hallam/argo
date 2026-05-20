# Wave-15 audit — audit-log coverage of the user journey

**Auditor**: claude (audit agent #3)
**Audited against**: `origin/main` (2026-05-21, branch `main`)
**Scope**: signup → magic-link verify → onboarding-provision → first-hire → workflow save → test run → schedule trigger save. For each step: which `AuditKind` actually fires, what payload it carries, whether the `/audit` feed renders it readably, whether the feed cross-links to the related entity, whether `?action_type=` filtering matches.

---

## TL;DR — readiness score 5 / 10

| Journey step | Audit kind emitted | Cross-link readable | Searchable / filterable | Verdict |
| ---------------------------------- | ---------------------------- | -------------------- | ----------------------------------------- | ------- |
| 1. Signup (`POST /auth/signup`) | None _(beyond HttpRequest)_ | n/a | not first-class | **gap** |
| 2. Magic-link verify (`/auth/verify`) | `AuthSucceeded` / `AuthFailed` | n/a | by `kind` column only | OK |
| 3. Onboarding-provision (`/onboarding/provision`) | `TenantProvisioned` | n/a | by `kind` column only | OK |
| 4. First-hire (`POST /workers`) | `HttpRequest` _(payload-only)_ | none — worker_id is a string in payload | endpoint substring only | **gap** |
| 5. Workflow save (`POST /workflows/<n>`) | `WorkflowDefinitionCreated` | none — no link to builder | by `kind` column only | OK |
| 5b. Workflow publish (`PUT .../head`) | `WorkflowHeadChanged` | none | by `kind` column only | OK |
| 6. Test run (`POST /verticals/<v>/run`) | **None** _(only TurnStart/TurnEnd inside the loop)_ | n/a | n/a | **gap** |
| 7. Schedule save (`POST /cron`) | **None** | n/a | n/a | **gap** |

**Coverage**: 4 of the 7 user-visible journey steps emit a first-class audit kind. 3 leak through only as a generic `http.request` row, and 1 (test run) emits no top-level row at all — only the per-turn rows the workflow runner writes _inside_ the request, with no envelope tying them back to the operator-initiated "test run" click.

The audit chain is solid where it fires — every emit goes through the canonical `AuditSink::write`, gets hash-chained, gets the v67 filter-dim columns extracted, gets tenant-scoped. What's missing is (a) first-class kinds for 3 journey steps, (b) `verb`/`worker_name`/`worker_id`/`action_type` payload enrichment so the feed renders readably, and (c) any kind of cross-link from a feed row to the entity it acts on.

---

## Step-by-step trace

### Step 1 — Signup (`POST /api/v1/auth/signup`)

**File**: `crates/argo-cli/src/handlers/gateway/auth_magic_link_signup.rs:20-76`.

What it does: mints a magic-link token, inserts a row into `user_account` magic-link table, sends an email via `deliver_magic_link`. Returns `200 {"status":"sent"}`.

**Audit emit**: **none**. The signup handler writes no `AuditKind` row. The universal HTTP wrapper at `routes_request_audit.rs:79-121` will write one `http.request` row carrying `{method:"POST", path:"/api/v1/auth/signup", status:200, ...}` post-response, but this is generic and not first-class for "signup initiated".

**Impact**: rate-limiter denials (429), bot-protection denials (200 silent), magic-link delivery failures all land only as `http.request` rows with no payload distinction beyond the response code. A buyer asking "show me every signup attempt this week" cannot filter by `action_type=signup`; they have to substring-match on `path` (which the indexed feed-filter does not do).

**Recommended new kind**: `SignupRequested` with payload `{email_hash, ip_hash, outcome:"sent"|"rate_limited"|"bot_blocked"|"silent_ok", source_ip}`.

### Step 2 — Magic-link verify (`POST /api/v1/auth/verify`)

**File**: `crates/argo-cli/src/handlers/gateway/auth_magic_link_audit.rs:40-74`, called from `auth_magic_link_verify.rs`.

**Audit emit**:
- `AuditKind::AuthSucceeded` on successful exchange → `mission-magic-link-audit-followup-2026-05-18`. Payload: `{method:"magic_link", user_id, tenant_id, source_ip}`. Tenant: the user's tenant if onboarded, else `ANONYMOUS_AUDIT_TENANT_ID`.
- `AuditKind::AuthFailed` on missing/invalid/expired/already_used/rate_limited/store_unavailable. Pre-auth, so under `ANONYMOUS_AUDIT_TENANT_ID`.

**Readability**: the feed renderer reads `payload_str(p,"verb")`; this payload doesn't set `verb`, so the row falls back to `row.kind` (`auth.succeeded` or `auth.failed`). Acceptable but not human-tuned. `worker_name` is absent, so the feed displays the worker cell as `"unknown"`. `action_type` payload key is absent → indexed SQL filter does not match — only the app-layer pass picks it up via the `kind` column fallback _but only because the kind label IS the value the operator filter is comparing against_.

**Verdict**: OK. The dedicated kind exists and tenant-scoping is correct (anonymous fallback for pre-auth failures).

### Step 3 — Onboarding-provision (`POST /api/v1/onboarding/provision`)

**File**: `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs:166-194`.

**Audit emit**: `AuditKind::TenantProvisioned`. Payload: `{tenant_id, workspace_id, user_id, workspace_name, vertical_id, created}`. Principal is `human:<email>`. Tenant scope is the new tenant.

**Readability**: same shape as auth — no `verb` / `worker_name` / `action_type` enrichment, but the row IS first-class with a meaningful kind label. The feed will display it as `tenant.provisioned` in the action_type cell.

**Verdict**: OK. First-class kind + sensible payload.

### Step 4 — First-hire (`POST /api/v1/workers`)

**File**: `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs:256-302`.

**Audit emit**: `AuditKind::HttpRequest` — **not** a dedicated `WorkerHired` kind. Payload carries `{endpoint:"workers.hire", worker_id, vertical_id, persona_name, approval_policy_level, duty_count, problem, answers, draft:{...}}` — semantically rich, but bucketed under the generic `http.request` kind.

**Impact**:
- A buyer asking the audit feed "show me every hire this quarter" must filter by `action_type=workers.hire` (the `endpoint` payload key, which the v67 filter_dims extractor reads ONLY into `action_type` if explicitly named, which it isn't — the payload uses `endpoint` not `action_type`). On the indexed path the filter MISSES; on the app-layer fallback, `row_passes_filters` reads `action_type` _or_ `endpoint`, so it matches there. Two paths diverge in user-visible behaviour.
- Feed display: `action_type` cell shows `workers.hire` (via the `endpoint` fallback in `audit_row_json`); `verb` is `"http.request"`; `worker_name` is `"unknown"`. The new worker is named in the payload but not surfaced into the listing's worker cell.
- The 30s buyer skim "what happened this morning" reads as `[http.request] http.request — unknown` for the most important business event in the journey.

**Recommended new kind**: `WorkerHired` with payload that hoists `worker_id`/`worker_name`/`vertical_id` into the dim-extracted top level so the listing reads `[worker.hired] hired Peta · Project Coordinator · acme.com`.

### Step 5 — Workflow save (`POST /api/v1/verticals/<v>/workflows/<name>`)

**File**: `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs:137-170`.

**Audit emit**: `AuditKind::WorkflowDefinitionCreated`. Payload: `{actor, tenant_id, vertical_id, name, version_id}`.

**Readability**: feed displays kind label `workflow_definition_created`; `verb` falls back to that; `worker_name` is `"unknown"` (because hire isn't a worker action — it's an operator action against the platform; the worker cell should arguably show the actor's `human:<email>` but doesn't). `action_type` is absent → indexed-path filter miss; app-layer fallback to `kind` column works.

**Verdict**: OK. First-class kind ships. Worker cell ambiguity is a UX nit.

### Step 5b — Workflow publish (`PUT .../head`)

**File**: `routes_workflows_handlers.rs:194-216`.

**Audit emit**: `AuditKind::WorkflowHeadChanged`. Wave-14 confirmed this is the "save the canvas" emit. Same payload shape as create.

**Verdict**: OK.

### Step 6 — Test run (`POST /api/v1/verticals/<v>/run`)

**File**: `crates/argo-cli/src/handlers/gateway/routes_verticals_run.rs:32-138`.

**Audit emit**: **none at the request level**. The handler:
1. Creates a new `session_id` via `store.create_session(...)` — no audit row.
2. Attaches the skill bundle — no audit row.
3. Calls `vertical_workflow::run_collect(...)` which runs the workflow steps. _Each tool dispatch inside the loop_ emits `ToolCall`/`ToolResult`/`AssistantMessage`/`TurnStart`/`TurnEnd` rows tied to the new session id. Skill-invoke steps emit `WorkflowStepStarted` + `WorkflowStepCompleted` (per #8 of the 2026-05-17 codex audit).

Wave-14 (test-run audit) recorded this exactly: "test runs leave zero footprint" at the operator-action layer. The wrapper-level `http.request` row fires (and carries path = `/verticals/<v>/run`), but no top-level "test run started by Mark on workflow X" envelope ties the new session to the operator click.

**Impact**: forensic reconstruction of "who hit Test Run and when" requires joining `http.request` rows (path-suffix match) with the new session's `TurnStart` rows. There's no first-class envelope. PR #1884 (canvas live-execution overlay) does NOT add one either — Wave-14 noted "test runs leave zero footprint, even in the PR".

**Recommended new kind**: `VerticalRunRequested` with payload `{actor, vertical_id, workflow_name, session_id, provider, model}` written immediately after `create_session` succeeds and before `run_collect`. Pair with `VerticalRunCompleted` carrying `{session_id, ok, total_ms, step_count, failed_step?}` post-run.

### Step 7 — Schedule save (`POST /cron`)

**File**: `crates/argo-cli/src/handlers/gateway/routes_cron.rs:158-273` (`cron_create_response`).

**Audit emit**: **none**. The handler writes a JSON file to `<cwd>/cron/jobs/<name>.json` and returns `201 Created`. No `AuditSink::write` call anywhere in the cron path (create / delete / list / show). Only the wrapper-level `http.request` row fires.

**Impact**:
- A buyer asking "show me every scheduled task this tenant created" cannot filter the audit feed at all — the feed sees only generic `http.request` rows tagged `path=/cron` and the payload doesn't include the job `name`, `schedule`, or `prompt`.
- The cron job itself is a filesystem entity, not a SQLite row. It's tenant-stamped (`CronJob::new(TenantId::new(), ...)` at routes_cron.rs:242) — but `TenantId::new()` mints a **fresh** UUID, NOT the request's tenant. **This is a separate bug** worth flagging: every cron job created via the gateway gets a random orphan tenant_id, not the workspace tenant.
- Cron-firing events (`argo-cron::driver` actually invoking a job) are not audit-emitted either — Wave-14's schedule-cron audit corroborates this.

**Recommended new kinds**:
- `CronJobCreated` with `{actor, name, schedule, platform, chat, prompt_sha256}` (full prompt may carry secrets; hash it).
- `CronJobDeleted` with `{actor, name}`.
- `CronJobFired` with `{name, fired_at, target}`.

---

## Audit-feed display readability

**File**: `routes_audit_feed.rs:225-258` (`audit_row_json`).

The feed JSON renderer reads 8 payload keys on every row: `worker_id`, `worker_name`, `verb`, `outcome_status`/`status`, `approver_id`/`approver_name`, `channel`, `cost_usd`, `data_classification`/`classification`. With fallbacks for each. Action type comes from `payload.action_type` _or_ `payload.endpoint` _or_ `row.kind`.

Of the 7 journey steps audited above, **none** populate the `verb`, `worker_name`, or `approver_name` payload keys. Some (hire, workflows) populate semantic fields buried under different names (`persona_name`, `actor`). The feed therefore renders journey rows as:

```
[http.request] http.request — unknown — succeeded
[auth.succeeded] auth.succeeded — unknown — succeeded
[tenant.provisioned] tenant.provisioned — unknown — succeeded
[workflow_definition_created] workflow_definition_created — unknown — succeeded
[workflow_head_changed] workflow_head_changed — unknown — succeeded
```

— machine-readable but not human-tuned. A first-pass UX read at the buyer demo (Wave-12-ish audit) would land "this audit page is honest but reads like a log file, not a story".

---

## Cross-link integrity

**File**: `web/components/skycot/ActivityRow.tsx:30`.

Every audit row in the `/audit` feed links to `/audit/action/<id>` — the detail view. There is NO link from a hire row to the worker's detail page, no link from a workflow-save row to the workflow builder, no link from a tenant-provisioned row to the workspace settings, no link from an auth row to the user's settings.

**`CrossSystemLink`** (`web/components/skycot/audit/CrossSystemLink.tsx`) is a different mechanism: it scans the output body for Stripe / Slack / GitHub / Linear / Jira / Salesforce / HubSpot / Zendesk URL substrings and renders external anchors. It is _outbound_ cross-linking. There is no inbound cross-link primitive.

**`routes_audit_detail_refs.rs:21-44`** confirms this on the backend: `build_external_refs` walks `payload.output_body` for external system markers only. Journey-step payloads don't carry `output_body` — they carry domain fields like `worker_id`, `vertical_id`, `version_id`, `tenant_id`. None of these get hyperlinked.

**Verdict**: cross-link integrity is 0 / 10 for internal entities. External-system cross-links work (Wave-12-ish), but for an operator clicking a "John was hired" row to land on John's worker page, there is no path.

---

## Search / filter by action-type for journey kinds

**Indexed SQL path** (post-v67 migration, `routes_audit_feed_indexed.rs`): filters `?action_type=<v>` by ANDing `audit_record.action_type IN (?,…) OR audit_record.action_type IS NULL`. The column is populated from `payload_json.action_type` via `filter_dims::extract_filter_dims` (`crates/argo-audit/src/filter_dims.rs:74-81`).

None of the journey-step payloads set `payload_json.action_type`. So:
- All journey rows land in the DB with `action_type` column = `NULL`.
- The indexed filter clause `(col IN (?) OR col IS NULL)` admits them only via the NULL branch, so the SQL filter is effectively a no-op for these.
- The app-layer fallback in `routes_audit_feed.rs:196-223` re-filters via `action_type(row)` which reads payload, then endpoint, then `row.kind`. So filtering by `action_type=workflow_head_changed` works there.

**Net effect**: filtering by journey kinds works on the legacy path and on rows that fell through the SQL pre-filter (because the column is NULL the row was admitted unconditionally to the app-layer pass). It is NOT optimised by the index. For a tenant with 10k rows this is fine; at 1M rows the indexed path becomes a meaningful win that journey rows don't get.

**`?q=` substring search** (`routes_audit_feed.rs:212-220`) searches `payload.verb` (absent → empty), `payload.worker_name` (absent → empty), and `row.id`. A buyer typing `q=hire` will hit zero rows because no payload exposes the literal `"hire"` token in any of the searched fields. The substring-search dimension is _broken_ for the journey kinds today.

---

## Top 3 missing audit kinds

1. **`WorkerHired`** — first-class kind for the first-hire endpoint, with `worker_id` / `worker_name` / `vertical_id` / `actor` hoisted into the dim-extracted payload positions so the audit feed renders as `[worker.hired] hired Peta · Project Coordinator` and clicking the row would (after follow-up wiring) deep-link to the worker detail page.
2. **`VerticalRunRequested` + `VerticalRunCompleted`** — envelope rows around the test-run loop so forensic reconstruction has a "Mark hit Test Run on the qualify_lead workflow at 14:32" anchor row, not a synthesized join of `http.request` + new-session `TurnStart`.
3. **`CronJobCreated` + `CronJobDeleted` + `CronJobFired`** — the entire schedule subsystem is audit-dark. CronJobs themselves get an orphan tenant_id (`TenantId::new()` at `routes_cron.rs:242`) which is a separate bug worth raising — the cron job is created but cannot be tenant-scoped on read.

Additionally worth considering: **`SignupRequested`** so the buyer "show me every signup this week" question has a first-class answer; today it requires path-substring filtering on `http.request` rows.

---

## Files referenced

- `~/code/argo/crates/argo-audit/src/event.rs` (AuditKind catalog, 44 variants)
- `~/code/argo/crates/argo-audit/src/filter_dims.rs` (v67 column extraction)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_request_audit.rs` (wrapper-level HttpRequest)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/auth_magic_link_signup.rs:20-76` (signup — no audit)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/auth_magic_link_audit.rs:40-74` (auth.{succeeded,failed})
- `~/code/argo/crates/argo-cli/src/handlers/gateway/onboarding_provision.rs:166-194` (tenant.provisioned)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs:256-302` (hire — generic HttpRequest)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs:137-260` (workflow_definition_created / workflow_head_changed / workflow_version_archived)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_verticals_run.rs:32-138` (test run — no envelope kind)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_cron.rs:158-307` (cron — zero audit emits + orphan tenant_id bug)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:65-258` (feed read path)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:28-103` (detail read path)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_audit_detail_render.rs` (detail render)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_audit_detail_refs.rs:14-44` (external-link scanner — outbound only)
- `~/code/argo/web/components/skycot/ActivityRow.tsx:30` (row → `/audit/action/<id>` only)
- `~/code/argo/web/components/skycot/audit/CrossSystemLink.tsx` (external cross-link, not internal)
- `~/code/argo/web/components/skycot/audit/AuditDayGroup.tsx` (feed row wrapper)
- `~/code/argo/web/lib/filters/audit.ts:59-215` (FE filter dimensions, 7 facets)
- `~/code/argo/web/lib/types/audit.ts:103-119` (AuditRow shape)
- `~/code/argo/docs/skycot/audit/test-run/2026-05-21-wave-14-test-run.md` (corroborates "test runs leave zero footprint")
- `~/code/argo/docs/skycot/audit/save-publish/2026-05-21-wave-14-save-publish.md` (corroborates `WorkflowHeadChanged` wiring)
- `~/code/argo/docs/skycot/audit/schedule-cron/2026-05-21-wave-14-schedule-cron.md` (corroborates schedule audit gap)
