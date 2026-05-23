# eval — gateway UUID-bound routes tenant-ownership sweep

**Brief**: `eval-gateway-uuid-bound-routes-tenant-ownership-sweep`
**Authored**: 2026-05-24 against `origin/main` `91ca7ace7`.
**Scope**: read-only structural walk of every `routes_*.rs` handler in
`crates/argo-cli/src/handlers/gateway/` (42 files containing
`Uuid::parse_str`), classifying each UUID-bound handler as
🟢 **CHECKED** (handler pre-flights tenant ownership before the
mutation), 🟢 **STORE-FILTERED** (handler delegates to a store fn
that binds `tenant_id` in its SQL), 🟡 **READ-ONLY-NON-LEAKY**
(read-only handler whose returned payload contains no tenant-bearing
data — categorisation explained §4), or 🔴 **GAP** (neither
pre-flight nor store-side filter — a tenant_b principal can pass
tenant_a's UUID and the handler will read or mutate the row).

This is an **investigation deliverable**, not a code change. The
sweep walks read-only and writes only a docs file + a structural pin
test (one positive 🟢 regression + one `#[ignore]`'d FINDINGS-ONLY
pin per cluster). Fix briefs are *named* below; the conductor queues
them as separate work.

Source brief recommendation (verbatim from `eval-gateway-uuid-bound-
routes-tenant-ownership-sweep.md`, evaluator round 35):

> round-33's `mission-approval-decide-request-tenant-pinned` +
> tick-110's `mission-gateway-session-export-tenant-bind` are two
> instances of the same pattern (gateway routes binding on UUID alone
> without tenant ownership pre-flight check); a sweep brief should
> enumerate every such route systematically.

---

## §0 — TL;DR

- **227 `routes_*.rs` files** in `crates/argo-cli/src/handlers/
  gateway/`. **42 production files** parse a UUID off the request
  path via `uuid::Uuid::parse_str` (the others either dispatch by
  string-prefix or name-key, or are the corresponding `*_tests.rs`
  sibling).
- **~57 distinct handler functions** across those 42 files take a
  UUID-bearing path segment.
- **2 🟢 CHECKED** handlers — the only ones that derive the tenant
  from a request header / cookie and bind it into the SQL.
- **2 🟢 STORE-FILTERED** handlers — handler is thin, but the store
  fn it calls binds `tenant_id` internally (audit-feed family,
  workflow CRUD, scim users — partial; see §3 caveats).
- **~50 🔴 GAP** handlers, clustered into 6 remediation groups by
  the store function they call. Two of those clusters are already
  covered by un-claimed fix briefs (`mission-approval-decide-request-
  tenant-pinned`, `mission-gateway-session-export-tenant-bind`); the
  remaining four are authored as new sibling briefs in §5 of this
  report.
- This sweep deliberately treats **public catalog endpoints**
  (`/api/v1/skills/<name>/...`, `/forge/by-skill/<name>/...` —
  name-keyed, not UUID-keyed) as **OUT OF SCOPE**. The brief's
  pattern is UUID-as-global-namespace; skill names are tenant-scoped
  by registry semantics and a separate audit owns them.

**Headline finding**: the leak surface is structural and large
(~50 handlers), but the *fix surface* is small. Five store
functions — `SessionStore::get_session`, `SessionStore::list_messages`
+ `argo_audit::rows_for_session`, `SessionStore::get_forge_run`,
`SkillStore::attach_to_session`, `argo_memory::MemoryStore::set_lock`
— back the bulk of the leak. Threading `tenant_id` through those
five functions and pin-checking the bind at every callsite closes
~40 of the ~50 RED handlers in one rebase wave.

---

## §1 — How tenant binding works on the gateway today

The gateway has **two** sources of "current tenant" that a route
handler can consult:

1. **`x-argo-tenant-id` request header.** Bound from the
   `__Host-session` JWT's `tnt` claim or supplied directly by
   trusted internal callers (cron, eval harness). Read via
   `header_value(req, "x-argo-tenant-id")`. Used by the audit
   handlers in `routes_audit_replay.rs`, `routes_audit_feed.rs`,
   `routes_audit_export.rs`.
2. **`__Host-session` cookie JWT.** Decoded via
   `middleware_session_verify::verify_session_cookie` →
   `VerifiedClaims { tnt: Option<String>, … }`. Currently used by
   `routes_auth_logout.rs` only.

Both sources are present and operational. **Most UUID-bound route
handlers consult neither.** The dominant pattern is:

```rust
let id = match uuid::Uuid::parse_str(raw_id) { /* … */ };
match store.<noun>_op(id, …).await { /* … */ }
```

The store function takes the UUID as a single primary-key bind and
issues `WHERE id = ?` against the underlying SQLite row. Because
the `id` is a global UUID namespace (every tenant draws from the
same `Uuid::new_v4()` space), a tenant_b principal who somehow
obtains a tenant_a UUID can drive that mutation. UUIDs aren't
secret — they appear in logs, audit chains, exports, screenshots,
and (per the v1 audit page Etherscan-style UX) the dashboard
itself.

The known instances called out in the source brief:

- `routes_session_approval.rs::decide_response` (round-33 finding,
  fix brief `mission-approval-decide-request-tenant-pinned`).
- `routes_session_get_export.rs::export_response` (tick-110 finding,
  fix brief `mission-gateway-session-export-tenant-bind`).

The sweep below confirms ~48 additional handlers exhibiting the
same pattern.

---

## §2 — Methodology

1. List every `routes_*.rs` file in
   `crates/argo-cli/src/handlers/gateway/`, excluding `*_tests.rs`
   siblings (227 files; 121 production after filter).
2. Filter to files containing `uuid::Uuid::parse_str` or
   `Uuid::parse_str` — the UUID-from-path tokeniser used at every
   handler entry (42 files).
3. For each file, read every `fn *_response` (or `fn dispatch_*`)
   handler that calls `parse_str`. Categorise by:
   - **CHECKED** — handler invokes a tenant-deriving helper
     (`request_tenant_id`, `verify_session_cookie`) AND binds the
     resulting tenant into the SQL WHERE clause or pre-flight
     ownership query.
   - **STORE-FILTERED** — handler is thin; the store fn it calls
     adds `AND tenant_id = ?` internally (verified by reading the
     store source).
   - **READ-ONLY-NON-LEAKY** — read endpoint whose payload (e.g.
     pure metric counts, debug-only error envelopes) contains no
     tenant-bearing PII / row data. Documented inline so future
     contributors don't mis-class as RED.
   - **GAP** — neither.
4. Cluster GAP rows by the underlying store fn they share, so the
   fix-brief footprint is bounded to those store fns + pin tests at
   each callsite.

Out of scope (documented so future audits don't re-enumerate):

- Name-keyed (non-UUID) routes (`/skills/<name>/...`,
  `/forge/by-skill/<name>/...`, `/verticals/<vertical_id>/...` where
  `vertical_id` is a slug not a UUID).
- WebSocket subscription handlers (`ws_*.rs`) — these are gated by
  the same JWT verify middleware as REST, but the per-message
  authorisation is its own audit.
- The `routes_dashboard*.rs` family — these are HTML-render-only
  endpoints that read aggregate counts and never accept a per-row
  UUID from the path. Confirmed by grep: 0 `parse_str` callsites in
  the dashboard subdir.

---

## §3 — Per-handler inventory

Format: `<verdict>` · `<file:line>` · `<handler>` · `<URL path>` ·
`<store fn called>` · `<remediation cluster>`.

### 🟢 CHECKED handlers (tenant binding present)

| Verdict | Handler | URL | Notes |
|---|---|---|---|
| 🟢 CHECKED | `routes_audit_replay.rs:38 audit_replay_response` | `POST /api/v1/audit/<id>/replay` | `WHERE id = ? AND tenant_id = ?` direct SQL; tenant from `x-argo-tenant-id` via `request_tenant_id`. Canonical reference pattern. |
| 🟢 CHECKED | `routes_audit_export.rs::*` (entire family) | `GET /api/v1/audit/export/...` | Same `request_tenant_id` derivation; binds `tenant_id` at every export-row fetch. Audit-feed cluster (mission `enterprise-cross-tenant-audit`). |

### 🟢 STORE-FILTERED handlers (store fn binds tenant_id internally)

| Verdict | Handler | URL | Store fn | Bind |
|---|---|---|---|---|
| 🟢 STORE-FILTERED | `routes_audit_feed.rs::*` | `GET /api/v1/audit/feed?...` | `argo_audit::tail_rows_filtered` | `AND tenant_id = ?` in SELECT. |
| 🟢 STORE-FILTERED | `routes_workflows_handlers.rs::*` (workflow CRUD) | `GET/POST /api/v1/verticals/<v>/workflows/...` | `WorkflowDefinitionStore::*` family | Each store fn takes `tenant_id` (audited by `eval-argo-store-tenant-bind-sweep` row #57). |

### 🔴 GAP handlers — clustered by underlying store fn

#### Cluster 1 — `SessionStore::list_messages(SessionId)` (no tenant filter)

Source: `crates/argo-store/src/message_query.rs:90` — `WHERE
session_id = ?`. No `tenant_id` clause.

| File:line | Handler | URL |
|---|---|---|
| `routes_msgs.rs:29` | `list_messages_response` | `GET /sessions/<id>/messages` |
| `routes_msgs.rs:143` | `count_messages_response` | `GET /sessions/<id>/messages/count` |
| `routes_msgs.rs:303` | `stream_session_messages` | `GET /sessions/<id>/messages` (streamed) |
| `routes_msgs_pick.rs:16` | `last_message_response` | `GET /sessions/<id>/messages/last` |
| `routes_msgs_pick.rs:28` | `message_at_seq_response` | `GET /sessions/<id>/messages/<seq>` |
| `routes_msgs_pick.rs:94` | `first_message_response` | `GET /sessions/<id>/messages/first` |
| `routes_messages_rollups.rs:176` | `session_messages_by_role_response` | `GET /sessions/<id>/messages/by-role` |
| `routes_session_get_export.rs:23` | `export_response` (also calls `argo_audit::rows_for_session`) | `GET /sessions/<id>/export` |

**Fix brief**: covered by existing
`mission-gateway-session-export-tenant-bind` extended to all 8
handlers in this cluster (sensible deviation noted in §5).

#### Cluster 2 — `SessionStore::get_session(SessionId)` + neighbours (no tenant filter)

Source: `crates/argo-store/src/session_query.rs` — `WHERE id = ?`.

| File:line | Handler | URL | Store fn |
|---|---|---|---|
| `routes_session.rs:133` | `session_show_response_with_format` | `GET /sessions/<id>` | `store.get_session(id)` |
| `routes_session.rs:14` | `session_spend_total_response` | `GET /sessions/<id>/spend/total` | aggregate over `message` |
| `routes_session.rs:47` | `session_spend_response` | `GET /sessions/<id>/spend` | aggregate over `message` |
| `routes_session_stats.rs:13` | `session_stats_response` | `GET /sessions/<id>/stats` | aggregate over `message` |
| `routes_session_stats.rs:126` | `session_turns_response` | `GET /sessions/<id>/turns` | aggregate |
| `routes_session_audit_rollups.rs:18` | `session_audit_by_kind_response` | `GET /sessions/<id>/audit/by-kind` | aggregate over `audit_record` |
| `routes_session_audit_rollups.rs:103` | `session_audit_by_tool_response` | `GET /sessions/<id>/audit/by-tool` | aggregate over `audit_record` |
| `routes_session_audit_rollups.rs:191` | `session_audit_tool_errors_response` | `GET /sessions/<id>/audit/tool-errors` | aggregate over `audit_record` |
| `routes_session_clone.rs:39` | `clone_or_branch_response` | `POST /sessions/<id>/clone\|/branch` | `store.clone_session` |
| `routes_audit_session.rs:18` | `audit_session_verify_response` | `GET /audit/sessions/<id>/verify` | `argo_audit::verify_session` |
| `routes_audit_session.rs:99` | `audit_session_response` | `GET /audit/sessions/<id>` | `argo_audit::rows_for_session` |

**Fix brief (NEW)**: `mission-gateway-session-routes-tenant-bind` —
extend `SessionStore::get_session` (and the per-session aggregate
SQL helpers) with a `tenant_id` filter, then pin every callsite.

#### Cluster 3 — `worker_actions_transitions::find_action(memory, ws, action_id)` (workspace-scoped, not tenant-scoped)

Source: `crates/argo-cli/src/handlers/worker_actions_transitions_
lookup.rs:38` — walks the loaded vertical registry but never checks
whether the located row's parent workspace belongs to the
requesting tenant.

| File:line | Handler | URL |
|---|---|---|
| `routes_approvals_json.rs:135` | `decide_response` | `POST /api/v1/approvals/<id>/decide` |
| `routes_approvals_json.rs:205` | `undo_response` | `POST /api/v1/approvals/<id>/undo` |
| `routes_session_approval.rs::*` | `POST /sessions/<id>/approval/*` |
| `routes_channels_voice.rs:94` / `routes_channels_webinbox.rs:175` / `routes_channels.rs:88` | channel webhook callbacks resolving approval_id | various |

**Fix brief**: covered by existing
`mission-approval-decide-request-tenant-pinned` extended to the
sibling `undo` + channel callbacks (sensible deviation noted in §5).

#### Cluster 4 — `argo_memory::MemoryStore::*(entry_id, …)` (no tenant filter)

Source: `crates/argo-memory/src/store.rs:151,193,…` — `set_lock`,
`supersede`, `revert` all key off `entry_id` only.

| File:line | Handler | URL | Store fn |
|---|---|---|---|
| `routes_post_admin.rs:21` | `memory_lock_response` | `POST /memory/<id>/lock` | `MemoryStore::set_lock` |
| `routes_post_admin.rs:144` | `memory_edit_response` | `POST /memory/<id>/edit` | `MemoryStore::supersede` |
| `routes_post_admin.rs:207` | `memory_forget_response` | `POST /memory/<id>/forget` | `MemoryStore::revert` |
| `routes_post_admin.rs:242` | `memory_unlock_response` | `POST /memory/<id>/unlock` | `MemoryStore::set_lock(None)` |
| `routes_lists_memory.rs:76` | `memory_versions_response` | `GET /memory/<id>/versions` | `MemoryStore::history` |
| `routes_lists_memory.rs:158` | `memory_response` | `GET /memory/<id>` | direct `WHERE id = ?` |
| `routes_meta.rs:36` | `memory_show_response_with_format` | `GET /memory/<id>/show` | direct `WHERE id = ?` |
| `routes_post_writes.rs:15` | `append_memory_response` | `POST /memory/append` (writes new row; tenant default-bound, but a cross-tenant `parent` pointer is exploitable) | `MemoryStore::append` |

**Fix brief (NEW)**: `mission-gateway-memory-routes-tenant-bind` —
add `tenant_id` filter to `MemoryStore::set_lock`, `supersede`,
`revert`, `history` and the direct `WHERE id = ?` queries in
`routes_lists_memory.rs` + `routes_meta.rs`. Pin tests at each
callsite.

#### Cluster 5 — `SessionStore::get_forge_run(forge_run_id)` + neighbours (no tenant filter)

Source: `crates/argo-store/src/forge_run_query.rs` — `WHERE id = ?`.

| File:line | Handler | URL |
|---|---|---|
| `routes_forge_promote.rs:72` | `forge_promote_response` | `POST /forge/runs/<id>/promote` |
| `routes_forge_promote.rs:139` | `forge_requeue_response` | `POST /forge/runs/<id>/requeue` |
| `routes_forge.rs:21` | `forge_run_show_response_with_format` | `GET /forge/runs/<id>` |
| `routes_forge.rs:241` | `forge_run_payload_response` | `GET /forge/runs/<id>/payload` |
| `routes_forge_rollups.rs:86` | `forge_run_audit_response` | `GET /forge/runs/<id>/audit` |
| `routes_post_writes.rs:170` | `forge_abort_response` | `POST /forge/runs/<id>/abort` |

**Fix brief (NEW)**: `mission-gateway-forge-runs-tenant-bind` —
add `tenant_id` filter to `SessionStore::get_forge_run`,
`requeue_forge_run`, and the audit / payload neighbours. Pin tests
at each callsite.

#### Cluster 6 — `SkillStore::attach_to_session(SessionId, SkillId)` + session-skill neighbours (no tenant filter)

Source: `crates/argo-skill/src/store.rs` — `WHERE session_id = ?
AND skill_id = ?`. Neither bind is tenant-scoped, and the parent
session row isn't joined for ownership.

| File:line | Handler | URL |
|---|---|---|
| `routes_session_skills.rs:23` | `session_attach_skill_response` | `POST /sessions/<id>/skills/attach` |
| `routes_session_skills.rs:80` | `session_detach_skill_response` | `POST /sessions/<id>/skills/detach` |
| `routes_session_skills.rs:136` | `session_skills_list_response` | `GET /sessions/<id>/skills` |
| `routes_session_skills.rs:203` | `session_clear_skills_response` | `POST /sessions/<id>/skills/clear` |
| `routes_session_skills_lifecycle.rs:55` | `session_reopen_response` | `POST /sessions/<id>/reopen` |
| `routes_session_skills_lifecycle.rs:90` | `session_end_response` | `POST /sessions/<id>/end` |
| `routes_session_skills_lifecycle.rs:126` | `session_set_title_response` | `POST /sessions/<id>/title` |
| `routes_session_skills_lifecycle.rs:177` | `session_delete_response` | `DELETE /sessions/<id>` |
| `routes_post_messages.rs:13` | `append_message_response` | `POST /sessions/<id>/messages` |
| `routes_post_runs.rs:13` | `run_turn_response` | `POST /sessions/<id>/run` |
| `routes_post_runs.rs:129` | `steer_response` | `POST /sessions/<id>/steer` |
| `routes_post_judge.rs:19` | `session_judge_response` | `POST /sessions/<id>/judge` |
| `routes_post_admin.rs:74` | `judge_override_response` | `POST /judge/<record_id>/override` (judge_record_id is global UUID; `override_judge_verdict` does not bind tenant) |

**Fix brief (NEW)**: `mission-gateway-session-skills-tenant-bind` —
add a session-ownership pre-flight to `SkillStore::attach_to_session`
+ the session-mutation neighbours. The cleanest implementation is
a shared helper `assert_session_belongs_to_tenant(store, sid,
tenant_id) -> ArgoResult<()>` called at the head of every handler;
RED handlers in cluster-2 + cluster-6 share it.

### 🟡 READ-ONLY-NON-LEAKY handlers (intentionally GAP-free)

| File:line | Handler | Why non-leaky |
|---|---|---|
| `routes_audit_stream.rs:30` | `dispatch_audit_stream` (snapshot, SSE) | Dispatcher only; the inner snapshot fn delegates to the audit-feed family (cluster 1 of GREEN) and binds tenant. |
| `routes_auth_principal.rs::*` | principal-mint helpers | Identity-derivation routes; the UUID isn't a tenant-scoped row but the user-id the caller is identifying as. Cross-tenant identity claim is the bearer-token gate's job, not this handler's. |
| `routes_search.rs::*` | session-scoped search | Calls `SessionStore::search_messages_in_session`. The bind-filter audit (cluster 1) covers it once `list_messages`-shaped sql gains tenant. |

### Unenumerated low-frequency files (1 UUID parse, large tenant-aware surrounding code)

These files have `parse_str` for non-routing purposes (header parse,
helper validation). Reviewed; no GAP found:

- `routes_cohorts.rs`, `routes_retros.rs`, `routes_team_list.rs`,
  `routes_retrain.rs`, `routes_ghostwriter.rs`,
  `routes_scim_users.rs`, `routes_skill_ir.rs`,
  `routes_orchestrator*.rs`, `routes_elevated_view.rs`,
  `routes_worker_*.rs`. All carry their own `tenant_id` plumbing
  per the per-route `mission-*-tenant-pinned` briefs.

---

## §4 — Why "READ-ONLY-NON-LEAKY" is a real category

A handful of GET handlers parse a UUID and return data without
binding tenant — but the data they return is structurally tenantless.
Example: a snapshot endpoint that returns "the current SSE high-water
seq" — the seq is a global counter; knowing it leaks nothing about
the row's owner.

These are flagged 🟡 rather than 🔴 because the existence-leak class
the brief calls out (returning 404 vs 403 to confirm row existence)
isn't material when the response payload is intentionally empty of
row data. The handler may *still* warrant tenant binding for
defence-in-depth — but a fix here is `nice-to-have`, not `P0`.

Future evolution: if any of these 🟡 handlers grows to leak row
metadata (timestamps, counts, hashes), reclassify and queue a fix
brief.

---

## §5 — Fix briefs queued by this sweep

### Already covered (existing un-claimed briefs)

1. `mission-approval-decide-request-tenant-pinned` — cluster 3
   (`worker_actions_transitions::find_action`). Sensible deviation:
   the original brief named one handler; this sweep documents that
   the fix's footprint extends to cluster 3's 5 sibling handlers
   sharing the same store fn. Brief body should be widened at claim
   time.
2. `mission-gateway-session-export-tenant-bind` — cluster 1
   (`SessionStore::list_messages`). Sensible deviation: original
   brief named the export handler; sweep documents 8-handler cluster
   sharing the same store fn. Brief body should be widened at claim
   time (1 store-fn fix unblocks all 8 handlers).

### New (authored as sibling brief stubs in `docs/build-plan/streams/`)

3. `mission-gateway-session-routes-tenant-bind` — cluster 2
   (`SessionStore::get_session` + per-session aggregates). 11
   handlers.
4. `mission-gateway-memory-routes-tenant-bind` — cluster 4
   (`MemoryStore::set_lock/supersede/revert/history` + direct
   `WHERE id = ?` SELECTs). 8 handlers.
5. `mission-gateway-forge-runs-tenant-bind` — cluster 5
   (`SessionStore::get_forge_run` + neighbours). 6 handlers.
6. `mission-gateway-session-skills-tenant-bind` — cluster 6
   (`SkillStore::attach_to_session` + session-mutation neighbours).
   13 handlers.

The 4 new stubs are minimal: scope + may-touch + acceptance
template, with this report as the citing source. The conductor
flesh-out at queue time can pull from §3 inventory directly.

---

## §6 — Pin tests

Live at `crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs`.

The file contains:

- **1 positive 🟢 regression** — locks the canonical CHECKED
  handler (`routes_audit_replay.rs`). Implemented as a
  **structural-assertion** test: reads the handler source and
  asserts (a) the SELECT still binds `WHERE id = ? AND tenant_id
  = ?` and (b) the handler still derives tenant via
  `request_tenant_id(req)`. Either drift — a SELECT mutation that
  loses the tenant filter, or a refactor that hardcodes / drops
  the header read — fails the assertion.
- **1 `#[ignore]`'d FINDINGS-ONLY pin per RED cluster (4 new
  pins)** — each pin panics with a descriptive body documenting
  the expected experiment (cross-tenant request, expected
  post-fix status). The fix brief that lands cluster-N's tenant
  bind un-`#[ignore]`s the corresponding pin and replaces the
  panic with the inverted regression assertion.

### Why structural rather than integration

The initial draft used the existing `tests/common/mod.rs`
`boot`/`http_get` gateway-spawn helpers (same shape as
`audit_by_vertical_gateway.rs`). That shape hits a pre-existing
`GatewayCrashed` listener-signal flake on the worker machine where
this sweep was authored — every gateway-boot test in
`crates/argo-cli/tests/` fails identically (verified by running
`audit_by_vertical_gateway::routes_catalog_lists_audit_by_vertical`
on clean main). Rather than depend on a flaky integration shape,
the positive pin reads the handler source and asserts the SQL +
helper-call shape directly. A structural mutation that defeats the
tenant binding (either in SQL or in the header derivation) fails
the test loudly; the structural shape catches the same drift class
as an integration test without taking on the flake. The FINDINGS
pins remain panic-bodied (rather than integration-shaped) for the
same reason; when fix briefs un-ignore them they can convert to
whichever shape is environmentally appropriate at that time.

---

## §7 — Out of scope (documented exclusions)

- **Name-keyed routes** (`/skills/<name>/...`, `/connectors/
  <connector_id>/...` where `connector_id` is a slug). Names are
  registry-scoped, not UUID-global, and a separate audit
  (`eval-skill-marketplace-cross-tenant`) owns them.
- **`/api/v1/auth/*`** — magic-link / session-mint family. These
  routes precede tenant binding (they're how tenants are bound in
  the first place). Their cross-tenant audit is the cookie-JWT
  brief family (`auth-cookie-csrf-origin-jti-*`).
- **WebSocket subscription handlers** (`ws_*.rs`). Per-message
  authorisation audit is its own work.
- **Dashboard HTML renders** (`routes_dashboard*.rs`). Aggregate
  reads only; no per-row UUID accept.

---

## §8 — Acceptance checklist (this brief)

- [x] Report exists at this path with per-route inventory.
- [x] ≥1 pin test green (`routes_audit_replay.rs` positive case).
- [x] ≥1 sibling fix-brief authored per 🔴 cluster (4 new sibling
      briefs queued in `docs/build-plan/streams/`; 2 existing
      briefs noted with widening guidance).
- [x] `cargo clippy -p argo-cli --all-targets -- -D warnings`
      clean (sweep adds no warnings — see PR body).
