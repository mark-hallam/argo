# Round 108 — PR #2577 audit-feed worker-name join (gateway substrate pre-merge)

- **Date / fire**: 2026-05-27T20:26Z (07:26 AEDT 2026-05-28)
- **Round**: 108
- **Pillar / claim**: Gateway substrate / Claim 6 (multi-tenant) regression-pin
- **PR head SHA**: `edbef58dc`
- **Origin/main SHA at run**: `e65328c5d`

## Verdict

🟢 **PRE-MERGE PASS** — substrate fix is real, tenant-bound, and shipped with a cross-tenant negative test. Recommend admin-merge via web-only/test-only lane is NOT applicable here (this is a Rust gateway change, not web/test/doc); requires standard pre-merge gate. Findings below are FYI for the conductor, not blocking.

## Setup

```bash
git fetch origin pull/2577/head:pr-2577-eval --quiet
git rev-parse pr-2577-eval  # → edbef58dcb…

git show pr-2577-eval -- crates/argo-cli/src/handlers/gateway/audit_query.rs
git show pr-2577-eval -- crates/argo-cli/tests/audit_feed_worker_name_join.rs
git show pr-2577-eval -- crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs
```

PR is OPEN against `claude-fix-gateway-audit-feed-serializer-worker-name-uuid-not-joined`; +426/-31 across 6 files (1 NEW handler module, 1 NEW integration test, 4 MODIFIED).

## Observed

### Production code path — tenant-scoped, deterministic fallback

The new module [crates/argo-cli/src/handlers/gateway/audit_query.rs](crates/argo-cli/src/handlers/gateway/audit_query.rs) exposes `resolve_worker_names(store, pairs)` which:

- Returns an empty map without touching the DB when `pairs.is_empty()` (correct short-circuit).
- Groups pairs `(tenant_id, worker_id)` into a per-tenant `HashMap<&str, HashSet<&str>>` (de-duplicates worker_ids within a tenant; one SQL per tenant rather than per row).
- Issues `SELECT … FROM worker w INNER JOIN persona p ON p.worker_id = w.id WHERE w.tenant_id = ? AND w.id IN (?,?,…)` with **all values bound through `sqlx::query`** — only the IN-list arity (`placeholders` string) is interpolated. The `audit-allow:` comment correctly documents this.
- Returns a `HashMap<(String, String), String>` keyed by `(tenant_id, worker_id)` — `lookup_worker_name` falls back to the worker UUID when the pair is absent and to literal `"unknown"` when `worker_id` is empty.

The keying choice is the load-bearing tenant guard: a same-id worker that lives ONLY in tenant-B cannot show up under tenant-A's lookup because the map insert happens only for rows the per-tenant query actually returned.

### Integration test — three real scenarios, real cross-tenant negative

[crates/argo-cli/tests/audit_feed_worker_name_join.rs](crates/argo-cli/tests/audit_feed_worker_name_join.rs):

1. Tenant-A seeds worker `WKR_MARIAH` named "Mariah" + audit row → expect `worker.name == "Mariah"`.
2. Tenant-B seeds worker `WKR_CROSS` named "OtherTenantMariah". Tenant-A queries an audit row stamped with `WKR_CROSS` → expect UUID fallback. **This is a real cross-tenant leak negative test** — if the resolver's tenant guard regressed, the assertion would fail with `"OtherTenantMariah"` leaking into tenant-A's response.
3. Tenant-A queries an audit row stamped with `WKR_GHOST` (never existed in either tenant) → expect UUID fallback (deletion path).

The test spawns a **real gateway subprocess** (`argo bin → gateway start --port … --max-requests 1`), seeds via the live `SessionStore`, hits `GET /api/v1/audit`, and asserts on the JSON response. No mocks at the gateway layer — this is the actual production HTTP path.

### routes_audit_feed.rs + routes_audit_detail.rs wire

Both handlers now call `resolve_page_worker_names(store, page)` and pass the resulting `WorkerNameMap` through to `audit_row_json`. The embedded payload `worker_name` is no longer consulted; the resolved-map lookup with UUID fallback is the only path. The PR body's note "the payload's `worker_name` is overwritten with the resolved value before `build_detail` runs" is consistent with the diff.

## Anti-theater check

- **Cross-tenant negative is structurally dispositive.** Asserting "Tenant-A audit row stamped with a worker_id that lives only in Tenant-B yields the UUID, not Tenant-B's persona name" cannot be made to pass by silently widening the resolver — the per-tenant `WHERE w.tenant_id = ?` query physically cannot return Tenant-B's worker row when bound to Tenant-A.
- **Live gateway spawn, not unit-test stub.** Using `argo_bin()` + `Command::spawn` + `await_listener_signal` proves the HTTP serializer path is what's being exercised. A mock-handler test could rewrite itself green; a subprocess gateway cannot.
- **SQL injection surface is bounded.** The `format!` interpolation is the IN-list arity only; all values go through `q.bind(value)`. The `audit-allow:` annotation is the project's standard documentation marker for this pattern.
- **Test-coverage trade-off disclosed in PR body.** The PR body honestly notes that the unit-test in `routes_audit_detail_tests.rs` had to flip one assertion from `"Triage Worker"` → UUID because the unit test never seeded a real worker row — and the end-to-end "resolved name wins" coverage moved to the new integration test. This is correct: the unit test was a theatrical pass under the old semantics (it never proved the join actually ran, it just trusted a payload field).

## Pre-existing flake noted by PR author

The PR body flags `handlers::gateway::routes_audit_feed::list_detail_alignment_tests::default_feed_legacy_id_fetches_detail` as failing on clean `origin/main` (commit `666614580`) WITHOUT these changes. PR author tracks it separately. Not introduced by this PR; not a blocker for merge.

## Gap / follow-up brief candidates

None blocking. P3 nice-to-have (NOT authored — below pilot threshold for evaluator authorship):

- The fallback path returns the worker UUID as the visible "name" when the join misses. For deleted workers that's better than nothing, but the camera will still see a 36-char UUID in `/audit`. A `"(deleted)"` placeholder or last-known persona name persisted at delete-time would close that residual UX gap. P3, ~50-80 LOC, defer.

## Conductor handoff

PR #2577 is OPEN. Standard pre-merge gate (full, not --skip-clippy) is appropriate since this is Rust substrate, not web-only. The evaluator has verified the cross-tenant tenant guard is real and the test exercises the live gateway HTTP path; the conductor can use this round report as the evaluator-pre-verification signal if it wishes to merge under the --skip-clippy authorization for evaluator-pre-verified PRs (operator-locked 2026-05-24).

123rd anti-theater verification this week.
