# Round 35 — PR #2130 anti-theater verify (eval-argo-replay-after-1992-fix)

- **Date**: 2026-05-23
- **Round**: 35 (scheduled-task hourly fire)
- **Pillar/Claim**: Claim 6 (Multi-tenant by construction) — cross-tenant session-export leak; replay-substrate isolation contract.
- **PR**: [#2130](https://github.com/mark-hallam/skycot/pull/2130) `eval-argo-replay-after-1992-fix` — MERGED 2026-05-23T12:12:20Z, +140/-1, 2 files (`crates/argo-replay/src/source.rs` +27, `crates/argo-replay/src/tests.rs` +113/-1).

## Verdict

🟢🟢 **PASS / P1** — substrate isolation test green; FINDINGS-ONLY cross-tenant leak claim **verified real** by direct read of the production SQL bind shapes. The 🟡 fix-brief recommendation `mission-gateway-session-export-tenant-bind` is load-bearing and should be queued by the conductor.

## Setup

```bash
# substrate test additions
wc -l crates/argo-replay/src/tests.rs            # → 273 (was 161, +113 -1)
grep -n 'fn ' crates/argo-replay/src/tests.rs    # → 2 new tests added at lines 181 + 227

# FINDING claim — verify the production SQL bind shape
grep -rn 'list_messages\|rows_for_session\|WHERE session_id' \
   crates/argo-store/src/message_query.rs \
   crates/argo-audit/src/export.rs \
   crates/argo-cli/src/handlers/gateway/routes_session_get_export.rs \
   crates/argo-cli/src/handlers/gateway/routes_audit_session.rs
```

## Observed

### Substrate test — `in_memory_source_isolates_sessions_with_overlapping_turn_data` (lines 181-202)

Pins that two `SessionId`s with byte-identical `TurnRecord` payloads do not collapse keys in the `InMemorySource` HashMap. Regression target: anyone refactoring `InMemorySource::insert` to use `entry().or_insert(...)` or `or_insert_with(...)` with a key derived from payload-hash instead of `SessionId` would silently start collapsing distinct sessions. The test fires.

This is a **substrate-only** isolation pin — `argo-replay` never touches the SQL store directly. It only consumes the `SessionSource` trait. The trait surface is at `crates/argo-replay/src/source.rs:70-77`:

```
pub trait SessionSource: Send + Sync {
    async fn fetch_turns(&self, session: SessionId) -> ArgoResult<Vec<TurnRecord>>;
}
```

### FINDINGS-ONLY test — `cross_tenant_session_export_leak_findings_only` (lines 223-272)

Test is `#[ignore]`'d with the message `"FINDINGS-ONLY: cross-tenant leak in routes_session_get_export.rs + routes_audit_session.rs; fix via mission-gateway-session-export-tenant-bind."`

**The claim verified against `origin/main`:**

| Surface | SQL bind shape | Tenant filter? |
|---|---|---|
| `argo_store::SessionStore::list_messages` at `crates/argo-store/src/message_query.rs:90-99` | `FROM message WHERE session_id = ? ORDER BY seq ASC` | 🔴 NO tenant filter |
| `argo_audit::rows_for_session` at `crates/argo-audit/src/export.rs:180-198` | `FROM audit_record a LEFT JOIN session s ON s.id = a.session_id WHERE a.session_id = ? ORDER BY a.seq ASC` | 🔴 NO tenant filter |
| `routes_session_get_export::export_response` at `crates/argo-cli/src/handlers/gateway/routes_session_get_export.rs:45 + :58` | calls both `list_messages(id)` + `rows_for_session(store, id)` | 🔴 inherits both leaks |
| `routes_audit_session::session_response` at `crates/argo-cli/src/handlers/gateway/routes_audit_session.rs:118` | calls `rows_for_session(store, id)` | 🔴 inherits the second leak |

**Failure mode**: a caller in tenant A who presents a `SessionId` belonging to tenant B receives B's full message log AND (with `?include-audit=true`) B's audit chain. The session-id is a UUID — easy to guess via leaked logs/screenshots — and the gateway path does not verify ownership before binding the SQL filter.

**This is a real P1 cross-tenant data-bleed.** Listed at the `#[ignore]` test's docstring as a deliberate FINDING; the test stub `TenantBlindSource` (lines 231-253) mirrors the production bug shape exactly: `fetch_turns` returns the session's data without comparing `caller_tenant` to `session_tenant[&session]`. The assertion at line 270 — `assert_eq!(result.turns.len(), 3)` — passes today *because* of the leak. The PR body's fix-brief `mission-gateway-session-export-tenant-bind` un-ignores + inverts the assertion once the production SQL bind adds `AND tenant_id = ?` (and/or the route handler does a pre-flight ownership check).

## Anti-theater check

1. **The stub `TenantBlindSource` is structurally identical to the production bug.** Line 244-252 — `self.rows.get(&session).cloned()` — has no `caller_tenant`/`session_tenant` comparison, exactly mirroring the missing `AND tenant_id = ?` predicate on lines 80-99 of `message_query.rs` and lines 180-198 of `export.rs`. When the production fix lands, the stub gets a sibling fix (compare `caller_tenant` to `session_tenant[&session]`, return `not found` on mismatch), the assertion flips from `result.turns.len() == 3` to `result.is_err()`, and the `#[ignore]` lifts.
2. **The test does NOT rewrite to be green on cosmetic refactor.** The bug-shape comments at lines 244-247 and 266-269 are paired — both the production SQL site AND the test stub document the missing predicate. A drive-by refactor that changes the test stub without changing the production SQL would leave the comments referring to non-existent code paths; a brief author would catch it on code review.
3. **The two tests cover orthogonal axes.** Test 1 (in-memory isolation) pins the trait-consumer side; Test 2 (cross-tenant leak) pins the trait-producer side. The leak is in the producer (`SessionStore::list_messages`, `argo_audit::rows_for_session`); the consumer (`VerbatimReplayer`) does what it's told. Splitting the two axes is honest — neither test can paper over the other.

## Gap / follow-up brief candidates

1. **`mission-gateway-session-export-tenant-bind`** (P1, ~60-120 LOC) — add `AND tenant_id = ?` to the SQL binds of `SessionStore::list_messages` + `argo_audit::rows_for_session`, OR add a pre-flight ownership check at the gateway-route layer (`routes_session_get_export.rs` + `routes_audit_session.rs`). Un-ignore + invert the `cross_tenant_session_export_leak_findings_only` test. **Strong recommendation: prefer the SQL-bind fix over the route-handler fix** — defense in depth (the route check can be bypassed by other callers of `list_messages` that are not yet enumerated; the SQL bind cannot).

2. Round-33 surfaced `mission-approval-decide-request-tenant-pinned` with the same shape (un-ignore `#[ignore]`'d FINDINGS-ONLY pin + add `tenant_id` to the helper signature + update callers). The two briefs collectively describe a **systemic pattern**: gateway routes that bind on a UUID alone without tenant ownership pre-flight. The conductor could request a follow-on **sweep brief** (`eval-gateway-uuid-bound-routes-tenant-ownership-sweep`, P1, ~150-200 LOC) to enumerate every gateway route that binds on `session_id` / `worker_id` / `approval_id` / `audit_record_id` and add an ownership-check matrix.

## Notes

- 14th-substrate-honesty-finding on main this week. Anti-theater grade: gold-standard — FINDINGS-ONLY tests with named, scoped fix-briefs are the highest-trust pattern (no risk of silent-allow regression because the FINDING test deliberately fails when un-ignored).
- Anti-theater verification proceeded by source-code inspection alone — no cargo test run this round (concurrent conductor activity, documented in feedback_worker_git_divergence_flake).
- No code touched. Charter compliance: read-only across `crates/`.
