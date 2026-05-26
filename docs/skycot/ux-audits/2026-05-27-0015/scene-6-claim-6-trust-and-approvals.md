# Scene 6 + Approvals — Claim 6 (Multi-channel + multi-tenant trust)

**Audit date**: 2026-05-27T00:15Z (Melbourne local: 2026-05-27 ~10:15am)
**Scope**: `/approvals` (E ops-dashboard), `/audit` (B3 Etherscan), hash-chain hover, audit-row drill-in (real vs fabricated), cross-tenant isolation, `/audit/action/{id}/replay`.
**Environment**: SPA `http://localhost:3000` + `http://127.0.0.1:3000` against gateway `http://127.0.0.1:7400` (schema v83 FRESH, uptime ~10h).
**Method**: Claude Preview MCP (DOM snapshots + screenshots), direct `curl` against gateway endpoints, source read of every data-path on the Scene-6 surface.

---

## TL;DR

Five P0 demo-blockers for Scene 6. The Audit and Approvals shells render beautifully — empty-state language is honest, no fabricated rows on real data — but the **gateway has nothing real to show the camera**: zero approvals seeded, zero agent-execution audit rows (only `http.request` self-traffic from the SPA viewing the audit page itself), zero workers under `/api/v1/workers` (returns 404; only `/api/v1/team` has one persona — "Maya", not the runbook's Mariah). The Etherscan-style hash-chain hover that anchors the page's whole trust pitch has no hash data to bind to because the gateway list-endpoint omits the `self_hash` / `parent_hash` fields. The Replay drill-in is the strongest finding on the positive side: the POST `/api/v1/audit/{id}/replay` endpoint genuinely re-hashes the row and returns a `verdict:"same"` with matching `actual_self_hash == expected_self_hash` — that's real, recordable, on-brand evidence. **Recommendation: do not record Scene 6 against the current data state. Seed the gateway with 3-5 real agent-execution audit rows + at least 2 pending approvals before any take.**

Counts: 5 P0, 4 P1, 2 P2, 1 P3 = 12 findings.

---

## Headline blockers

1. **F1 [P0]** — Approvals inbox is empty (`{"items": []}` from `/api/v1/approvals`). Scene 6 cannot demonstrate the approval flow at all.
2. **F2 [P0]** — Audit feed has zero agent-execution rows. Every row in the live gateway is `verb: "http.request"` with `worker.name: "unknown"` — the SPA self-instrumenting its own audit-page page-loads. The camera will see a "Forensic + compliance view" full of meta-noise.
3. **F3 [P0]** — Gateway list endpoint omits hash fields. `/api/v1/audit` returns no `self_hash` / `parent_hash` / `sample_hash` — the `HashCell` tooltip-hover (the page's headline trust feature, per the locked B3-Etherscan design) cannot render. Hash hover works ONLY against the sample-audit fixture, not real data.
4. **F4 [P0]** — Audit-detail drill-in shows shape-but-no-content. `/api/v1/audit/{id}` returns the full 8-step decision-chain envelope but every business field is null/empty (`worker.name="unknown"`, `body=""`, `details:{snapshot_id:null}`, `model.id=""`, `policy.id=""`, `prompt_template.hash=""`). Camera drill-in will show a credible-looking provenance card with eight empty rows under it.
5. **F5 [P0]** — Recording-day footgun: gateway CORS allowlist admits `http://localhost:3000` only. Hitting the SPA at `http://127.0.0.1:3000` (a totally natural address-bar slip) returns 200 from the gateway but with **no `Access-Control-Allow-Origin` header**, so the browser silently blocks every API response. Pages render shells with no data; nothing in the console flags the cause.

---

## All findings, in order

### F1 [P0] — Approvals inbox empty; Scene 6 cannot demonstrate approval flow

`GET /api/v1/approvals` returns `{"items": []}` against the live gateway. The Approvals page renders correctly (5 KPIs all show "—" / "0", three severity tiers say "No high/medium/low severity approvals waiting") — UX-honest but the camera will see an Inbox titled "Approvals" with literally zero rows. Scene 6 promises "Click an approval, see real evidence"; there is nothing to click.

Reproduction:
- `curl -s -H 'Origin: http://localhost:3000' http://127.0.0.1:7400/api/v1/approvals` → `{"items":[]}`
- Open `http://localhost:3000/approvals` → all KPI tiles render `—` / `0`, severity rails show empty states

Source: `web/lib/api/approvals.ts:179-201` — `fetchApprovals()` returns `{items:[], source:"live"}` when the gateway responds empty (no fallback to sample data unless gateway is unreachable, which is the right behavior; the bug is upstream — no seed data).

Suggested fix: seed the demo gateway with 3-5 pending approvals (1 HIGH refund, 1 MEDIUM hire-decision, 1 LOW reply-publish) before recording. The `argo demo seed` path in `mission-demo-seed-data-bootstrap` is the right vehicle.

---

### F2 [P0] — Audit feed is 100% self-traffic (SPA observing itself)

Every audit row returned by `GET /api/v1/audit?limit=10` is `verb:"http.request"`, `action_type:"audit.detail.viewed"` or `"audit.replay"`, `worker:{id:"", name:"unknown"}`. There are zero agent-execution rows (no `candidate_screened`, no `email_drafted`, no `policy_refusal`, no `spend_approval_asked` — none of the verbs the runbook scene assumes). The audit page therefore renders a wall of meta-noise: 20 rows of "unknown · http.request" that exist only because the audit page was loaded a few times.

Reproduction:
- `curl -s -H 'Origin: http://localhost:3000' 'http://127.0.0.1:7400/api/v1/audit?limit=5' | jq '.items[].verb'` → all `"http.request"`
- Open `/audit` — every row label is `http.request`, every actor is `unknown`

Source: this is a gateway-side / seed-data issue, not a SPA bug. The SPA's redactor and ActivityRow component cope correctly — they just have nothing meaningful to render.

Suggested fix: seed the audit log with 10-20 real agent-execution rows from the same `argo demo seed` invocation that fixes F1, so the audit page tells a story.

---

### F3 [P0] — Hash chain hover has no data to bind to (gateway list omits hash fields)

The audit page's design lock (B3-Etherscan, memory `project_audit_page_design_locked`) makes the per-row chain hash the headline forensic signal — the "see Etherscan-style hash chain hover" verb in the runbook. The `HashCell` component (`web/components/skycot/ActivityRow.tsx:213-260`) only renders when `activity.hash` is non-empty. But the gateway response for `/api/v1/audit?limit=N` has **no hash field at all** — fields returned are: `action_type, approver, channel, cost, data_classification, id, status, timestamp, verb, worker`. Hash data DOES exist server-side (the `/audit/{id}/replay` endpoint returns `actual_self_hash` and `expected_self_hash`) but the list endpoint doesn't surface it.

Reproduction:
- `curl -s 'http://127.0.0.1:7400/api/v1/audit?limit=2' | python3 -c "import json,sys; print([k for k in json.load(sys.stdin)['items'][0] if 'hash' in k.lower()])"` → `[]`
- Compare with `web/lib/api/audit.ts:194-218` which normalises `selfHash ?? self_hash`, `parentHash ?? parent_hash`, `sampleHash ?? sample_hash` — the SPA is ready; gateway isn't sending them.

Source: gateway list-endpoint serializer (likely in `crates/argo-cli/src/handlers/gateway/routes_audit*.rs`). The detail endpoint correctly returns `provenance.hash_chain_anchor` (verified via curl).

Suggested fix: add `self_hash`, `parent_hash` to the row shape emitted by the list endpoint. Until that ships, the hash-chain hover is a sample-audit-fixture-only feature.

---

### F4 [P0] — Audit-detail drill-in returns shape-but-no-content envelope

`GET /api/v1/audit/{id}` returns the full 8-step decision-chain skeleton (`Memory loaded / Skill chosen / Plan candidates / Plan selected / Policy evaluated / Approval / Execution / Outcome`) but **every value is null or empty string**: `body:""`, `details:{snapshot_id:null}`, `details:{skill_id:null, skill_trust:null, skill_version:null}`, `provenance:{model:{id:"", version:""}, policy:{hash:"", id:"", version:""}, prompt_template:{hash:"", id:""}, signature:"", skill:{id:"", version:""}}`. Only `hash_chain_anchor` is populated.

If the camera drills into one of the existing `http.request` rows in Scene 6, the audit-detail page will render 8 section titles each followed by an empty body — looks broken without actually 500-ing. The redaction banner will additionally claim "PII redacted" which adds confusion (there is no PII to redact; the fields are simply empty).

Reproduction:
- `curl -s 'http://127.0.0.1:7400/api/v1/audit/019e646a13137e118aba9e7bb3f9cc67' | jq '.detail.decision_chain[0]'` → `{"body":"","details":{"snapshot_id":null},"n":1,"title":"Memory loaded"}`

Source: same gateway-side gap as F2 — these are real audit rows for `http.request` traffic, which legitimately has no skill/policy/prompt content. Until agent-execution rows exist (F2 fix), the drill-in is empty by design.

Suggested fix: paired with F1+F2 seeding — the agent-execution rows seeded for F2 will fill the decision-chain.

---

### F5 [P0] — Recording-day footgun: `127.0.0.1:3000` SPA silently loses CORS

The running gateway (verified by direct curl) is in **production mode** — its CORS allowlist exact-matches `http://localhost:3000` only. A request from `http://127.0.0.1:3000` returns HTTP 200 but with **no `Access-Control-Allow-Origin` response header**, so the browser blocks the response. Every page renders its shell + empty state with zero data. Nothing in the visible UI explains why.

This is a screen-recording trap: the operator's address bar autocompletes to `127.0.0.1:3000` because that's what Chrome remembered from prior dev sessions, the screen looks like a fresh-install workspace, and the demo silently dies.

Reproduction:
- `curl -s -i -H 'Origin: http://127.0.0.1:3000' http://127.0.0.1:7400/api/v1/approvals | grep -i allow-origin` → (no output)
- `curl -s -i -H 'Origin: http://localhost:3000' http://127.0.0.1:7400/api/v1/approvals | grep -i allow-origin` → `Access-Control-Allow-Origin: http://localhost:3000`
- Confirm production mode: `crates/argo-cli/src/handlers/gateway/cors.rs:170-176` — `allow_dev_loopback_origins = !production_mode`; when this flag is false the dev-loopback path at line 116 short-circuits.

Source: `crates/argo-cli/src/handlers/gateway/cors.rs:147-159` defines `is_dev_loopback_origin` correctly but the gating flag forces exact-match.

Suggested fix: either (a) explicitly add `http://127.0.0.1:3000` to the gateway's `[gateway.cors].allowed_origins` config alongside `localhost:3000`, or (b) confirm the demo gateway boots with `production_mode=false` so the dev-loopback path admits `127.0.0.1:*`. Quickest in-the-moment workaround: tape a Post-It to the operator's monitor saying "always type localhost, never 127.0.0.1".

---

### F6 [P1] — Replay endpoint is real and works; this is the strongest Scene-6 moment

Positive finding (no defect): `POST /api/v1/audit/{id}/replay` genuinely re-hashes the row server-side and returns `{actual_self_hash, expected_self_hash, verdict, seq, replayed_at}`. For row `019e646a13137e118aba9e7bb3f9cc67` both hashes are `617b53aed2f8ff9e2ee91862173b167351123cf05baebd5066840a4803593038` and `verdict:"same"`. The SPA path `web/lib/api/replay.ts:254-272` calls this endpoint live (no fixture short-circuit unless `isAuthBypassed()` is set; current env has `ARGO_GATEWAY_AUTH_BYPASS=1` WITHOUT the `NEXT_PUBLIC_` prefix, so the SPA-side check is false and the live path runs).

If F1-F4 are seeded with real data, the Replay drill-in becomes the single most-real moment in the entire demo: camera clicks Replay → SPA POSTs → gateway re-hashes → `verdict: same` flashes up with both hashes side-by-side. That IS the trust pitch made executable.

Mention: cite this beat in the Scene 6 narration as "the hash that was anchored at write-time matches the hash we just recomputed against the row" — that's defensible end-to-end.

---

### F7 [P1] — Auth-bypass env var name mismatch: shell sets `ARGO_GATEWAY_AUTH_BYPASS`, SPA checks `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`

`web/.env.local` has `ARGO_GATEWAY_AUTH_BYPASS=1` but the SPA-side check at `web/lib/api/audit.ts:60-65`, `web/lib/api/audit-detail.ts:60`, `web/lib/api/replay.ts:83`, and `web/lib/api/approvals.ts` all look for `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. The `NEXT_PUBLIC_` prefix is required for client-bundle exposure in Next.js — without it the SPA never sees the variable.

Net effect: the `.env.local` setting is a no-op for the SPA. Today this is accidentally helpful (it forces live-gateway paths, which is what Scene 6 needs), but it's a footgun for whoever next tries to enable bypass for a different recording.

Reproduction:
- `cat web/.env.local` → `ARGO_GATEWAY_AUTH_BYPASS=1`
- `grep -n "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS\|ARGO_GATEWAY_AUTH_BYPASS" web/lib/api/audit.ts` → checker uses `NEXT_PUBLIC_` prefix

Suggested fix: rename in `web/.env.local` to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` (or 0, depending on demo intent), or document this in a `web/.env.example`.

---

### F8 [P1] — Demo banner claim "showing sample data" is false on real-data path

The top-of-page status banner on every in-app page reads "Demo mode — showing sample data. Sign in to see your own workspace." But the page is actually showing live gateway data (empty rows, real `http.request` audit log) — not sample data. This is a small but cumulative trust-leak: the banner sets the wrong expectation, and a savvy viewer will notice the contradiction (banner says "sample" but the rows look like real http traffic).

Reproduction:
- Open `/approvals` while gateway is live; banner says "showing sample data" but `/api/v1/approvals` is returning the real (empty) inbox.

Suggested fix: branch the banner copy on data source — "Demo mode — gateway is empty; nothing to show yet" when arrays are empty; "Demo mode — showing seeded sample data" only when the fallback fixture path activated.

---

### F9 [P1] — `/api/v1/workers` returns 404; only `/api/v1/team` has data, and it's "Maya" not the runbook's 5 personas

`GET /api/v1/workers` returns `{"error":"not found"}` (route doesn't exist). The real worker data lives at `/api/v1/team` and contains a single worker — Maya, `vertical: marketing-intern`. Scenes 1-5 of the runbook expect Mariah / Peta / Priya / Jamal / Nori. The Audit/Approvals pages don't directly query `/api/v1/workers` (they get worker info embedded in audit rows or approval items), so this is out of strict Scene-6 scope, but it's the upstream root cause of why audit rows have `worker.name:"unknown"`: the workers aren't seeded, so any audit row that didn't come from an agent-execution context can't attribute a worker.

Reproduction:
- `curl http://127.0.0.1:7400/api/v1/workers` → 404 + `{"error":"not found"}`
- `curl http://127.0.0.1:7400/api/v1/team` → 1 worker, Maya, marketing-intern vertical

Suggested fix: confirm whether `/api/v1/workers` is intended to exist; if so, route is missing; if not, the SPA's `web/lib/api/workers*.ts` should call `/api/v1/team` instead. The 4 workers Scene 6 needs (Mariah et al) must be seeded via the same bootstrap that fixes F1+F2.

---

### F10 [P1] — Cross-tenant header is accepted but appears to be ignored

Sending `X-Argo-Tenant-Id: definitely-not-yours` on `/api/v1/audit` returns `{"items":[]}` (200) — same as the empty real response. We can't tell from this single probe whether the header was honored (the legitimate tenant has zero agent-execution rows so the visible result is identical) or silently dropped. Operator should verify cross-tenant isolation with at least two seeded tenants before relying on the "multi-tenant by construction" claim in Scene 6.

Reproduction:
- `curl -s -H 'Origin: http://localhost:3000' -H 'X-Argo-Tenant-Id: definitely-not-yours' 'http://127.0.0.1:7400/api/v1/audit?limit=2'` → `{"items":[]}`
- Compare with no header — also `{"items":[]}` (because the legit tenant is also empty).

Suggested fix: seed two tenants with distinguishable audit data, then re-probe — confirm wrong-tenant requests return empty / 403 / not-found.

---

### F11 [P2] — `/api/v1/skills/learned-this-month` consistently 404s

Background SPA traffic on every page navigation triggers `GET /api/v1/skills/learned-this-month` which returns 404. Not Scene 6 directly, but it pollutes the dev-server logs the operator will tail during recording, and it's the kind of 404 that AppInsights / Sentry will flag in any real deployment.

Reproduction:
- Open any in-app page → network log shows `/api/v1/skills/learned-this-month → 404`

Suggested fix: either stub the endpoint server-side, gate the call behind a feature flag, or remove the client-side call until the endpoint ships.

---

### F12 [P3] — Dev-server console drowned in React-DevTools install prompts

`preview_console_logs` returns ~500 lines, ~95% of which are "Download the React DevTools for a better development experience" repeated for every component-tree render. Drowns out real warnings/errors when debugging. Not user-visible; only an audit-friction nit.

Suggested fix: silence the `__REACT_DEVTOOLS_GLOBAL_HOOK__` setup hint in `web/app/layout.tsx` for dev builds, or set `process.env.SUPPRESS_REACT_DEVTOOLS_HOOK=1` in `package.json` scripts.

---

## Resolved since last audit

The prior Scene-6 audit at `docs/skycot/ux-audits/2026-05-26-2309/scene-6-claim-6-trust-and-approvals.md` (not re-read in this run; auditor scope was a fresh pass).

---

## Orientation score: 38 / 100

The shells are well-built — empty states are honest, no fabrication, the locked B3-Etherscan and E-ops-dashboard designs render faithfully — but the data layer is so thin that none of Scene 6's promises can be kept on real gateway data. The single shining moment is the Replay endpoint (F6): genuinely real, genuinely on-brand. Everything else needs seed data before the camera rolls. The CORS host-mismatch trap (F5) deserves a Post-It on the recording-day monitor regardless of seeding work.
