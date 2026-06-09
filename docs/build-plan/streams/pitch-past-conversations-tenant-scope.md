# pitch-past-conversations-tenant-scope

**Owner**: claude.
**Authored against**: origin/main `517955308` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-past-conversations-tenant-scope`.

## Why (VERIFIED 2026-06-09, read-only)
The Pitch / Coach "Past conversations" history panel reads the live-voice
sessions LIST endpoint (`GET /api/v1/voice/sessions`, SPA client
`web/lib/api/voice.ts:241` `listVoiceSessions`). The handler is
`voice_sessions_response` →`group_sessions` in
`crates/argo-cli/src/handlers/gateway/routes_voice_realtime.rs:238-345`.

Two verified gaps. The first is a correctness/robustness gap, not a leak; the
second is a missing regression pin on the isolation guarantee:

1. **The SQL read is tenant-blind; tenant isolation rests entirely on a single
   in-memory `continue`.** `voice_sessions_response` calls
   `tail_rows_filtered(store, SESSIONS_SCAN_CAP=500, kind="voice.transcript_turn")`
   (`routes_voice_realtime.rs:245-254`). `tail_rows_filtered`
   (`crates/argo-audit/src/export.rs:117-172`) builds its WHERE clause from
   only `kind` / `since_micros` / `session_id` — **there is NO tenant filter at
   the query level** (verified: export.rs:124-138 enumerates every clause).
   So the query returns the most-recent 500 `voice.transcript_turn` rows
   **across ALL tenants**, and the ONLY thing that stops tenant A seeing tenant
   B's rows is the per-row in-memory check at
   `routes_voice_realtime.rs:283`: `if p.get("tenant_id") != Some(tenant) { continue; }`.
   That check is correct (the write path stamps `payload.tenant_id` via
   `AuditSink::write` → `stamp_tenant_id`, `crates/argo-audit/src/sink.rs:207`),
   so today there is no leak — but the protection is one un-pinned line and the
   query is doing the wrong thing.

2. **Scan-cap starvation makes a tenant's own history silently vanish.**
   Because the 500-row window is filled across all tenants before the in-memory
   filter runs, a busy tenant B can push tenant A's rows out of the window —
   tenant A then sees an empty/partial "Past conversations" panel even though
   their rows exist (`SESSIONS_SCAN_CAP` comment at routes_voice_realtime.rs:230-233
   already flags this as the scale-later path). Pushing the tenant predicate
   into the SQL fixes both the robustness gap and removes the single-line
   dependency.

3. **No cross-tenant negative test exists.** `routes_voice_realtime_tests.rs`
   has `transcript_turns_persist_and_sessions_group_by_session_id` (single
   tenant `ws-default`, line 51) but no test proving tenant A's
   `GET /api/v1/voice/sessions` excludes tenant B's persisted turns. The
   isolation guarantee is therefore unprotected against a future refactor (e.g.
   someone "optimizing away" the line-283 filter once the SQL looks scoped).

NB: the related audit-roundtrip fix **#2737** (`d1ccee151`,
`fix-work-audit-rows-tenant-roundtrip`) aligned the *write-vs-read tenant
resolution* for the pitch-ask + genesis audit WRITERS so a row round-trips into
the caller's feed. It did NOT touch the voice-sessions LIST read path or add a
tenant predicate to `tail_rows_filtered`. So this is a distinct, still-open gap.

## Grep-before-write
- `rg -n 'fn tail_rows_filtered|TailFilters|where_clauses|tenant' crates/argo-audit/src/export.rs` — the tenant-blind LIST query + its filter struct. A `tenant_id` field is the additive change; grep first to confirm no `tenant` field already exists on `TailFilters` (it does not as of `517955308`).
- `rg -n 'pub struct TailFilters' crates/argo-audit/src/` — confirm the canonical home before extending it (do NOT declare a parallel filter type; extend `TailFilters`).
- `rg -n 'tail_rows_filtered\(' crates/ --type rust` — enumerate every caller before adding a field, so an additive `tenant: Option<&str>` (defaulting `None` = current behavior) doesn't break them. `TailFilters` already derives `Default`, so `..Default::default()` callers are unaffected.
- `rg -n 'group_sessions|voice_sessions_response|extract_tenant' crates/argo-cli/src/handlers/gateway/routes_voice_realtime.rs` — the LIST handler + its tenant source (`rate_limit::extract_tenant`).
- `rg -n 'fn stamp_tenant_id|payload_json.tenant_id' crates/argo-audit/src/sink*.rs` — confirms the column `audit_record.tenant_id` and `payload_json.tenant_id` are both written, so a SQL `a.tenant_id = ?` predicate is correct and matches the in-memory check.

## Fix (security / tenant-filter only — no business-logic churn)
1. Extend `TailFilters` (`crates/argo-audit/src/export.rs`) with an additive
   `tenant: Option<&str>` field (default `None` = today's all-tenant behavior;
   `..Default::default()` callers unchanged). When `Some`, push
   `a.tenant_id = ?` into `where_clauses` and bind it (the denorm column exists
   and equals `payload_json.tenant_id`).
2. In `voice_sessions_response` (`routes_voice_realtime.rs:245-254`), set
   `tenant: Some(&tenant)` on the `TailFilters` so the 500-row window is filled
   from the CALLER's tenant only. Keep the existing in-memory `tenant_id` check
   at line 283 as defence-in-depth (belt-and-braces; do not remove it).
3. Do NOT change the grouping, mode-filter, ordering, or response shape. No
   change to the write path. Status codes unchanged.

## Acceptance (truth-bounded)
1. **Cross-tenant NEGATIVE test** (new, in `routes_voice_realtime_tests.rs`):
   persist `voice.transcript_turn` rows for tenant A (`X-Argo-Tenant-Id: ws-a`)
   AND tenant B (`ws-b`) into one shared in-memory store; then
   `GET /api/v1/voice/sessions` as tenant A returns ONLY A's sessions and the
   body does NOT contain any of B's `session_id`s or text — and the symmetric
   call as B excludes A. Assert the negative explicitly (B's session id is
   `!body.contains(...)`).
2. **Starvation regression**: persist > `SESSIONS_SCAN_CAP` (use a small test
   override or assert via the SQL-level predicate) rows for tenant B followed by
   one row for tenant A; tenant A's LIST still returns A's one session (proves
   the tenant predicate is in the query, not only the post-filter). If overriding
   the cap is impractical in-test, assert at the `tail_rows_filtered` unit level
   that `tenant: Some("ws-a")` returns zero `ws-b` rows.
3. **Existing test still green**:
   `transcript_turns_persist_and_sessions_group_by_session_id` unchanged.
4. `cargo test -p argo-cli -p argo-audit` green; crate-scoped clippy clean on
   touched files. Keep both touched files < 500 LOC (export.rs and
   routes_voice_realtime.rs are well under; the test file too).

## On ship (same commit)
Update the `FEATURE_MATRIX.md` row for the voice/pitch history surface (tenant-
scoped LIST), regenerate `docs/skycot/feature-matrix.html` via
`python3 scripts/render-feature-matrix.py` in the same commit. Note it closes
the conductor-flagged pitch-past-conversations tenant-scope gap (distinct from
#2737).

## Effort
S (additive `Option` field on one filter struct + one call-site + a cross-tenant
negative test; no business-logic change, no migration).
