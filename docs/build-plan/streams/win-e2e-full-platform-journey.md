# win-e2e-full-platform-journey

**Owner**: windows-fleet — **P1 — run the FULL SKYCOT platform journey end-to-end ON REAL WINDOWS.**
**Depends on**: none. (Builds on #2920 `win-gateway-e2e-on-windows`, which proved /health serves; this is the full teammate lifecycle, not just /health.)
**Authored against**: origin/main `059e974b9dc8566883d46a2366f4e37f4f0114c6` on `2026-06-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-e2e-full-platform-journey`
**Fleet**: WINDOWS (`BUILD_STATUS_WINDOWS.md`). Build+run+verify ON REAL WINDOWS (Latitude5440). Push branch `win-e2e-full-platform-journey`, flip the row IN_REVIEW, do **NOT** self-merge — the macOS conductor gates + adversarial-reviews + merges. NO THEATER: this is a TEST brief — the deliverable is a DOCUMENTED real Windows run with concrete evidence; if a primitive can't work yet on Windows, the report says so **honestly** (real status code or honest gap, never a fake pass).

---

## Goal

Prove the **single binary serves the whole platform on Windows** by driving one
complete AI-teammate lifecycle through the served HTTP API on a real Windows
host: **boot the gateway → confirm it serves → hire a Genesis teammate → run it
→ approve the probation hold(s) → verify the hash-chained audit chain + billing
attribution + team rollup all reflect what happened.**

#2920 proved `/health` + a single authenticated read round-trip serve on
Windows. This brief proves the *journey* — the camera-visible product flow —
runs server-side on Windows end to end. This is the **full-stack Windows
e2e** the BLOCKED `win-cu-executor-end-to-end-run` row is NOT (that row is the
computer-use desktop loop, which needs a real ANTHROPIC_API_KEY + a live
desktop and is operator-gated; THIS brief drives the API/CLI journey and stays
agent-doable, with the LLM-turn step honestly marked if no key is present).

---

## Premise (VERIFIED real on origin/main `059e974b9`)

- The gateway boots + serves a real HTTP API on Windows — proven by #2920 /
  `docs/skycot/windows/GATEWAY-E2E.md` (/health + /readyz + /livez + /version
  200; auth-gated /audit + /workspace round-trip read live SQLite rows;
  bearer-auth gate correctly 401s when bypass is off).
- The journey routes exist server-side (grounded by grep on current main):
  - Genesis: `crates/argo-cli/src/handlers/gateway/routes_genesis_gate.rs`,
    `routes_genesis_quiz.rs`, `routes_genesis_rubric.rs` (SSE + quiz +
    comprehension rubric).
  - First-hire onboarding: `routes_onboarding.rs` (writes the worker row that
    `/api/v1/team` keys off).
  - Worker lifecycle + dispatch: `routes_dispatch_post.rs`,
    `routes_dispatch_post_paths.rs` — `POST /api/v1/workers/<id>/{pause,resume,archive}`,
    `POST /api/v1/workers/<id>/actions/<action>/executor`,
    `POST /api/v1/workers/<id>/capability-grants`.
  - Approvals: `POST /api/v1/approvals/bundles` + `POST /api/v1/approvals/bundles/<id>/decide`
    (`routes_dispatch_post_paths.rs:100-115`).
  - Team rollup: `GET /api/v1/team`, `GET /api/v1/team/scorecard`
    (`routes_dispatch_get_api_groups.rs:94-99`).
  - Billing attribution: `GET /api/v1/billing/meter`
    (`routes_dispatch_get_api_groups.rs:130-131`).
  - Demo principal mint for an authenticated session without a real magic-link:
    `demo_mode.rs` / `POST /api/v1/demo/mint-principal`.
  - Audit chain: `GET /audit` + `GET /audit/count` (proven live on Windows by
    #2920 — `{"breaks":0,...}` clean chain).
- So the journey is constructible from routes that already exist on main. What
  is UNPROVEN (the point of this brief) is whether the *whole sequence* runs
  end-to-end **on Windows** with real status codes + a real audit chain.

---

## Do (real Windows — Latitude5440)

Work in an **isolated** env so no shared `~/.argo` state is touched (mirror the
#2920 recipe exactly — it is proven to boot on Windows):

### 0. Build + boot the gateway

1. Build a fresh binary from the claimed SHA if the shared target dir is free
   (`cargo build -p argo-cli` → `argo.exe`). If the shared
   `CARGO_TARGET_DIR` is contended by a sibling Windows worker, you MAY reuse
   the most recent real Windows build present — but **record the binary commit
   SHA and its distance from `origin/main`** in the report (NO THEATER, exactly
   as #2920 did its binary-vs-main disclosure).
2. Boot, isolated, on a non-default port:
   ```
   ARGO_HOME=<temp>/argo-e2e-journey
   SKYCOT_DB_KEY_BASE64=<base64 of 32 random bytes>   # required to boot (no OS-keyring in temp env)
   ANTHROPIC_API_KEY=<real key if available, else sk-ant-placeholder-...>  # see step 3 / GAP
   argo.exe gateway start --bind 127.0.0.1 --port 7402 --quiet
   ```
3. Confirm it SERVES: `GET /health` → **200** with `schema_version` + `build_commit`;
   `GET /readyz` → **200**. Capture the listener (`netstat` shows
   `127.0.0.1:7402 LISTENING`) and the boot lines. Record the live schema
   version.

### 1. Authenticate the journey session

4. Mint a dev principal so subsequent writes are authenticated:
   `POST /api/v1/demo/mint-principal` (or set `ARGO_GATEWAY_AUTH_BYPASS=1` for
   the loopback dev path as #2920 did). Record which auth path you used and the
   tenant_id / principal it resolves to.

### 2. Hire a Genesis teammate

5. Drive the Genesis hire via the API: start a Genesis session
   (`/api/v1/genesis` SSE / the genesis-gate route), supply a plain-English role
   ("a support teammate that drafts replies and escalates refunds"), and walk
   the comprehension gate (`routes_genesis_quiz.rs` / `routes_genesis_rubric.rs`).
   - If `ANTHROPIC_API_KEY` is a **placeholder** (no live LLM), the Genesis SSE
     turn cannot produce a real model response — **say so honestly** and instead
     drive the **deterministic** first-hire onboarding path
     (`routes_onboarding.rs`) to create the worker row, so the rest of the
     journey (run → approve → audit/billing/team) still runs on real Windows.
     Mark this substitution explicitly as a GAP in the report (LLM-turn requires
     an operator key; the worker-creation + lifecycle is real either way).
6. Capture the created **`worker_id`** (UUID) and the worker row from
   `GET /api/v1/workers` / `GET /api/v1/workers/<id>/overview`.

### 3. Run the teammate (dispatch a task)

7. Dispatch a small task to the worker and/or exercise the worker-action
   executor route (`POST /api/v1/workers/<id>/actions/<action>/executor`) such
   that it produces a **probation hold** (a teammate in probation must escalate
   for approval before acting — the truth-boundary/approval-first behavior).
   Capture the HTTP status and the response body.
8. Confirm a pending approval exists: `GET /api/v1/approvals` (or the bundle
   list). Capture the **approval/bundle id** and its `pending` state.

### 4. Approve the probation hold(s)

9. Approve via `POST /api/v1/approvals/bundles/<id>/decide` with an `approve`
   decision (authenticated). Capture the **200** + the decided state.
10. Re-fetch the approval → confirm it flipped from `pending` to `approved`
    (real state transition, not a re-submit). Confirm the approver identity is
    attributed.

### 5. Verify the receipts (the whole point — checkable evidence)

11. **Audit chain**: `GET /audit` → **200**, `breaks: 0` (clean hash chain),
    and the `kinds` map now includes the journey's event kinds (e.g. a hire /
    genesis kind, a dispatch/run kind, an approval-decision kind). Capture the
    **audit chain head** (the latest row id / hash) and `GET /audit/count`
    before-vs-after the journey to show the chain GREW by the journey's rows.
12. **Billing attribution**: `GET /api/v1/billing/meter` → **200**; confirm a
    usage/meter row is attributed to the journey's tenant_id + worker_id (or
    honestly record if the meter is zero because no metered LLM/provider call
    fired — that is the expected consequence of a placeholder key, and is a
    GAP not a fail).
13. **Team rollup**: `GET /api/v1/team` + `GET /api/v1/team/scorecard` → **200**;
    confirm the hired teammate appears in the roster and the rollup reflects the
    run (activity count / last-active). Record the JSON.

### 6. Honest teardown

14. Stop the gateway. Confirm the isolated `sessions.db` persisted the journey's
    rows (it survives the run — read one back if useful). Leave no shared state
    touched.

---

## CHECKABLE evidence the report MUST contain (NO THEATER)

Every row must be **real captured output from this Windows host** — `curl`
bodies, CLI stdout, status codes. No paraphrase-only claims.

1. **Environment block** (exactly like GATEWAY-E2E.md): machine, OS build,
   binary commit SHA + distance-behind-`origin/main`, target triple, rustc,
   build profile (and whether `--features bundled-sqlcipher` — if not, note the
   at-rest-plaintext GAP), live schema version.
2. **Boot proof**: the `/health` 200 body, `/readyz` 200 body, the `netstat`
   LISTENING line, and the boot log lines.
3. **The worker_id** (UUID) of the hired teammate + the worker-row JSON.
4. **The run/dispatch** HTTP status + body, and the **approval/bundle id** with
   its `pending` → `approved` state transition (both states captured, with the
   `decide` 200).
5. **Audit chain**: the `breaks: 0` body, the `kinds` map showing the journey's
   kinds, `audit/count` BEFORE vs AFTER (delta > 0), and the **audit chain
   head** (row id / hash).
6. **Billing**: the `/api/v1/billing/meter` 200 body (real attribution, or an
   honest zero-meter GAP with the reason).
7. **Team rollup**: the `/api/v1/team` + `/api/v1/team/scorecard` 200 bodies
   showing the hired teammate.
8. **A per-step PASS / GAP table** with HTTP codes, and a one-paragraph honest
   verdict: did the FULL journey serve end-to-end on Windows? Any step that
   needs an operator key / live desktop is a recorded GAP with the exact reason,
   **never a fabricated pass**.
9. (Optional but valued) a screenshot of the dashboard (`GET /` dashboard route)
   or the SPA if a Windows browser is wired, showing the hired teammate +
   approval — clearly labelled if the SPA is NOT run on Windows.

Results doc: **`docs/skycot/windows/E2E-FULL-JOURNEY.md`**.

---

## Verify (definition of done)

The report at `docs/skycot/windows/E2E-FULL-JOURNEY.md` documents a **genuine
end-to-end teammate journey on real Windows** — boot → hire → run → approve →
audit/billing/team receipts — with real status codes, the worker_id, the
audit chain head + count delta, and an honest GAP list for any operator-gated
step (live LLM key / live desktop). Branch `win-e2e-full-platform-journey`
pushed; row flipped IN_REVIEW; FEATURE_MATRIX touched in-band if any
capability state changes (otherwise this is a test-only artifact and the
ledger row note carries the result). Do NOT self-merge — conductor gates.