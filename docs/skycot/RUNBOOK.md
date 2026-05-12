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

---

## Genesis API

Two JSON endpoints back the SPA "hire a worker" flow (Track E2-c
+ E2-d). v1 is a thin HTTP wrapper around the existing
`argo-genesis` pipeline; the keyword classifier runs offline so
cost is reported as `null`.

### `POST /api/v1/genesis/classify`

**Body**

```json
{ "problem": "Marketing intern to handle our weekly newsletter" }
```

`problem` is required, must be at least 15 trimmed characters.

**200 response**

```json
{
  "classification": "marketing_intern",
  "questions": [
    { "id": "role", "text": "...", "answer_type": "free-text", "required": true },
    { "id": "name", "text": "...", "answer_type": "free-text", "required": true },
    { "id": "industry", "text": "...", "answer_type": "free-text", "required": false },
    { "id": "duties", "text": "...", "answer_type": "free-text", "required": true },
    { "id": "channels", "text": "...", "answer_type": "multi-select",
      "options": ["slack-dm", "email", "sms", "discord", "whatsapp"], "required": true },
    { "id": "spend_cap", "text": "...", "answer_type": "free-text", "required": true }
  ],
  "cost_usd": null
}
```

`classification` is one of the 12 archetype slugs or `"unknown"`
when no keyword matches. `cost_usd` is `null` in v1 — the
hardening brief `mission-enterprise-genesis-cost-cap` wires
per-tenant cost tracking.

**Errors**

- `400 bad_request` — body parse error, missing `problem`, or
  `problem` shorter than 15 trimmed characters.
- `408 timeout` — Genesis pipeline exceeded 30s (LLM-classifier
  rollout safety net; the v1 keyword path completes in
  microseconds).

**Example**

```bash
curl -sS -X POST http://localhost:8080/api/v1/genesis/classify \
  -H "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"problem":"Customer support agent for our SaaS billing tickets"}'
```

### `POST /api/v1/genesis/draft-worker`

**Body**

```json
{
  "problem": "Marketing intern to handle our weekly newsletter",
  "answers": {
    "role": "marketing intern",
    "name": "Maya",
    "industry": "B2B SaaS",
    "duties": ["draft newsletter", "post to LinkedIn", "qualify leads"],
    "channels": ["slack-dm", "email"],
    "spend_cap": 50
  },
  "adjustment": "Avoid emailing leads on weekends."
}
```

`answers` is the operator's responses to the question set returned
by `/classify`. `adjustment` is optional free text appended to the
charter (used by the Adjust drawer in E2-e). `duties` and
`channels` accept either a JSON array or a delimited string for
forward-compat with pre-E2-d clients.

**200 response**

```json
{
  "worker_draft": {
    "suggestedName": "Maya",
    "persona": "maya",
    "charter": "...full rendered charter...",
    "duties": ["draft newsletter", "post to LinkedIn", "qualify leads"],
    "exampleReplies": ["Sure — I can ...", "..."],
    "approvalPolicy": "5 of 8 actions require operator approval. ...",
    "suggestedVertical": "marketing_intern_maya"
  },
  "cost_usd": null
}
```

**Errors**

- `400 bad_request` — body parse error, missing `problem` /
  `answers`, missing required answer (`role`, `name`, `duties`,
  `channels`, `spend_cap`), non-positive `spend_cap`.
- `408 timeout` — Genesis pipeline exceeded 30s.
- `500 internal` — Genesis pipeline error (rare; usually means an
  invalid persona slug or template-loader failure).

**Auth + audit**

Both endpoints require the same bearer token as every other
`/sessions*` write. Every successful call writes one
`http.request` audit row with the Genesis-specific payload
`{endpoint, problem_length, question_count, cost_usd}` so
operators can replay hire-flow attempts via `argo audit query`.

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
rotating workspace-managed secrets. It does NOT generate key
material itself — actual rotation is delegated to subsystem
commands (`argo db rekey`, `argo credentials encrypt-all`,
`argo connector reauth <name>`, etc.). Every commit writes a
`security.key_rotated` audit row, so `audit-key-age` can answer
"is anything overdue?" by reading the audit chain.

Registered key categories (v1):

| Slug | Subsystem | Rotation primitive |
| --- | --- | --- |
| `db-encryption` | SQLCipher database | `argo db rekey` |
| `credentials-master` | credentials.toml | `argo credentials encrypt-all --new-master-key` |
| `gateway-bearer` | Gateway HTTP auth | operator follow-up (config + restart) |
| `channel-hmac` | Voice / channel HMAC | operator follow-up (config + restart) |
| `connector-oauth` | Per-connector OAuth | `argo connector reauth <name>` |

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
  <slug> --new-key-source <src>` to record the rotation, then
  run the subsystem command printed by `commit`. Re-running
  `commit` with the same `--new-key-source` on the same UTC day
  is a no-op (deterministic `rotation_id`), so it is safe to
  retry after a partial failure.

`--new-key-source` is one of `random` (default; OS RNG via the
`rand` crate inside the subsystem command), `prompt` (read from
stdin without echo), or `env:<VAR>` (read from an env var). Only
the redacted descriptor (`random` / `prompt` / `env`) appears in
the audit row; the raw value is never logged.

Cross-process synchronization and HSM / KMS integration are
explicit non-goals in v1. Single-tenant deployments only.

## Context-window compression

Long-running sessions can blow past a model's context-window cap
(200K tokens for Claude Sonnet 4.x, 128K for `gpt-4o`, etc.). The
runtime reacts before the provider rejects the call: when the
running token-count crosses a configurable trigger (default 80% of
`model.context_window_tokens`), older turns are summarised into a
single synthetic `assistant` message wrapped in
`<summary>…</summary>`, and the rest of the history (system prompt
+ last N user turns, default N=10) is kept verbatim.

The DB representation of `session.messages` is left untouched —
compression is a *view*, applied to the next turn's model input. A
later replay or `argo audit query session-replay` recovers the
original chain.

### Operator knobs (`argo.toml`)

```toml
[runtime.context_compress]
enabled = true            # master enable
trigger_pct = 80          # fire at 80% of model context window
keep_last_turns = 10      # trailing user turns kept verbatim
summary_max_tokens = 500  # cap on the generated summary
```

Aggressive operators set `trigger_pct = 70` to compress earlier
(useful when the summariser itself is on a small model and you want
budget headroom for it). Setting `enabled = false` short-circuits
the path entirely — the runtime falls back to the previous
truncate-or-fail behaviour and is responsible for that downside.

### Audit trail

Each compression pass writes one `runtime.context_compressed` audit
row with payload:

```json
{
  "pre_token_count": 184320,
  "post_token_count": 47210,
  "summary_length_tokens": 463,
  "turns_compressed": 42
}
```

`argo audit query --kind runtime.context_compressed` answers "how
often is this session paying the summarisation cost?" and feeds the
operator's decision on whether to drop `trigger_pct` further or
shorten `keep_last_turns`.

### What this is NOT

- **Not lossless.** The summary is a model-generated paraphrase;
  exact wording from collapsed turns is gone from the model-input
  view. The DB still has the originals if you need them.
- **Not a replacement for shorter sessions.** Compression buys
  headroom; it does not make the model retain everything.
- **Not multimodal-aware.** Image / audio content blocks are not
  compressed in v1 — only text turns participate in the summary.

---

## Action throttling

Some action types — `send_email`, `post_listing`, `create_invoice`
— have natural per-minute rate limits at the downstream API.
Without a worker-side gate, dispatch fires as fast as the loop
spins, hits 429s, then burns retry budget on avoidable load.

`mission-per-action-type-throttling` adds a token-bucket gate at
the dispatch layer. The bucket is keyed by
`(tenant_id, action_type)` — one slow tenant cannot starve
another tenant's dispatch.

### How it works

- One [`ThrottleRegistry`] per supervisor / dispatcher.
- For each dispatch attempt, the runtime calls
  `try_acquire(tenant_id, action_type, Utc::now())`.
- `Proceed` → fire the action immediately.
- `Wait { delay }` → yield to the scheduler for `delay` and retry.
  No busy-wait — the gate stays at the dispatch layer.
- When the cumulative wait observed across one dispatch's poll
  loop crosses `audit_threshold_secs` (default 10s), the dispatcher
  emits a `runtime.action_throttled` audit row with payload
  `{ action_type, queued_for_secs, tenant_id }`.

### Operator knobs (`argo.toml`)

```toml
[runtime.action_throttle]
enabled = true              # master enable, default false
audit_threshold_secs = 10   # emit audit row when wait ≥ this

[runtime.action_throttle.policies.send_email]
tokens_per_sec = 5.0
burst = 10

[runtime.action_throttle.policies.post_listing]
tokens_per_sec = 2.0
burst = 3
```

Disabled by default — the runtime dispatch path is zero-overhead
until `enabled = true`. Per-action policies are opt-in; an action
type absent from the `policies` table dispatches without
throttling even when the master enable is on.

### Inspection

```text
$ argo doctor throttles
argo doctor throttles — per-action-type throttle gate
  status:            ENABLED
  audit threshold:   10s (runtime.action_throttled when wait ≥)
  policies (2):
    - post_listing: 2 tok/s, burst 3
    - send_email: 5 tok/s, burst 10
  buckets:           (no live state — registry inspection is CLI-snapshot only in v1)
```

`argo doctor throttles --json` is the contract; the text form is
operator convenience.

### Sustained throttling means under-tuned config

A `runtime.action_throttled` row firing once is normal — bursts
exceed steady-state by design. A *sustained* stream of audit rows
at the same `(tenant_id, action_type)` means `tokens_per_sec` is
too tight for the workload. Either raise the rate, raise `burst`,
or accept the queueing and watch the dispatch latency rise.

### What this is NOT

- **Not a per-action-instance rate limiter.** v1 throttles by
  type, not by recipient (per-prospect / per-customer). That
  would multiply bucket count by N and is a different brief.
- **Not a priority queue.** FIFO in v1 — every blocked dispatch
  gets the same `Wait` treatment regardless of caller.
- **Not multi-host coordinated.** Single-host coverage. Distributed
  rate-limiting (Redis-token or equivalent) is the zero-trust
  brief's surface, not this one.

## Sandbox pre-warming

E2B sandbox creation has a cold-start cost (~10-15s). When a
vertical fires a burst of sandboxed actions in succession, each
cold-start chains. The
`argo-executor-computer-use::sandbox_prewarm` module keeps a
small reservoir of pre-opened sandboxes during high-activity
windows so the next acquire returns instantly.

### When to enable

Demo, real-time, and customer-facing workloads — anywhere a
multi-second wait between actions is visible to the operator or
end-user. Nightly-batch tenants should leave this off: the
pre-warm reservoir consumes E2B-minutes even when unused, and
batch latency hides cold-start cost in queue time anyway.

### Triggers

Two triggers drive admission to the reservoir, both bounded by
`max_prewarm`:

1. **Sliding-window spawn rate.** Every demand for a sandbox
   (slot miss + reservoir miss) lands in a sliding window
   (`window_seconds`, default 300s). When the count *strictly
   exceeds* `spawn_rate_threshold` (default 3), the prewarmer
   opens `burst_size` (default 2) extra sandboxes in the
   background.
2. **Hourly tick during operator-active hours.** Once per active
   hour the prewarmer opens one sandbox. Active hours are the
   inverse of the operator's quiet-hours block; empty
   `active_hours` means "every hour is active".

### Configuration

```toml
[runtime.sandbox_prewarm]
enabled = false                 # opt-in
spawn_rate_threshold = 3        # spawns/window above which burst fires
max_prewarm = 5                 # reservoir cap
window_seconds = 300            # sliding-window width
burst_size = 2                  # extra sandboxes per burst
idle_ttl_seconds = 600          # reservoir reaper TTL
active_hours = []               # empty = every hour active
```

### Audit trail

The prewarmer emits two structured-log kinds via `tracing`:

- `runtime.sandbox_prewarmed` — one per sandbox admitted to the
  reservoir. Fields: `template`, `sandbox_id`, `occupancy`.
- `runtime.sandbox_prewarm_consumed` — one per FIFO take of a
  pre-warmed sandbox by dispatch. Fields: `template`,
  `sandbox_id`, `occupancy`.

A `runtime.sandbox_prewarm_open_failed` warn-row surfaces when
the background open call fails — pre-warming is best-effort,
never a correctness requirement.

### Pairing with the sandbox-pool reaper

Pre-warming creates; the out-of-band reaper
(`argo-executor-computer-use::e2b_reap_cron`) destroys idle
sandboxes E2B-side. The two balance: an over-eager pre-warm
threshold burns wallet, an over-aggressive reaper drops
just-warmed sandboxes before they're used. The shipped defaults
(threshold 3, reservoir 5, idle TTL 600s) leave comfortable
headroom for demo-cadence workloads. Operators tuning either
knob should watch both audit streams together.

### What this is NOT

- **Not ML-driven.** The trigger model is a sliding-window
  heuristic + an hourly tick. No prediction, no anomaly
  detection. If your traffic doesn't fit "bursty around active
  hours," disable this and let the in-band pool handle cold opens.
- **Not multi-tenant-budgeted.** v1 uses a single global
  reservoir cap. Per-tenant pre-warm budgets are a follow-up.
- **Not for non-E2B sandbox types.** Wayland and native sandboxes
  have different lifecycle costs; this brief covers E2B only.

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
