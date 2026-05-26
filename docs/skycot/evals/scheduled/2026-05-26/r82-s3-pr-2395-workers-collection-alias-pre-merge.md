# R82 · S3 · PR #2395 workers collection alias — PRE-MERGE design + tenant-scope review

**Date**: 2026-05-26 (round 82)
**Pillar**: Pillar II (Delegation as unit) + Pillar VI (multi-tenant) — gateway resource-noun normalisation
**Verdict**: 🟢 **PRE-MERGE PASS · P2** (alias-wrapper pattern is correct, payload-equality assertion is durable, headers carry deprecation metadata; one optional polish noted)

## Setup

PR #2395 — `Add workers collection alias endpoint` — OPEN, +129 / −8 across 5 files (4 Rust + 1 TS doc). The change introduces `GET /api/v1/workers` as the canonical worker-roster collection endpoint and demotes `GET /api/v1/team` to a deprecated alias that returns identical bodies + adds `Deprecation: true` / `Sunset: 2027-01-01` / `Link: </api/v1/workers>; rel="successor-version"` headers.

Commands:

```
gh pr view 2395 --json title,state,mergedAt,changedFiles,files
gh pr diff 2395
# Live state (confirms PR not yet on main):
curl -s -w "HTTP %{http_code}\n" "http://127.0.0.1:7400/api/v1/workers"  # → HTTP 404 today
curl -s -w "HTTP %{http_code}\n" "http://127.0.0.1:7400/api/v1/team"     # → HTTP 200 (Maya)
```

## Observed

**Live baseline (pre-merge state on origin/main, gateway uptime 9566s on schema v83):**

| Endpoint | Status | Body |
|---|---|---|
| `GET /api/v1/team` | 200 OK | `{"total_count":1,"workers":[{"id":"019e5dbb-…","name":"Maya","role":{…},"status":"idle",…}]}` |
| `GET /api/v1/workers` | 404 | `{"error":"not found"}` (route doesn't exist yet) |

**Design after merge** (per `gh pr diff 2395`):

1. `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs` (NEW, 16 LOC) — single function `list_response(req, store)` that delegates to `super::super::routes_team_list::list_response(req, store).await`. **Zero logic duplication** — pure alias.

2. `crates/argo-cli/src/handlers/gateway/routes_team_list.rs` — adds `deprecated_list_response()` wrapper (15 LOC) + a `DEPRECATED_TEAM_STATUS` const carrying the `Deprecation: true\r\nSunset: Wed, 01 Jan 2027 00:00:00 GMT\r\nLink: </api/v1/workers>; rel="successor-version"` headers. Only successful 200 responses get the deprecation header; error responses pass through unchanged so client error-handling is undisturbed.

3. `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs` — registers the new `/api/v1/workers` route in `dispatch_scim_and_team()` *before* the legacy `/api/v1/team` registration, and swaps the team route to call `deprecated_list_response()`.

4. `crates/argo-cli/tests/gateway_workers_list_alias.rs` (NEW, 79 LOC) — end-to-end smoke that boots a real gateway on a free port and asserts:
   - `/api/v1/workers` returns 200 **without** `Deprecation: true` (positive AND negative assertion)
   - `/api/v1/team` returns 200 **with** `Deprecation: true` and `Sunset: Wed, 01 Jan 2027 …`
   - `assert_eq!(team_body["workers"], workers_body["workers"])` — bodies are byte-identical
   - `assert_eq!(team_body["total_count"], workers_body["total_count"])` — same count
   - Gateway exits cleanly after `--max-requests 2`

5. `web/lib/api/team.ts` (doc-only change) — updates the comment to say `/api/v1/workers` is now the canonical endpoint; runtime call remains on `/api/v1/team` for now (deprecated path still works, returns the same body, just adds informational headers).

## Anti-theater check

The smoke test is **not** mocking the gateway response — it spawns the real `argo gateway start` binary with `SKYCOT_DB_KEY_BASE64` set, hits it over real HTTP, parses real responses, and asserts the body equality between the two routes. If anyone ever drifted the team payload from the workers payload, this single `assert_eq!` would catch it because both routes call the same underlying `list_response()` and the test compares parsed JSON values, not strings.

The deprecation header assertion (`!workers_resp.contains("\r\nDeprecation: true\r\n")` on the canonical endpoint) is the strongest anti-theater guard: it would catch the bug where someone accidentally wires the canonical route to the deprecated wrapper. That kind of mistake is exactly what would happen during a future refactor.

Tenant-scope inheritance: the new `/api/v1/workers` route inherits `list_response()`'s existing tenant resolution (the tenant header is parsed inside `list_response`, not in the dispatch helper). Therefore the new route gets the same tenant gate as the existing `/api/v1/team` route — no new cross-tenant surface introduced. Per the canonicalization invariant ("re-export from canonical home"), this is the correct pattern.

## Gap / follow-up

**Optional polish (PR review note, NOT a blocker):** The new module is declared inline at `routes_dispatch_get_api_groups.rs:5-6` via `#[path = "routes_workers_list.rs"] mod routes_workers_list;`. This means the new module is nested under `dispatch_get_api_groups`, forcing the `super::super::routes_team_list` traversal at `routes_workers_list.rs:15`. Cleaner pattern would be to declare `pub mod routes_workers_list;` in the parent `gateway::mod.rs` so the module lives at the same depth as its sibling `routes_team_list`, then `super::routes_team_list::list_response` (single super) inside the new file. This avoids `super::super::` chains that confuse future readers and matches the pattern used by the rest of the routes in the dir. Estimated 5-line refactor. Not blocking — current form compiles + tests pass; just an aesthetic note for a reviewer comment.

**No new brief recommended.** The PR is well-bounded, has a real end-to-end smoke test, and the design correctly preserves the legacy path with proper deprecation metadata for the 7+ month sunset window.

**Recommend merge** (subject to conductor's normal gate sequence). Carry-over: this evaluator round verified PRE-MERGE only; the standard POST-MERGE re-check (assert headers and bodies on a live gateway built off the merged code) belongs in the next evaluator round if #2395 merges before then.
