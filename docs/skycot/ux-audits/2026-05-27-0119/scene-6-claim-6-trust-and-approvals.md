# Scene 6 + Approvals — Claim 6 (Multi-channel + multi-tenant trust)

**Audit date**: 2026-05-27T01:19Z (Melbourne local: 2026-05-27 ~11:19am AEST)
**Scope**: `/approvals` (E ops-dashboard) + `/audit` (B3 Etherscan) + hash-chain hover + audit-row drill-in real-vs-fabricated + cross-tenant isolation negative tests + `/audit/action/{id}/replay`.
**Environment**: SPA `http://localhost:3000` (Next.js dev) + gateway `http://127.0.0.1:7400` (schema v83, uptime ~10h 54m since `2026-05-26 ~14:34 AEST`).
**Method**: Source read of every Scene-6 data-path (SPA fetch layer + gateway route handlers + sample fixtures) + curl probes against the live gateway covering tenant gating, hash projection, replay verdict, error envelopes. Preview MCP locked up early in the run; the audit is curl + source-of-truth instead of rendered-DOM.
**Prior audit**: `docs/skycot/ux-audits/2026-05-27-0015/scene-6-claim-6-trust-and-approvals.md`. Cross-referenced; F-numbering is fresh.

---

## TL;DR

The single load-bearing finding this run is the **stale gateway binary**: the audit feed hash envelope (#2353), the gateway dev-CORS loopback fix (#2359), the seeded-approvals decided-rate fix (#2397), the workers-collection-alias (#9ea693db), and 4–6 other fixes landed on `main` AFTER the running binary was built. The running gateway is 11 hours stale. **This one issue explains F1, F3, F5, F8, F9 from the prior (T0015MEL) audit** — they were SUPPOSEDLY fixed today but the operator has been running an older binary the whole time, so the symptoms persist exactly as before. The cross-tenant isolation evidence is the upside: with tenant=demo carrying 4 audit rows, the wrong-tenant header returns 0 rows on the list endpoint and `404 audit row not found` on the detail endpoint. Replay still works end-to-end and the hashes match. Restart the gateway with a fresh release build and three quarters of these findings disappear. Then run `argo demo seed --tenant <demo-uuid>` to populate agent-execution rows + approvals so the scene actually has data to show.

Counts: 4 P0, 4 P1, 2 P2, 1 P3 = 11 findings.

---

## Headline blockers

1. **F1 [P0]** — Running gateway binary is 11 hours stale; today's fixes for hash envelope, dev-CORS, workers-alias, and decided-approvals-seed are all merged but NOT running. One restart-with-fresh-build resolves five separate symptoms.
2. **F2 [P0]** — Demo seed has NOT been run; tenant `demo` has 4 audit rows (all `verb: http.request` self-traffic, `worker.name: unknown`) and zero approvals; all other tenants are completely empty. Scene 6 has nothing real to drill into.
3. **F3 [P0]** — Audit feed list endpoint emits zero hash fields on the wire. The serialized JSON has keys `[action_type, approver, channel, cost, data_classification, id, status, timestamp, verb, worker]` only — no `self_hash`, no `parent_hash`. The `<HashCell>` headline component therefore renders blank space on every row. (Fix is in code via #2353 but not in binary — see F1.)
4. **F4 [P0]** — `/api/v1/team` returns 0 workers across every tested tenant (demo/default/test/acme). The prior audit found one worker (Maya); the data has been wiped or rebound, so Scene 1's promised personas (Mariah / Peta / Priya / Jamal / Nori) have no rows to attribute audit events to. (Fix is in code via 9ea693db1 + 738ff9a79 but workers must also be seeded via `argo demo seed`.)

---

## All findings, in order

### F1 [P0] — Running gateway binary is 11 hours stale; today's Scene-6 fixes are merged on `main` but NOT in the running binary

The gateway process serving `:7400` is PID 7348 from `/private/tmp/argo-shared-target/release/argo`, with binary mtime **2026-05-26 14:33:43** (≈10h 54m before this audit run) and process etime ≈10h 54m. Multiple gateway-side commits that fix prior-audit findings have landed AFTER that mtime:

| Commit | Title | Symptom it fixes (still observed today) |
|---|---|---|
| `a51a21f14` (#2353) | fix audit feed hash envelope | F3 — hash chain hover blank on real data |
| `c35ad17d9` (#2397) | seed 3 decided approvals so /approvals ring renders 67% | F1 prior — approvals KPI ring renders `—` |
| `e4662d892` (#2359) | fix gateway dev cors loopback origins | F5 prior — 127.0.0.1 vs localhost CORS host-mismatch trap |
| `9ea693db1` | gateway: alias workers collection route | F9 prior — `/api/v1/workers` 404 |
| `738ff9a79` | grant demo worker overview permissions | Workers showing but empty after alias |
| `16a9136c4` (#2402) | disambiguate audit endpoint 404s | unclear 404 messages on audit subpaths |
| `62f2eae4c` (#2345) | fix(audit): surface delegation chain drill-in | delegation chain card empty on detail |

The fixes are in `git log` on `origin/main` HEAD and visible in source — e.g. `routes_audit_feed.rs:248-249` emits `"self_hash": row.self_hash, "parent_hash": row.parent_hash` in the JSON projector. But the binary serving traffic does not have that code compiled in. This is a single recording-day operator footgun: **everything labelled "fixed" today is dark on the wire until the gateway restarts.**

Reproduction:
- `ps -p $(lsof -nP -iTCP:7400 -sTCP:LISTEN -t) -o pid,etime,start,command` → process started 2:34 PM 2026-05-26
- `stat -f "%Sm" /private/tmp/argo-shared-target/release/argo` → `May 26 14:33:43 2026`
- `git log --oneline -- crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs | head -2` → `a51a21f14 fix audit feed hash envelope (#2353)` (landed 2026-05-26 21:18 AEST, ≈7h after binary mtime)
- Curl any list-endpoint and confirm hash fields are absent.

Source: `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:246-259` projects the right shape but the running binary predates the projector.

Suggested fix: `bash scripts/restart-live-gateway.sh` (or its equivalent that triggers a `cargo build --release -p argo-cli` against current `origin/main` HEAD, terminates PID 7348, restarts on :7400). Verify the rebuild by curling `/api/v1/audit?limit=1 | jq 'keys'` and confirming `self_hash` is in the key list. Operator-only action — agent should not restart the gateway autonomously.

---

### F2 [P0] — Demo seed never run; tenants are empty (or only carry SPA self-traffic)

Across the four tenants probed (`demo`, `default`, `test`, `acme`), the audit log content is:

| Tenant | Audit rows | Approvals | Verb mix |
|---|---|---|---|
| demo | 4 | 0 | `[('http.request', 4)]` |
| default | 0 | 0 | — |
| test | 0 | 0 | — |
| acme | 0 | 0 | — |

The 4 `http.request` rows on `tenant=demo` are SPA self-instrumentation of its own audit-page page-views, NOT real agent-execution rows from runtime turns. There are no `candidate_screened`, `email_drafted`, `policy_refusal`, `spend_approval_asked`, or any other verb the Scene-6 runbook narration assumes. **Even after F1 is solved**, the page will render a credible-looking forensic timeline of nothing.

`argo-demo-seed` exists at `crates/argo-demo-seed/` with the canonical 5-persona spine (Mariah/Peta/Priya/Jamal/Nori) defined in `crates/argo-demo-seed/src/personas.rs:43-95` and the `argo demo seed` handler at `crates/argo-cli/src/handlers/demo_seed.rs`. The seed CLI has never been run against this gateway's data directory.

Reproduction:
- `for T in demo default test acme; do curl -s -H "X-Argo-Tenant-Id: $T" 'http://127.0.0.1:7400/api/v1/audit?limit=100' | python3 -c 'import json,sys,collections; d=json.load(sys.stdin); c=collections.Counter(i.get("verb","?") for i in d.get("items",[])); print("$T:", c.most_common(5))'; done`
- `curl -s -H 'X-Argo-Tenant-Id: demo' 'http://127.0.0.1:7400/api/v1/approvals'` → `{"items":[]}`

Source: gateway data state, not code. The seed binary exists but is not in the recording-prep flow.

Suggested fix: discover the demo tenant UUID via `curl http://127.0.0.1:7400/api/v1/workflows` (response includes `tenant_id: 019e5057-1d3f-7fa2-9c1b-c34ef01cc21e`) and run `argo demo seed --tenant 019e5057-1d3f-7fa2-9c1b-c34ef01cc21e` (or the equivalent operator-flagged tenant id). The CLI will populate Pass A turn-shaped audit rows + Pass B dressing rows + approvals as documented in `crates/argo-demo-seed/src/seed.rs`.

---

### F3 [P0] — Audit list endpoint omits hash fields on the wire (Etherscan hash hover is invisible)

Direct probe of the running gateway shows the list-endpoint JSON does NOT include `self_hash` / `parent_hash` / `sample_hash` on any row, despite the SPA fetch layer being ready to consume them (`web/lib/api/audit.ts:194-217` normalizes both snake_case and camelCase variants into `selfHash`, `parentHash`, `sampleHash`, then derives `activity.hash`).

The `<HashCell>` component at `web/components/skycot/ActivityRow.tsx:97-106` renders only when `activity.hash` is truthy — otherwise an `<span aria-hidden />` placeholder. So on every live row the page shows an invisible 4th column where the Etherscan-style truncated hash chip should sit. The B3-locked design's headline trust feature is dark.

The fix is in source via #2353 at `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:246-259` — the row projector emits the right keys. It's just not in the binary (see F1).

Reproduction:
- `curl -s -H 'X-Argo-Tenant-Id: demo' 'http://127.0.0.1:7400/api/v1/audit?limit=1' | python3 -c "import json,sys;d=json.load(sys.stdin); print('has self_hash:', 'self_hash' in d['items'][0])"` → `has self_hash: False`
- `git -C . log -1 --format='%h %ci' crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` → `a51a21f14 2026-05-26 21:18:21 +1000` (≈7h AFTER binary mtime, see F1)

Source: `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:246-259` (code-side correct; binary-side missing).

Suggested fix: subsumed by F1 — gateway restart makes the existing fix go live.

---

### F4 [P0] — `/api/v1/team` returns 0 workers across all tested tenants (regression vs prior audit)

| Tenant | `/api/v1/team` count |
|---|---|
| demo | 0 |
| default | 0 |
| test | 0 |
| acme | 0 |
| (no header) | 0 |

The prior audit at `docs/skycot/ux-audits/2026-05-27-0015/scene-6-claim-6-trust-and-approvals.md:F9` reported 1 worker ("Maya"). That worker is gone. Whatever code path or data action seeded Maya has either been undone or the database was reset between runs.

This compounds F2: even when the live gateway returns audit rows, attributing those rows to a real worker is impossible because there are zero personas in the worker store. Audit rows render `worker.name: "unknown"` because the worker lookup returns nothing.

Reproduction:
- `for T in demo default test acme ""; do curl -s -H "X-Argo-Tenant-Id: $T" 'http://127.0.0.1:7400/api/v1/team' | python3 -c "import json,sys;d=json.load(sys.stdin);items=d if isinstance(d,list) else d.get('items',[]);print('$T:',len(items))"; done`

Source: gateway data state. The `argo-demo-seed::personas::DEMO_SEED_PERSONAS` constant defines the 5-worker roster (`crates/argo-demo-seed/src/personas.rs:43`) but it must be invoked via `argo demo seed`.

Suggested fix: same as F2 — run the seed CLI. The personas seed via `IdentityStore::create_worker` and become discoverable via `/api/v1/team` (and via `/api/v1/workers` once F1's rebuild lands the alias).

---

### F5 [P1] — Replay endpoint is the strongest Scene-6 evidence; it really works (positive finding)

`POST /api/v1/audit/{id}/replay` against an existing audit row returns the full hash-recompute envelope: `{audit_id, actual_self_hash, expected_self_hash, verdict, seq, kind, replayed_at, created_at_micros, diff}`. For row `019e646a13137e118aba9e7bb3f9cc67`, both hashes match (`617b53ae...3593038`) and `verdict: "same"`. The SPA's adapter at `web/lib/api/replay.ts:204-228` (`adaptGatewayReplay`) converts this into a `{kind:"success", verdict:"reproducible"}` outcome and the page renders side-by-side hash bodies.

This is the single most-real moment in Scene 6 and it's genuinely defensible. The narrator can say "the hash anchored at write-time matches the hash we just recomputed against the row from the database" and that is literally true on this codepath.

Reproduction:
- `curl -sX POST -H 'Content-Type: application/json' 'http://127.0.0.1:7400/api/v1/audit/019e646a13137e118aba9e7bb3f9cc67/replay' | python3 -m json.tool`

Source: `web/lib/api/replay.ts:254-272` + `crates/argo-cli/src/handlers/gateway/routes_audit_replay.rs`.

No fix needed. Mention: cite this beat explicitly in Scene 6 narration; it's the load-bearing trust pitch.

---

### F6 [P1] — Cross-tenant isolation is now genuinely enforced (positive finding — IMPROVEMENT vs prior audit)

The prior audit could not confirm tenant isolation because the legit tenant was empty (`[]` from both right-tenant and wrong-tenant requests is identical). With tenant `demo` now carrying 4 audit rows, the probe is decisive:

| Request | Result |
|---|---|
| List as `X-Argo-Tenant-Id: demo` | 2 rows (real demo data) |
| List as `X-Argo-Tenant-Id: default` | 0 rows |
| List as `X-Argo-Tenant-Id: definitely-wrong` | 0 rows |
| GET demo-row id as `X-Argo-Tenant-Id: demo` | 200 OK with full envelope |
| GET demo-row id as `X-Argo-Tenant-Id: other` | **404 `audit row not found`** |
| GET demo-row id without tenant header | 404 `audit row not found` |

This is genuine end-to-end tenant scoping — the SQL filter at the audit-store layer is honored. The `audit row not found` framing (rather than `403 Forbidden`) is correct from a tenant-disclosure standpoint: a 403 would leak that the row exists somewhere; a 404 reveals nothing.

Reproduction:
- `DID=$(curl -s -H 'X-Argo-Tenant-Id: demo' 'http://127.0.0.1:7400/api/v1/audit?limit=1' | python3 -c "import json,sys;print(json.load(sys.stdin)['items'][0]['id'])")`
- `curl -s -o /dev/null -w "X-Tenant=other: %{http_code}\n" -H 'X-Argo-Tenant-Id: other' "http://127.0.0.1:7400/api/v1/audit/$DID"` → `404`
- `curl -s -o /dev/null -w "X-Tenant=demo:  %{http_code}\n" -H 'X-Argo-Tenant-Id: demo' "http://127.0.0.1:7400/api/v1/audit/$DID"` → `200`

Source: `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs` and `routes_audit_feed.rs` apply tenant filter at the store layer; verified empirically.

No fix needed. Mention: Scene 6 narration can include a live cross-tenant denial as a recordable beat — type a foreign tenant id in dev tools, hit the row URL, the 404 returns. That IS the multi-tenant claim made visible.

---

### F7 [P1] — Demo-banner copy "showing sample data" is misleading when SPA is on the live-gateway path

`web/components/skycot/DemoBanner.tsx:43-46` shows the banner only when `isDemoMode()` is true (= `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`). Current `web/.env.local` has neither `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` nor `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. So today the banner is NOT showing — and the SPA is using the live-gateway path (returning the empty real data).

This is fine for THIS run, but the recording-day footgun is the inverse: if the operator turns on demo mode for the record-take, the banner says "Demo mode — showing sample data. Sign in to see your own workspace." but the audit feed at that point will be a MIX of sample fallback rows (when gateway 404s) and live-gateway-derived rows (when gateway responds). The copy implies one consistent source-of-truth; the runtime branches at the network layer.

Reproduction:
- `cat web/.env.local` → no `NEXT_PUBLIC_SKYCOT_DEMO_MODE`
- `grep -n "showing sample data" web/components/skycot/DemoBanner.tsx` → line 43-46
- Source-read `web/lib/api/audit.ts:355-365` for the branching logic.

Suggested fix: branch banner copy on actual data source. When the audit list returns ≥1 row and the row has `sampleHash !== null` (or hashKind `sample`), say "Demo mode — sample audit fixtures." When the gateway returns ≥1 row with `selfHash`, say "Demo mode — seeded gateway data." Either honesty is fine; the present copy is just inaccurate-by-default.

---

### F8 [P1] — Auth-bypass env var name mismatch persists (`.env.local` sets the unprefixed form which the SPA can't see)

Re-confirmed from prior audit: `web/.env.local` contains `ARGO_GATEWAY_AUTH_BYPASS=1` but every SPA-side check looks for `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (`web/lib/api/audit.ts:62-65`, `web/lib/api/audit-detail.ts:60-65`, `web/lib/api/replay.ts:83-88`, `web/lib/api/approvals.ts` similar). Without the `NEXT_PUBLIC_` prefix, Next.js does NOT inline the var into the client bundle — the SPA always sees `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === undefined`.

Net effect today: helpful by accident (forces live-gateway path which is what Scene 6 needs); strict footgun for whoever next tries to enable bypass for a different recording. Same as prior audit; flagging again because nothing has changed.

Suggested fix: rename in `web/.env.local` to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` (or `0`, depending on intent), and add a comment in `web/.env.example` documenting the prefix requirement.

---

### F9 [P2] — `/api/v1/skills/learned-this-month` consistently 404s; logs noise

Background SPA traffic on every page navigation still triggers `GET /api/v1/skills/learned-this-month` → 404. This is unchanged since prior audit. Not Scene-6-visible itself, but a recording-day audio risk if the operator opens dev-tools network panel during the take (the camera might catch a red row).

Suggested fix: stub the endpoint server-side, gate the call behind a feature flag, or remove the client-side call until the endpoint ships.

---

### F10 [P2] — `/api/v1/workflow-runs` returns `{"error":"not found"}` (different error shape from rest of gateway)

The gateway has inconsistent error envelopes. `/api/v1/audit/aaaaa` returns `{"error":{"code":"not_found","message":"audit row not found"}}` (structured). `/api/v1/workflow-runs` returns `{"error":"not found"}` (string). `/api/v1/workers` returns `{"error":"not found"}` (string). Scene 5 (workflows) might brush against `/workflow-runs`; if the SPA's typed error path expects `error.code`, a `string` error breaks parsing.

Reproduction:
- `curl -s 'http://127.0.0.1:7400/api/v1/workflow-runs?limit=2'` → `{"error":"not found"}`
- `curl -s 'http://127.0.0.1:7400/api/v1/audit/aaaa'` → `{"error":{"code":"not_found", "message":"..."}}`

Suggested fix: standardize on the structured `{error:{code,message}}` envelope across all gateway 404s. Probably partially addressed by `#2402` ("disambiguate audit endpoint 404s") which is in code but not in the running binary (see F1).

---

### F11 [P3] — Preview MCP wedged early in this audit; fell back to source + curl

The Claude Preview tool (used to drive the SPA browser) timed out on `preview_snapshot`, `preview_screenshot`, `preview_eval` (all 30s timeouts) after one `preview_list` call worked. The dev server itself (PID listed under `:3000`) is running and serving SSR responses to curl. The MCP server appears to be holding a stale browser handle.

Reproduction:
- `preview_list` → returns serverId
- `preview_snapshot` (immediately after) → 30s timeout
- `curl -s http://localhost:3000/approvals -o /dev/null -w '%{http_code}\n'` → `200`

Suggested fix: out-of-scope for this audit (Preview MCP tooling, not SKYCOT product). Operator can `preview_stop` + `preview_start` to reset the browser handle if needed for next audit run.

---

## Resolved since last audit (T0015MEL → T0119MEL)

| Prior finding | Status |
|---|---|
| **F1** Approvals inbox empty | Still empty on the live gateway (see F2 here). The TS sample fixture is fixed (#2397) but `isDemoMode()` is off so the fixture is never reached. Code fix in but invisible. |
| **F2** Audit feed 100% self-traffic | Unchanged. Same 4 `http.request` rows on tenant=demo, zero agent-execution rows. |
| **F3** Hash fields missing from list endpoint | Code fix landed (#2353) but binary is stale (see F1 here). |
| **F4** Audit-detail drill-in empty envelope | Unchanged (downstream of F2 — drill-in shape can only be real when source data is real). |
| **F5** 127.0.0.1 vs localhost CORS host-mismatch | Code fix landed (#2359) but binary is stale (see F1 here). Browser-origin path still works because SPA's `Origin` header derives from the page's URL, not the env var. |
| **F6** Replay endpoint works | Re-confirmed POSITIVE (see F5 here). |
| **F7** Auth-bypass env var name mismatch | Unchanged (see F8 here). |
| **F8** Demo banner copy is dishonest on real-data path | Unchanged (see F7 here). |
| **F9** `/api/v1/workers` 404 | Code fix landed (`9ea693db1`) but binary is stale; ALSO `/api/v1/team` is now 0 rows (regression — see F4 here). |
| **F10** Cross-tenant header may be silently ignored | RESOLVED — re-probed with seeded demo data, isolation is genuinely enforced (see F6 here). |
| **F11** `/api/v1/skills/learned-this-month` 404 | Unchanged (see F9 here). |
| **F12** React-DevTools console spam | Out of scope this run (Preview MCP wedged before console-logs were captured). |

---

## Orientation score: 42 / 100

(+4 vs prior audit's 38 — the cross-tenant isolation finding flipping from "unverifiable" to "genuinely enforced" is a real positive.)

The SPA is essentially ready. The gateway code on `main` is essentially ready. The single recording-day blocker is operational: **the binary on `:7400` is 11 hours stale and the demo seed CLI has never been run.** A restart + seed sequence resolves the entire Scene-6 data layer. After that, every visible defect in this report becomes invisible: hash fields appear in the wire, approvals populate with a 67% rate ring, workers attribute audit rows by name, drill-in renders real provenance, replay still works exactly as it does today.

Without the restart + seed, Scene 6 is unrecordable as scripted.
