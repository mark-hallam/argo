# Windows full-platform journey (`win-e2e-full-platform-journey`)

**Question:** does the single binary serve the WHOLE teammate lifecycle on
Windows (not just `/health`)? Boot → hire a Genesis teammate → run it → probation
behaviour → audit/billing/team receipts.

**Verdict: 🟢 the platform journey serves end-to-end on Windows.** A real
Genesis teammate was hired through the served API (with a **real LLM safety
verdict via the local Claude Code CLI — no API key**), run under enforced
probation, and the hash-chained audit chain + team roster + scorecard + billing
meter all reflect it. Three honest GAPs recorded (approval-decide bundle,
zero billing meter, plaintext DB) — none a fabricated pass.

NO THEATER: every row below is real captured output from this Windows host.

---

## 1. Environment

| Field | Value |
|---|---|
| Machine | `Latitude5440` — Windows 11 Pro 10.0.26200, x86_64 |
| Binary | `argo 0.0.1 (f38280b 2026-06-14)` — **freshly built from `origin/main`, 0 commits behind** |
| Target / rustc | `x86_64-pc-windows-msvc` / `1.96.0` · profile **debug** |
| At-rest encryption | **plaintext** — debug binary lacks `--features bundled-sqlcipher`, so `sessions.db` starts `SQLite format 3` (GAP, same as GATEWAY-E2E.md); `SKYCOT_DB_KEY_BASE64` still *required* to boot |
| Live schema version | **94** |
| Isolation | dedicated `ARGO_HOME` + cwd under `%TEMP%`; no shared `~/.argo` touched |

Boot env (extends the #2920 recipe): `SKYCOT_DB_KEY_BASE64` (required) +
`ARGO_DEMO_MODE=1` **and** `ARGO_GATEWAY_AUTH_BYPASS=1` (demo-mode alone is
refused as production-unsafe) + `ANTHROPIC_API_KEY=sk-ant-placeholder-…` +
**`ARGO_VERTICALS_DIR`** (so `load_vertical` finds the packs — a fresh cwd has
none) + **`ARGO_MANIFESTS_DIR`** (so the Genesis safety judge can load the
`claude-code-cli` provider). The last two were a discovery: an isolated env does
NOT auto-resolve packs/providers — recorded for operators.

---

## 2. Boot proof

```
argo gateway: startup_check ok in 62 ms
argo gateway: duty scheduler enabled (interval=60s, gate=closed, runner=wired, sink=wired)
argo gateway: audit chain watchdog enabled
argo gateway: listening on 127.0.0.1:7402 (pid 35264)
argo gateway: workspace = 'argo-e2e-cwd'

$ netstat → TCP 127.0.0.1:7402 ... LISTENING 35264

$ GET /health  → 200 {"status":"ok","version":"0.0.1","schema_version":94,"build_commit":"f38280b","uptime_seconds":19}
$ GET /readyz  → 200 {"status":"ok",...}
```

---

## 3. Authenticate

```
$ POST /api/v1/demo/mint-principal  {"email":"e2e-journey@example.com"}
→ 200 {"ok":true,"principal":"human:e2e-journey@example.com","session_token":"<JWT>"}
   JWT claims: tnt=019ec4d0-c04c-7d00-904b-8f33f6e850cc, role=owner, demo=true
```
Subsequent requests use `Authorization: Bearer <JWT>` (NOT the `__Host-session`
cookie — cookie-auth requires an `X-CSRF-Token`; Bearer avoids CSRF).

---

## 4. The journey (real status codes)

### Step 2 — Hire a Genesis teammate ✅ (with a REAL LLM safety verdict, no API key)

`POST /api/v1/onboarding/first-hire` `{description, vertical_id:"customer-support", name:"Avery", confirmed_contract_digest}`.

- First pass (no digest): **422 `genesis_gate_failed` (`needs_review`)** — but the
  deterministic gate fully ran: `comprehension 5/5`, `eval 10/10`, and a real
  `role_contract` was synthesised (actions `draft_reply`/`send_response`/
  `escalate_ticket`/…, with `approval_required:[escalate_ticket, send_response]`).
  Two blockers: contract `unconfirmed` + **safety judge unavailable** (it needs
  the `claude-code-cli` provider).
- Pointing `ARGO_MANIFESTS_DIR` at the repo `manifests/` let the safety judge run
  a **real LLM turn via the local Claude Code CLI** (no Anthropic key). Re-hire
  with `confirmed_contract_digest`:

```
→ 201 Created {"worker_id":"019ec4d9-9a9c-7e63-9510-e5c6ed958a15","name":"Avery",
               "vertical_id":"customer-support","genesis_invoked":true,"genesis_fallback":false}
```

This is **stronger than the brief's fallback** (which expected the LLM step to be
a GAP): a genuine Genesis hire with a real safety verdict ran on Windows with no
API key, via the local CLI provider.

`GET /api/v1/workers/019ec4d9…/overview` → 200, charter "Customer Support Agent —
synthesised by Genesis…", plan id `customer-support-agent-avery`.

### Step 3 — Run the teammate (probation enforced) ✅

```
$ POST /api/v1/workers/019ec4d9…/run  {"note":"…","trigger_source":"manual_run_now"}
→ 200 {"status":"completed","probation":true,"held_tools":[],"chars":272,
       "action_id":"019ec4da0152…","session_id":"019ec4d9-fdad…","job_id":"019ec4d9-fd8c…"}
```

The freshly-hired teammate is in 24h probation: the run is **`probation:true`**
and the audit records a **`halt`** (the probation gate halts the autonomous turn
— the truth-boundary / approval-first behaviour). Two runs were driven; both
halted (`halt:2` in the audit). The teammate's turn drafted text (272 chars) and
did NOT autonomously act — exactly the probation guarantee.

### Step 4 — Approve the probation hold ⚠️ GAP (honest)

No approval **bundle** auto-formed: the teammate's turn drafted text without
emitting a committing-tool call, so `held_tools:[]` and `GET /api/v1/approvals`
→ `{"items":[]}`. A direct `POST /api/v1/approvals/bundles` (to exercise the
decide state-machine) returned a generic **404** (not dispatched at that exact
path / needs real member ids). **The probation ENFORCEMENT (the `halt`) is the
truth-boundary evidence**; the pending→approved *decide* transition was not
exercised on Windows this run (recorded as a GAP, not faked). A follow-up could
drive a worker-action that emits a committing tool to form a real hold-bundle.

### Step 5 — The receipts ✅

**Audit chain** (`GET /audit`, `GET /audit/count`, `GET /audit/recent`):
```
baseline: rows 3, count 4
final:    rows 51, count 52   → Δ +48 rows
{"breaks":0,"kinds":{"auth.succeeded":4,"halt":2,"hipaa_compliance_check":3,
   "http.request":39,"permission.demo_override_granted":1,"turn_start":2},"ok":true,"rows":51}
chain head: seq 52, id 019ec4dce0ed…, parent_hash 0c05ab7e… → self_hash 757e62c9…
```
**`breaks:0`** — clean hash chain on Windows; the journey's kinds (`turn_start`,
`halt`, `auth.succeeded`) are present and the chain GREW by the journey's rows
with a verifiable `parent_hash → self_hash` link.

**Team** (`GET /api/v1/team`, `GET /api/v1/team/scorecard`) → 200:
```
team:      {"total_count":1,"workers":[{"id":"019ec4d9…","name":"Avery",
            "role":{"vertical":"customer-support",...},"status":"idle"}]}
scorecard: {"summary":{"workers":1,...},"workers":[{"worker_id":"019ec4d9…",
            "worker_name":"Avery","vertical_id":"customer-support"}]}
```
The hired teammate appears in both the roster and the rollup. ✅

**Billing** (`GET /api/v1/billing/meter`) → 200, `{"tier":"free","used_actions":0,…}`.
⚠️ GAP: the meter serves but reads **zero** — no metered provider call fired
through the billing path (the local-CLI safety/turn LLM calls aren't metered via
this meter). Expected for this env; recorded, not faked.

---

## 5. Per-step PASS / GAP table

| Step | Endpoint | HTTP | Result |
|---|---|---|---|
| Boot + serve | `GET /health`, `/readyz` | 200 | ✅ PASS — schema 94, build f38280b |
| Authenticate | `POST /api/v1/demo/mint-principal` | 200 | ✅ PASS — owner JWT, tenant 019ec4d0 |
| Hire (Genesis) | `POST /api/v1/onboarding/first-hire` | **201** | ✅ PASS — worker 019ec4d9, **real safety judge via local CLI** |
| Worker overview | `GET /api/v1/workers/<id>/overview` | 200 | ✅ PASS — Genesis charter |
| Run (probation) | `POST /api/v1/workers/<id>/run` | 200 | ✅ PASS — `probation:true`, audit `halt` (enforced) |
| Approve hold | `GET /api/v1/approvals` / bundle decide | — | ⚠️ GAP — no committing-tool hold formed; direct create 404'd |
| Audit chain | `GET /audit` (+count, +recent) | 200 | ✅ PASS — breaks:0, Δ+48 rows, chain head w/ hashes |
| Billing meter | `GET /api/v1/billing/meter` | 200 | ⚠️ GAP — serves, but zero meter (no metered call) |
| Team rollup | `GET /api/v1/team` + `/scorecard` | 200 | ✅ PASS — Avery in roster + scorecard |

## 6. Honest GAPs / findings (NO THEATER)

1. **Approval-decide not exercised** — the probation run's turn drafted text
   without a committing-tool call, so no hold-bundle formed; the direct
   bundle-create endpoint 404'd. Probation *enforcement* (halt) IS proven; the
   pending→approved *decide* transition is a recorded GAP.
2. **Billing meter reads zero** — no metered provider call fired (the local-CLI
   LLM turns aren't billed through this meter).
3. **At-rest plaintext** — debug binary lacks `bundled-sqlcipher` (same as #2920);
   `sessions.db` is `SQLite format 3`.
4. **NEW — `argo gateway stop` is broken on Windows**: it returned
   `kill pid <N>: SIGTERM not supported on this platform` and the gateway kept
   listening (had to `taskkill /PID`). **Recommend a follow-up brief**
   `win-gateway-stop-portability` (Windows needs a non-SIGTERM stop path).
5. **NEW — isolated env needs `ARGO_VERTICALS_DIR` + `ARGO_MANIFESTS_DIR`**: a
   fresh cwd loads no packs/providers, so `load_vertical` 404s and the Genesis
   safety judge can't find `claude-code-cli`. Not a bug, but an operator gotcha
   worth documenting for Windows deployments.

## 7. Verdict

**The single binary serves the full teammate journey on Windows.** Boot → a
**real** Genesis hire (with a live LLM safety verdict via the local CLI, no API
key) → a probation-enforced run → a clean hash-chained audit chain → team roster
+ scorecard all served with real status codes on `x86_64-pc-windows-msvc`. The
two product GAPs (approval-decide bundle, zero billing meter) and the
`gateway stop` portability bug are recorded honestly rather than papered over.

_Verified by the Windows fleet (Latitude5440) on a fresh `f38280b` (= origin/main)
build. Worker `019ec4d9-9a9c-7e63-9510-e5c6ed958a15`, tenant
`019ec4d0-c04c-7d00-904b-8f33f6e850cc`, audit chain head seq 52._
