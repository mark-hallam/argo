# fix-audit-list-kind-query-param-ignored

**Owner**: claude or codex.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-list-kind-query-param-ignored`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/scene-1-and-2-claim-1-and-2.md` F4 [P0] + F13 [P3]).

🟡 **P2 — Gateway `/api/v1/audit?kind=X` silently ignores the filter and returns ALL rows. The SPA URL key is `?action_type=X` (which works), but the runbook narration + any external operator using the public API documentation reaches for `?kind=` (matches the audit row's `kind` field naming convention used elsewhere). A filter that silently returns all rows is worse than a filter that errors — it gives the caller a false confidence that no rows of that kind exist.**

## Root cause

Verified via curl during audit:

```
curl -s 'http://127.0.0.1:7400/api/v1/audit?kind=worker_duty.fired'
# returns 20 rows (ALL recent rows, kind filter ignored)

curl -s 'http://127.0.0.1:7400/api/v1/audit?action_type=worker_duty.fired'
# returns {"items": []} (filter honored, no rows of that kind)
```

The gateway handler accepts the request, doesn't recognize the `kind` query-param, doesn't 400, returns everything. The SPA-side filter helper at `web/lib/filters/audit.ts:76-80` uses `action_type` as the URL key, so the SPA path works fine — but any external caller (runbook reader, partner integration, jq pipeline, CLI script) hitting the public API with `?kind=` gets the "looks empty" wrong-answer.

Secondary surface: the v1 demo runbook `docs/skycot/demos/v1-mission-mvp-runbook.md` writes `Filter by kind = worker_duty.fired` — narrating a URL spelling the gateway doesn't honor. The runbook should use `action_type=` (matching the SPA) OR the gateway should honor both.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/scene-1-and-2-claim-1-and-2.md` F4 [P0] (narrow slice — kind filter ignored; the broader "no rows of that kind exist" half is covered by `fix-audit-feed-action-kind-variety` and `fix-demo-seed-emit-worker-hired-audit-rows`)
- `docs/skycot/ux-audits/2026-05-27-0015/scene-1-and-2-claim-1-and-2.md` F13 [P3] — Audit page filter URL key (`action_type`) vs runbook vocabulary (`kind`)

## Fix recipe

Two coordinated changes; ship both:

**Gateway side** (pick one):

A. **Accept both names, prefer `kind`** (recommended — matches the row schema where the field is named `kind` / `action_type`):
   - In the gateway audit-list handler (likely `crates/argo-cli/src/handlers/gateway/routes_audit_list.rs` — verify with `rg -n 'fn handle_audit' crates/argo-cli/src/handlers/gateway/`), accept `?kind=`, `?action_type=`, and (if convenient) `?verb=` as synonyms.
   - Return 400 `unknown_query_param` for any other unrecognised key (so future drifts surface loudly).

B. **Honor `kind` only, deprecate `action_type`** — adds a breaking change to the SPA; skip unless the operator wants to rename the row field too.

**Runbook side**:
   - `docs/skycot/demos/v1-mission-mvp-runbook.md` — change "Filter by `kind = worker_duty.fired`" → "Filter by `action_type = worker_duty.fired`" (or leave as `kind` if option A ships).

**Regression test**:
   - Add a gateway integration test that asserts:
     - `GET /api/v1/audit?kind=worker_duty.fired` returns 200 with only rows where `kind === "worker_duty.fired"`.
     - `GET /api/v1/audit?action_type=worker_duty.fired` returns 200 with only rows where `action_type === "worker_duty.fired"`.
     - `GET /api/v1/audit?gibberish=foo` returns 400 (or 200 with a `warnings: ["unknown_query_param"]` envelope — operator pick).

## Acceptance criteria

- [ ] `curl 'http://127.0.0.1:7400/api/v1/audit?kind=worker_duty.fired'` returns only rows of that kind (or 0 rows if the seed has none — the point is the filter is honored).
- [ ] `curl 'http://127.0.0.1:7400/api/v1/audit?action_type=worker_duty.fired'` returns the same result.
- [ ] Unknown query params either 400 or surface a warning envelope.
- [ ] Runbook reads consistently with the gateway behavior.
- [ ] `cargo test -p argo-cli routes_audit_list` passes (new test or extension).
- [ ] No SPA changes required — `web/lib/filters/audit.ts:76-80` continues to use `action_type` and gets the same answer.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_list.rs` (or equivalent — verify path) (~20-40 LOC)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (~1 LOC doc fix)
- One Rust integration test (~50 LOC)

## Out of scope

- Adding new `kind` values to the audit row enum (covered by `fix-audit-feed-action-kind-variety` + `fix-audit-feed-missing-skill-invoked-and-workflow-step-executed-rows`)
- Seeding agent-execution audit rows so Scene 2 has rows to filter (covered by `fix-demo-seed-emit-worker-hired-audit-rows`)
- Page-side UI filter chips (works correctly today)

## Effort

S. ~50 LOC across gateway + test + runbook. ~1-2 hours. P2 because the runbook narrates the wrong URL spelling and an external API consumer will silently get "no rows" when the rows exist.
