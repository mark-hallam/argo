# SKYCOT Operator Runbook

**Status**: v1 customer-launch — 28 PRs merged, 4-day swarm
delivered the full Worker OS stack, e2e gate (claude-e2e-integration-test #810) passes
offline.

A walkthrough from `cargo install` to your first auto-approved
action and a live audit trail. Written for an operator who has
never seen SKYCOT — by the end you'll know what to install,
how to drive it, and where to look when something goes wrong.

If you want the *why*, read [MISSION.md](MISSION.md). This
document is the *how*.

> For dev / staging cleanup tooling (autonomous-coding-session
> disk guardrails — NOT production), see
> [`../dev-runtime-guardrails.md`](../dev-runtime-guardrails.md).

This runbook was split into topic-grouped sub-files so each one
stays under the project's 500-LOC-per-file ceiling. The sections
below are unchanged in content and ordering — they just live in
separate files now.

---

## Topics

1. [Getting started](runbook/getting-started.md) — what you're
   getting (§1), install (§2), bootstrap (§3).
2. [Workers and approvals](runbook/workers.md) — generate your
   first worker (§4), configure approvals (§5).
3. [Actions, skills, and handoffs](runbook/actions.md) — drive
   the first action (§6), skill learning loop (§7), multi-worker
   handoff (§8).
4. [Audit and observability](runbook/observability.md) — `argo
   audit query`, `argo doctor`, Prometheus, OTLP exporter, live
   audit stream, coverage, common failure modes (§9).
5. [Security](runbook/security.md) — gateway bearer-token auth,
   public read-only allowlist, integration-test bypass (§11).
6. [Production deploy](runbook/production-deploy.md) — Nginx,
   Caddy, systemd, and Docker Compose recipes for putting the
   gateway in front of real traffic (§12).
7. [Reference](runbook/reference.md) — where to next (§10), CLI
   command index (Appendix A), customer-demo file checklist
   (Appendix B).
8. [Troubleshooting](runbook/troubleshooting.md) — five
   operator-reported failures (schema mismatch, port already
   bound, missing API key, unknown channel, unknown executor
   kind) with verbatim error text + fix recipes.

---

## Quick links by task

- **First time installing?** → [Getting started](runbook/getting-started.md)
- **Generating a worker from a charter?** →
  [Workers and approvals §4](runbook/workers.md#4-generate-your-first-worker)
- **Setting up auto-approval?** →
  [Workers and approvals §5](runbook/workers.md#5-configure-approvals)
- **Driving an action through approve → execute → complete?** →
  [Actions §6](runbook/actions.md#6-drive-the-first-action)
- **Approving a learned skill proposal?** →
  [Actions §7](runbook/actions.md#7-skill-learning-loop)
- **Querying the audit ledger?** →
  [Observability §9.1](runbook/observability.md#91-argo-audit-query)
- **Wiring up Prometheus or OTLP?** →
  [Observability §9.3](runbook/observability.md#93-prometheus)
- **Shipping logs to Loki / Grafana / ELK?** →
  [Observability §9.8](runbook/observability.md#98-log-aggregation)
- **Provider, schema, or SQLite lock errors?** →
  [Error triage](runbook/error-dashboard.md)
- **Gateway exits 78 on boot?** →
  [Observability §9.7](runbook/observability.md#97-startup-health-check-exit-78)
- **Setting a gateway bearer token?** →
  [Security §11.1](runbook/security.md#111-gateway-bearer-token-auth)
- **Putting the gateway behind a reverse proxy?** →
  [Production deploy §12](runbook/production-deploy.md#12-production-deploy)
- **Customer-demo readiness check?** →
  [Reference Appendix B](runbook/reference.md#appendix-b--file-checklist-for-a-customer-demo)
- **Driving the hire flow over HTTP?** → [Genesis API](#genesis-api)
- **Approval flow over JSON?** → [Approvals JSON API](#approvals-json-api)
- **Planning a read-replica deployment?** → [Architecture: read-replica design](architecture/read-replica-design.md)
- **Schema / port / API-key / channel / executor error?** → [Troubleshooting](runbook/troubleshooting.md)

---

## Genesis API

See [`runbook/genesis-api.md`](runbook/genesis-api.md) for the full
Genesis API operator guide — the two JSON endpoints
(`POST /api/v1/genesis/classify` and
`POST /api/v1/genesis/draft-worker`) that back the SPA "hire a
worker" flow, with request/response shapes, error codes, auth, and
audit-row payloads.

---

## Approvals JSON API

The gateway exposes a JSON wrapper over the worker-action
approval state machine at `/api/v1/approvals` (the legacy HTML
`/inbox` route is preserved separately). All endpoints require
`Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN` when the
gateway bearer-auth token is configured.

```bash
# List pending approvals (default 50 per page, max 100):
curl -sH "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  http://127.0.0.1:8734/api/v1/approvals

# Resume from prior page using next_cursor from the response body:
curl -sH "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  "http://127.0.0.1:8734/api/v1/approvals?limit=50&cursor=<token>"

# Approve / refuse a specific approval:
curl -sX POST -H "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  -d '{"decision":"approve"}' \
  http://127.0.0.1:8734/api/v1/approvals/<id>/decide

# Reverse a decision within the 30-second undo window:
curl -sX POST -H "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  http://127.0.0.1:8734/api/v1/approvals/<id>/undo
```

Error envelopes follow `{ "error": { "code": "...", "message":
"..." } }`. Standard codes: `bad_request` (400),
`approval_not_found` (404), `already_decided` (409),
`undo_window_expired` (410), `internal_error` (500).

## Multi-tenant audit (v1)

Stream `mission-enterprise-cross-tenant-audit` adds per-tenant
scoping to the `GET /api/v1/audit` feed. v1 keeps the existing
single-tenant-per-workspace convention; multi-tenant deployments
opt in by setting `[tenant] default_id` in `argo.toml`.

### Workspace tenant id

`argo_config::workspace_tenant_id(<workspace_root>)` resolves the
tenant id every audit-write and audit-read goes through:

1. `[tenant] default_id = "..."` in `<workspace>/argo.toml`, if
   non-empty and not the shipped placeholder (`"default"`).
2. `ws-<16hex>` derived from a SHA-256 of the workspace root path
   when no explicit id is set. Stable across runs on the same path
   so single-tenant deployments self-name without operator config.

The helper never returns the `__legacy__` sentinel — that bucket
belongs exclusively to pre-migration rows (see below).

### The `__legacy__` bucket

Migration 64 (`audit_record.tenant_id`) backfills every existing
`audit_record` row with `tenant_id = '__legacy__'`. The default
workspace tenant's read scope UNIONs the legacy bucket so
historical audit visibility is preserved with zero operator
intervention.

A *deployment-time decision* faces operators rolling out real
multi-tenant: legacy rows must migrate to real tenant ids BEFORE
the bucket exits the default-tenant scope, otherwise an
operator-A request will see operator-B's pre-migration audits.
The follow-up brief `mission-audit-legacy-tenant-migrate-cli` adds
`argo audit migrate-legacy-tenant <from-tenant> <to-tenant>`.

### Cross-tenant view capability

`audit.view.cross_tenant` (constant
`argo_capability::AUDIT_VIEW_CROSS_TENANT`) gates the optional
`?as_tenant=<other>` admin probe on `GET /api/v1/audit`. No built-
in role grants it — operators grant it explicitly:

```rust,ignore
use argo_capability::{audit_view_cross_tenant_capability, CapabilityEngine};
use argo_types::Principal;

let engine = CapabilityEngine::with_defaults();
// Per-principal one-shot grant (e.g. an admin session):
engine.grant_dynamic(
    Principal::Local { os_user: "alice".into() },
    audit_view_cross_tenant_capability(),
);
```

Without the capability, the `?as_tenant=<other>` query param is
silently ignored — the feed responds with the workspace tenant's
own rows. This is intentional: a 403 would leak the target
tenant's existence to an unauthorised caller. With the capability
the legacy `__legacy__` bucket is NOT unioned into the response,
so admin probes never pull pre-migration rows into the wrong
tenant view.

### Inspecting tenant assignment

```bash
# How many rows live under each tenant?
sqlite3 ~/.argo/sessions.db \
  "SELECT tenant_id, COUNT(*) FROM audit_record GROUP BY tenant_id"
```

`__legacy__` rows accumulate from pre-migration history;
`ws-default` accumulates from un-tenanted `AuditSink::new(store)`
call sites that haven't been migrated to `new_with_tenant` yet
(both are unioned into the default-tenant read scope).

---

## Team API

The gateway exposes a tenant-scoped roster for the SPA Team page at
`/api/v1/team` (Stream J6). Each `WorkerSummary` carries the persona
+ role + a derived runtime `status` + the E3-c Today-block headline.

```bash
# List the workspace tenant's workers:
curl -sH "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  http://127.0.0.1:8734/api/v1/team

# Explicit tenant scope (must match the workspace tenant; mismatch → 403):
curl -sH "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  "http://127.0.0.1:8734/api/v1/team?tenant_id=<uuid>"
```

Response shape:

```json
{
  "workers": [
    {
      "id": "<uuid>",
      "name": "Maya",
      "avatar_url": null,
      "role": { "tagline": "Own outbound follow-up.", "vertical": "gtm-engineer" },
      "status": "waiting",
      "department": { "id": "...", "name": "Sales" },
      "team": { "id": "...", "name": "Outbound" },
      "headline": {
        "bullets": ["8 replies sent", "$340 in pending offers"],
        "tomorrow": "morning routine at 06:30"
      },
      "last_active_at": "2026-05-12T17:42:00Z",
      "pending_approval_count": 2
    }
  ],
  "total_count": 47
}
```

`status` is derived: circuit `Open` → `blocked`, `HalfOpen` →
`paused`, pending approval > 0 → `waiting`, activity within the
last 5 minutes → `working`, else `idle`. The list is sorted
waiting → working → idle → paused → blocked, ties broken by
`last_active_at` desc. Workers are capped at 200 in v1; check
`total_count > workers.length` to detect truncation
(`mission-enterprise-team-pagination` lifts the cap). RBAC is
tenant-scoped only in v1 — per-role visibility ships via
`mission-enterprise-team-rbac-scoping`.

### Per-worker trust grade

`GET /api/v1/workers/<id>/trust-grade` (mission-trust-grade-ui)
returns a read-only snapshot of the worker's autonomy grade:
current rung on the `argo_skill::TrustGrade` ladder, the signals
behind it, and the criteria gap to the next rung. The SPA Trust
tab (`/workers/<id>/trust`) consumes this directly. Trust grade
is derived from observed behavior and is intentionally NOT
operator-tunable from the UI; the underlying `worker.trust_grade`
u8 is set by the evaluation logic in `argo-identity` and the
routing engine.

---

## Vuln management

The vulnerability-management workflow is the operator process for
triaging RustSec advisories, GitHub Security Advisories, and NVD
CVEs that affect the build. SEV 1–4 taxonomy, P0/P1/P2/P3 triage
SLAs, named-owner table, and the `argo security audit` CLI live in
[`security/vuln-management.md`](security/vuln-management.md).

Quick start:

```bash
# One-shot: merge cargo audit + cargo deny check advisories into a
# single envelope; exit 1 if anything unmitigated above the floor.
argo security audit --severity-floor high

# Daily cron sweep — compact JSON for piping into the ops dashboard.
argo security audit --severity-floor low --compact
```

Exit codes: `0` clean, `1` unmitigated advisories above floor, `2`
tooling error (missing `cargo audit` / `cargo deny`, network
failure pulling the RustSec feed). The envelope shape +
named-owner table + disclosure policy are documented in
[`security/vuln-management.md`](security/vuln-management.md).

## Key rotation

`argo security rotate-keys` is the operator-facing coordinator for
rotating workspace-managed secrets. For categories whose subsystem
CLI is wired (`db-encryption` today), the coordinator spawns the
subsystem command directly with `std::process::Command`, captures
its stdout/stderr/exit-code into the `security.key_rotated` audit
row, and propagates non-zero exits as a retryable failure. For
env-var-rotation categories that can't self-rotate from inside a
running process (`gateway-bearer`, `channel-hmac`), and for
categories whose subsystem flag surface hasn't landed yet
(`credentials-master`, `connector-oauth`), the coordinator records
rotation intent and prints the operator follow-ups instead.

Registered key categories (v1):

| Slug | Subsystem | Dispatch | Rotation primitive |
| --- | --- | --- | --- |
| `db-encryption` | SQLCipher database | auto-spawn | `argo db rekey --new-key-source <src>` |
| `credentials-master` | credentials.toml | operator follow-up | `argo cred encrypt-all` (after `ARGO_CREDENTIALS_MASTER_KEY` rewrite) |
| `gateway-bearer` | Gateway HTTP auth | operator follow-up | env-var change + daemon restart |
| `channel-hmac` | Voice / channel HMAC | operator follow-up | env-var change + daemon restart |
| `connector-oauth` | Per-connector OAuth | operator follow-up | `argo connector authorize <name>` per connector |

Bidirectional voice chat: not shipped in v1.0 — see
[`operator/voice-channels-status.md`](operator/voice-channels-status.md).

### Suggested operator cadence

- **Weekly (cron-friendly):** `argo security rotate-keys
  audit-key-age --json` exits non-zero (1) if any key is older
  than the threshold (default 90 days). Wire to PagerDuty /
  email by checking the exit code; exit 2 means a config error
  and warrants a human eye, not a page.
- **Monthly:** for any key that the audit flagged, run
  `argo security rotate-keys plan <slug>` to preview the work
  (subsystem command + operator follow-ups) and copy it into the
  maintenance-window doc.
- **Maintenance window:** `argo security rotate-keys commit
  <slug> --new-key-source <src>` to record the rotation. For
  auto-spawn categories the coordinator runs the subsystem
  command itself and captures the output into the audit row; for
  operator-follow-up categories it prints the checklist. Re-running
  `commit` with the same `--new-key-source` on the same UTC day
  is a no-op when the prior attempt succeeded, and resumes the
  rotation when the prior attempt failed (deterministic
  `rotation_id` keyed on `<slug>:<redacted-source>:<utc-day>`).
- **Dry-run:** `argo security rotate-keys commit <slug>
  --new-key-source <src> --dry-run` prints the resolved
  subprocess command line (subsystem categories) or the
  follow-up checklist (operator-prompt categories) without
  spawning a process or writing an audit row.

`--new-key-source` is one of `random` (default; OS RNG via the
`rand` crate inside the subsystem command), `prompt` (read from
stdin without echo), or `env:<VAR>` (read from an env var). Only
the redacted descriptor (`random` / `prompt` / `env`) appears in
the audit row; the raw value is never logged.

Cross-process synchronization and HSM / KMS integration are
explicit non-goals in v1. Single-tenant deployments only.

## Context-window compression

See [`runbook/context-window-compression.md`](runbook/context-window-compression.md)
for the full guide — the trigger-based summarisation that keeps
long-running sessions inside the provider's context-window cap,
the operator knobs in `[runtime.context_compress]`, the
`runtime.context_compressed` audit payload, and the deterministic
`context_truncate` fallback for when summarisation itself fails or
a single turn is too big to fit.

---

## Action throttling

See [`runbook/action-throttling.md`](runbook/action-throttling.md)
for the full guide — the per-`(tenant_id, action_type)`
token-bucket gate at the dispatch layer, its `argo.toml` policies,
`argo doctor throttles` inspection, and how to read sustained
`runtime.action_throttled` audit rows as a "tighten or widen"
signal.

## Sandbox pre-warming

See [`runbook/sandbox-prewarming.md`](runbook/sandbox-prewarming.md)
for the full guide — the
`argo-executor-computer-use::sandbox_prewarm` reservoir that hides
E2B cold-start cost during operator-active hours, its sliding-window
+ hourly-tick triggers, the `[runtime.sandbox_prewarm]` knobs, and
how it pairs with the out-of-band E2B reaper.

## Fuzz testing

Argo ships cargo-fuzz harnesses against four parse-once boundaries
where malformed input could panic or behave unexpectedly: the audit
chain hasher, the redaction-rule scanner, the Genesis keyword
classifier, and the MCP JSON-RPC frame parser. The harnesses live
under `crates/<crate>/fuzz/` and form their own cargo workspace —
they don't appear in the main `cargo build`.

### Running a smoke locally

cargo-fuzz wraps LLVM libFuzzer, so the nightly toolchain is
required.

```bash
rustup toolchain install nightly
cargo install cargo-fuzz --locked

# 60-second smoke of the audit chain-hash target.
cd crates/argo-audit/fuzz
mkdir -p corpus/audit_chain_hash
cp -n seeds/audit_chain_hash/* corpus/audit_chain_hash/ 2>/dev/null || true
cargo +nightly fuzz run audit_chain_hash -- -max_total_time=60
```

Targets shipped today:

| Crate           | Target               | What it fuzzes                                |
| --------------- | -------------------- | --------------------------------------------- |
| `argo-audit`   | `audit_chain_hash`   | `compute_chain_hash(parent, kind, payload)`   |
| `argo-audit`   | `audit_chain_walk`   | Multi-row chain composition + replay          |
| `argo-redact`  | `redact_text`        | Regex scanner over arbitrary UTF-8            |
| `argo-redact`  | `redact_row`         | JSON-row redaction wrapper                    |
| `argo-genesis` | `archetype_classify` | Charter → archetype keyword classifier        |
| `argo-genesis` | `archetype_from_slug`| Slug-tolerant `Archetype::from_slug` lookup   |
| `argo-mcp`     | `jsonrpc_request`    | JSON-RPC request frame decoder + round-trip   |
| `argo-mcp`     | `jsonrpc_response`   | JSON-RPC response frame decoder + round-trip  |
| `argo-mcp`     | `call_tool_result`   | `tools/call` response content decoder         |

### Reproducing a crash from an artifact

When libFuzzer finds an input that panics, it writes the bytes to
`fuzz/artifacts/<target>/crash-<sha1>` (and a sibling
`leak-<sha1>` for leak detections). Reproduce locally:

```bash
cd crates/argo-audit/fuzz
cargo +nightly fuzz run audit_chain_hash artifacts/audit_chain_hash/crash-<sha1>
```

The argument after the target name is the path to a single input
file; libFuzzer runs the harness once against that input and
returns the panic backtrace.

When triaging, save the crash artifact into
`crates/<crate>/tests/fixtures/fuzz-regressions/` and add a regular
`#[test]` that feeds the bytes through the same entrypoint — the
crash artifact becomes a permanent regression test.

### Continuous fuzzing in CI

The nightly cron `.github/workflows/fuzz-nightly.yml.disabled` runs
every target for 60 seconds against the checked-in seed corpus.
It's disabled by default (rename to `.yml` once CI billing is
restored); the workflow uploads any crash artifacts so operators
can re-run the input locally via the recipe above.

## Retros

After a worker completes a batch of actions (or a long-running
duty wraps), the operator can run a structured **retrospective**
over a time window. A retro pairs auto-aggregated metrics from
the audit chain with a 5-section operator-fillable template:

1. **Surface metrics** (read-only counts)
2. **What went well**
3. **Surprises**
4. **What to change**
5. **Action items**

### Endpoints (mission-retro-review-ui, #209)

- `POST   /api/v1/workers/<id>/retros` — create a retro. Body
  keys (both optional): `window_start_unix`, `window_end_unix`.
  Defaults to `[now-7d, now]`. Aggregated metrics are frozen at
  creation and persisted on `retro.metrics_json`.
- `PATCH  /api/v1/workers/<id>/retros/<retro_id>` — overwrite
  one or more of the five section slots. Returns 409 once the
  retro is finalised; unknown section keys are rejected with 400
  so typos surface immediately.
- `POST   /api/v1/workers/<id>/retros/<retro_id>/finalize` —
  lock the retro. Subsequent PATCH/finalize calls return 409.
- `GET    /api/v1/workers/<id>/retros` — list retros for one
  worker, newest first.
- `GET    /api/v1/retros[?q=<theme>]` — tenant-scoped index. The
  `q` parameter does a case-insensitive substring match against
  the sections JSON column.

### Aggregated metrics

The `metrics_json` column carries six counts derived from
`audit_record` joined against `session.worker_id`:

- `actions_total` — `tool_call` rows
- `actions_approved` — `approval.granted`
- `actions_refused` — `approval.revoked`
- `escalations` — `escalation.required`
- `learned_skills_added` — `skill.approved`
- `error_count_by_category` — JSON object keyed by the offending
  audit kind (`halt`, `rbac.deny`, `lock.violate`, …) so future
  audit-kind additions land additional buckets without a wire
  migration.

### Audit trail

Every retro mutation emits one `http.request` row with an
`event_type` discriminator inside the payload:

- `retro.created` — written by the create handler with the
  retro id, worker id, and the chosen window.
- `retro.finalized` — written by the finalize handler with the
  retro id and the locked timestamp.

This mirrors the elevated-view pattern: the retro brief did NOT
expand `argo_audit::AuditKind`, so the audit chain stays under
the same enumerable kinds the verifier and dashboards already
recognise.

### What this is NOT

- **Not cross-worker.** v1 scopes one retro to one worker — the
  brief explicitly defers fleet-level retro aggregation.
- **Not vertical-specific.** v1 ships one canonical 5-section
  template; per-vertical retro shapes are a follow-up.
- **Not a skill-learning feedback loop.** Retros surface
  patterns; promoting them into learned skills runs through the
  separate skill-review path.

## Approval-Expiry Sweeper

The gateway starts an approval-expiry sweeper whenever the session
store opens successfully. Every tick calls the store's
`approval_request` timeout primitive and flips overdue `pending` rows
to `expired`, using each row's `default_decision` as the recorded
decision.

The tick interval is controlled by `ARGO_APPROVAL_EXPIRY_TICK_SECS`.
Default is `30`; positive values below `5` are clamped to `5`, and
`0`, negative, or non-numeric values are ignored with a warning.

When a sweep expires one or more rows, it writes one
`approval.expired_by_timeout` audit row with payload:

```json
{
  "kind": "approval.expired_by_timeout",
  "count": 2,
  "swept_at_iso": "2026-05-13T00:00:00+00:00"
}
```

Use `argo doctor approvals --since 24h` to list rows expired by this
sweeper, including the approval id, action id, decided-at timestamp,
and applied decision.

## Migrations

SKYCOT's workspace schema is **forward-only**. Each numbered
`vN.rs` migration is applied exactly once, in order, on startup;
there is no `down`-migration CLI and no supported in-place
rollback. A buggy `vN` is corrected by shipping a `vN+1` that
fix-forwards; a disastrous apply is recovered by restoring the
pre-upgrade backup, not by reversing the schema.

Full operator-facing reference (model, fix-forward recipe,
additive-only compatibility rules, the 3-step dance for
non-additive change, disaster recovery): see
[`operator/migrations.md`](operator/migrations.md).

## At-rest encryption

See [`runbook/at-rest-encryption.md`](runbook/at-rest-encryption.md)
for the full operator guide — opt-in SQLCipher page-level
encryption of the workspace DB, the `[security.encryption]
key_source` config, enabling on a fresh workspace, in-place
rotation via `argo db rekey`, and the backup / key-custody model.

See [`runbook/byok-cloud-kms.md`](runbook/byok-cloud-kms.md) for
the BYOK extension — wrapping the DEK under an enterprise AWS /
GCP / Azure KMS key so cloud KMS holds the root of trust.

## Per-vertical regression

See [`runbook/per-vertical-regression.md`](runbook/per-vertical-regression.md)
for the full operator guide — the `argo-vertical-regression`
static structural test crate that walks every `verticals/<id>/`
manifest + `eval/seed.jsonl` fixture and asserts action-type
vocabulary + approval consistency, with no LLM and no E2B.
