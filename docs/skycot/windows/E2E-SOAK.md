# Windows platform-journey SOAK (`win-e2e-soak-multi-run`)

**Question:** is the single binary *stable under repeated use* on Windows, and
does per-run state (roster, usage meter, audit chain) stay correct and monotonic
as teammates accumulate across passes? (The single-pass journey —
`win-e2e-full-platform-journey` / `E2E-FULL-JOURNEY.md` — proved it runs *once*;
this repeats it.)

**Verdict: 🟢 7/7 consecutive passes clean.** Roster grew `0 → 7` (+1 every
pass, no reset, no double-count); the hash-chained audit chain grew monotonically
with **`breaks:0`**; probation was enforced on **every** pass (`halt:7`,
`turn_start:7` — exactly one each per pass); all 7 teammates accumulated in **one
tenant** (no `#2933`-class state-split). Two product GAPs (approval-decide
bundle, zero billing meter) recorded honestly — identical to the single-pass
journey, not regressions. No flakes, no lock residue, no slowdown across passes.

NO THEATER: every row below is real captured output from this Windows host
(`soak-results.json`, machine-parsed, not prose).

---

## 1. Environment

| Field | Value |
|---|---|
| Machine | `LATITUDE5440` — Windows 11 Pro 10.0.26200, x86_64 |
| Binary | `argo 0.0.1 (b0e346b 2026-06-15)` — **freshly built RELEASE from `origin/main` (b0e346b)** |
| Binary path / size | `C:\Users\operator\AppData\Local\argo-target\release\argo.exe` · **82,027,520 B (78.2 MB)** · profile **release** · rustc 1.96.0 |
| Same binary every pass | ✅ one build, 7 passes (single-binary-stability soak, not a rebuild soak) |
| State root (one, accumulating) | `…\Temp\soak-run2-b0e346b\home` (fresh `ARGO_HOME`; **not** wiped between passes) |
| Gateway | `127.0.0.1:7433`, pid 25960, schema 94 |
| At-rest encryption | **plaintext** — release binary built without `--features bundled-sqlcipher` (same documented GAP as the journey); `SKYCOT_DB_KEY_BASE64` still required to boot |
| N (passes) | **7** |
| Wall-clock | `2026-06-14T17:43:54Z → 17:45:40Z` (~1 min 46 s for boot + 7 passes + teardown) |

Boot env (same recipe as the journey): `SKYCOT_DB_KEY_BASE64` +
`ARGO_DEMO_MODE=1` **and** `ARGO_GATEWAY_AUTH_BYPASS=1` + placeholder
`ANTHROPIC_API_KEY` + `ARGO_VERTICALS_DIR` (repo `verticals/`) +
`ARGO_MANIFESTS_DIR` (repo `manifests/`, so the Genesis safety judge loads the
`claude-code-cli` provider → a **real LLM safety verdict via the local Claude
Code CLI, no API key**, on every hire).

---

## 2. Per-pass PASS/FAIL table

One principal (`human:soak@example.com`) was minted once and reused for all
passes (→ single tenant); a uniquely-named teammate (`Soak1`…`Soak7`) was hired
each pass so roster growth is observable.

| Pass | Hire submit→confirm | Worker id | Run (status / probation) | Approve | Audit rows (Δ / total) | Roster after | Meter after | Result |
|---|---|---|---|---|---|---|---|---|
| 1 | 422 → **201** | `019ec73b-ee21…` | completed / probation:true | none (GAP) | +14 / 17 | **1** | 0 | ✅ PASS |
| 2 | 422 → **201** | `019ec73c-23db…` | completed / probation:true | none (GAP) | +14 / 31 | **2** | 0 | ✅ PASS |
| 3 | 422 → **201** | `019ec73c-5952…` | completed / probation:true | none (GAP) | +14 / 45 | **3** | 0 | ✅ PASS |
| 4 | 422 → **201** | `019ec73c-9f8b…` | completed / probation:true | none (GAP) | +14 / 59 | **4** | 0 | ✅ PASS |
| 5 | 422 → **201** | `019ec73c-d3f4…` | completed / probation:true | none (GAP) | +14 / 73 | **5** | 0 | ✅ PASS |
| 6 | 422 → **201** | `019ec73d-068c…` | completed / probation:true | none (GAP) | +14 / 87 | **6** | 0 | ✅ PASS |
| 7 | 422 → **201** | `019ec73d-3955…` | completed / probation:true | none (GAP) | +14 / 101 | **7** | 0 | ✅ PASS |

- **Hire is the real 2-step Genesis flow every pass**: submit (no digest) → `422
  genesis_gate_failed (needs_review)` carrying the synthesised `role_contract` +
  its `contract_digest` (`68e9efa4dc5b…`, identical each pass — same
  customer-support contract, deterministic); confirm with
  `confirmed_contract_digest` → **`201`** with a real safety verdict. No pass
  fell back or faked.
- **Run is probation-enforced every pass**: `status:completed, probation:true`,
  and the audit records exactly one `halt` per pass (the probation gate halts the
  autonomous turn — the truth-boundary).

---

## 3. Cumulative-state assertion block (the soak-specific signal)

| Quantity | Baseline | After 7 passes | Per-pass delta | Verdict |
|---|---|---|---|---|
| **Roster** (teammate count) | 0 | **7** | **+1 each, monotonic** | ✅ no reset, no double-count |
| **Audit rows** (`/audit/count`) | 3 | **101** | **+14 each, monotonic** | ✅ non-decreasing (Δ includes the pass's own audited API calls) |
| **Meter** (`used_actions`) | 0 | 0 | 0 | ⚠️ GAP (see §5) — no metered call fired, same as journey |
| **`turn_start`** audit kind | — | **7** | +1 each | ✅ one turn per pass |
| **`halt`** audit kind | — | **7** | +1 each | ✅ probation enforced **every** pass |

**Audit-chain integrity (final `GET /audit`):**
```
{"breaks":0,"ok":true,"rows":105,
 "kinds":{"auth.succeeded":1,"halt":7,"hipaa_compliance_check":1,
          "http.request":89,"turn_start":7}}
chain head: seq 57, parent_hash babd5576…  →  self_hash 05fdce76…  (linked)
```
**`breaks:0`** across all 7 passes — the hash chain stayed linked and grew
monotonically; `halt:7`/`turn_start:7` confirm each pass's run is represented.
(`/audit/count` read 107 at the very end vs 101 after pass 7 — the +6 is the
final read-back queries auditing themselves; expected, monotonic.)

## 4. Single-tenant / workspace proof (guards the #2933 path-split class)

- All 7 hires used **one** principal (`human:soak@example.com`); `/api/v1/team`
  (tenant-scoped) reached **`total_count:7`** — i.e. all 7 teammates resolved to
  the **same** tenant's roster. A path-case / separator split (the `#2933` class)
  would have forked the roster and it could never have reached 7 under one
  principal. **No state-split observed.**
- Operative tenant (from the audit `worker.run` rows):
  `019ec73b-a41b-7973-92ce-d2481f0bd3b4`, consistent across passes.
- **Observation (not a bug):** with `ARGO_GATEWAY_AUTH_BYPASS=1` the caller is
  resolved from the `X-Argo-Principal` header, so the demo-mint's JWT tenant
  (`019ec73b-a68c-…`) is *unused* — nothing landed there; the principal-resolved
  tenant above is the single operative one. (See §5 methodology note.)

## 5. Flakes / GAPs (NO THEATER)

1. **No flakes.** All 7 passes are byte-identical in shape (422→201→completed→
   halt), ~13–18 s each with no slowdown, no lock-file residue, no DB-lock error,
   and a clean teardown (`Stop-Process 25960`). A clean soak — the table is the proof.
2. **Approval-decide not exercised (GAP)** — `approvals_count:0` every pass: the
   probation run drafted text without emitting a committing-tool call, so no
   hold-bundle formed. Probation *enforcement* (the `halt`) IS proven 7×; the
   pending→approved *decide* transition is unexercised — identical to the
   single-pass journey, not a regression.
3. **Billing meter reads zero (GAP)** — `used_actions:0` every pass: the local
   Claude Code CLI safety/turn LLM calls are not metered through this meter. Same
   as the journey.
4. **At-rest plaintext (GAP)** — release binary built without `bundled-sqlcipher`
   (same as the journey / `#2920`); `SKYCOT_DB_KEY_BASE64` is still required to boot.
5. **`argo gateway stop` is broken on Windows (pre-existing, `#2940` finding)** —
   teardown used `Stop-Process` (taskkill) of the one gateway pid, so the soak
   leaked **0** extra processes (one gateway booted, one killed). Tracked as
   `win-gateway-stop-portability`.
6. **Methodology note (operator gotcha):** the onboarding/API routes resolve the
   caller from the **`X-Argo-Principal: human:<email>`** header (trusted under
   `ARGO_GATEWAY_AUTH_BYPASS=1`), **not** from `Authorization: Bearer <JWT>`
   alone — a Bearer-only request 401s on the onboarding seam
   (`routes_onboarding.rs:11`). The first soak attempt 401'd all 7 hires on
   Bearer-only; adding `X-Argo-Principal` fixed it. The journey doc's "Bearer
   avoids CSRF" note is incomplete for these routes — recorded for operators.

## 6. Verdict

**The single binary is stable under repeated use on Windows.** Seven consecutive
real hire → run(probation) → audit passes against one accumulating state root:
roster monotonic `0 → 7`, audit chain monotonic with `breaks:0` and a verifiable
`parent_hash → self_hash` link, probation enforced on every pass (`halt:7`), and
all teammates in one tenant (no path-split). The two product GAPs
(approval-decide, zero meter) and the `gateway stop` portability bug are recorded
honestly rather than papered over.

_Run by the Windows fleet (Latitude5440) on a fresh RELEASE `b0e346b` (=
origin/main) build. Operative tenant `019ec73b-a41b-7973-92ce-d2481f0bd3b4`,
workers `019ec73b-ee21…` … `019ec73d-3955…`, final audit chain head seq 57,
`breaks:0`. Raw machine output: `soak-results.json` (7/7 PASS)._
