# Round 19 — anti-theater verify of three live items

**Date**: 2026-05-23T16:25Z (round 19; concurrent session holds round 18 for LIVE-criteria drafts of claims 4/5/6 — coexisting)
**Pillar**: process integrity + mission claim 1 + claim 6 (cross-tenant)

## Scenarios (3)

| # | Scenario | Verdict | Severity |
|---|---|---|---|
| A | Round-17 Claim 1 LIVE regression test — anti-theater pass | 🟢 PASS | — |
| B | #2100 mission-orchestrator-duty-store-tenant-args — verify before merge | 🟢 PASS | — (round-16 P2 closed) |
| C | #2097 token-cache escalation premise — read the source, confirm leak shape | 🟢 PREMISE VERIFIED REAL | P0 confirmed |

Plus operational sweep: gateway DOWN, 5/5 FINDINGS-ONLY fix-by deadlines within budget, no unread cross-agent handoffs, 7 open PRs (3 stale CHANGES-REQUESTED cluster from tick-85 still rotting).

---

## Scenario A — Claim 1 LIVE regression test verified PASSES cold

**File**: `crates/argo-identity/tests/claim_1_live_workers_persist.rs` (commit `345587c20`)
**Brief**: round-17 self-claimed "PASSES on main" — this is the anti-theater follow-up.

### Setup

```
cargo test -p argo-identity --test claim_1_live_workers_persist
```

### Observed

```
running 1 test
test claim_1_live_workers_persist_across_simulated_restart ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; finished in 0.15s
```

Cold compile + run on fresh `origin/main`. 6.35s compile, 0.15s test execution.

### Anti-theater check

Read the test source end-to-end:

1. **No state-leak vector** — `tempfile::tempdir()` is created fresh each invocation; the `.db` file lives there. If the test was state-dependent (passing only because of leftover data on the file system), the new tempdir blocks that.
2. **Pre-flight forbidden-TTL check is structural** — line 107-119 reads `pragma_table_info('worker')` and asserts none of `expires_at`, `ttl`, `valid_until`, `expiry` exist as columns. A future migration adding any of those breaks this loud — claim 1 LIVE assumes workers persist indefinitely, not until a deadline.
3. **5-hire phase exercises real persistence** — file-backed SQLite (not `:memory:`), drop both `IdentityStore` and `SessionStore`, re-open the SAME file via `SessionStore::open_unencrypted_for_tests(&db_path)`. If `IdentityStore::create_worker_from_draft` were holding state in-process (cache, lazy hashmap, etc.), the second-open `list_workers` would return zero or wrong count and the test breaks.
4. **Mariah-id stability check** — line 219-227 asserts `mariah_id_before_restart == mariah_id_after_restart`. Catches the failure mode where identity is regenerated on every resolve (would still produce 5 workers, but the IDs would not match).
5. **Second restart cycle** — phase 3 (lines 235-263) re-opens the same file AGAIN, lists, finds Mariah by name. Pins the "24h later" half of the operator-locked LIVE criteria (no actual clock advance, but the migration-idempotency + same-id-still-resolvable invariants are what 24h-later would need).

**Honest deferrals** documented in the test's own docstring (lines 27-35):
- Memory roundtrip ("write marker as Mariah, restart, read marker back") deferred to `eval-claim-1-live-memory-roundtrip-extension`
- "Gateway restart" is a store-handle drop + re-open, not actual subprocess re-spawn

### Gap / follow-up

None for this scenario. Follow-up brief `eval-claim-1-live-memory-roundtrip-extension` is the right next step (~150 LOC: extend the existing test with argo-memory write/read across restart phase). Worth a P2 conductor consideration in a quiet tick.

---

## Scenario B — #2100 mission-orchestrator-duty-store-tenant-args verified

**PR**: [#2100](https://github.com/argo-platform/argo/pull/2100) (`claude-mission-orchestrator-duty-store-tenant-args` branch)
**Source of finding**: round-16 P2 recommendation. DutyStore mutators `get`/`mark_fired`/`set_paused`/`link_fire`/`sessions_for_duty` took `id: &str` only; production seam held via `DutyScheduler::tick` pre-filter but surface was not self-defending.

### Setup

```
git fetch origin pull/2100/head:pr-2100 && git checkout pr-2100
cargo test -p argo-orchestrator --test cross_tenant_duty_store_test
```

### Observed

```
running 5 tests
test duty_get_cross_tenant_returns_none_identical_to_nonexistent_id ... ok
test duty_sessions_for_duty_cross_tenant_returns_empty_identical_to_nonexistent_id ... ok
test duty_set_paused_cross_tenant_is_noop_identical_to_nonexistent_id ... ok
test duty_mark_fired_cross_tenant_is_noop_identical_to_nonexistent_id ... ok
test duty_link_fire_cross_tenant_is_noop_identical_to_nonexistent_id ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; finished in 0.59s
```

Production surface inspection at `crates/argo-orchestrator/src/duty_scheduler_store.rs:75-232`:

- `get` (line 78): `WHERE id = ? AND tenant_id = ?` — binds tenant via `tenant_id.0.as_bytes().as_slice()`
- `list_for_tenant` (line 94): `WHERE tenant_id = ?` (unchanged, already tenant-scoped)
- `set_paused` (line 131): `UPDATE role_duty SET paused_until = ? WHERE id = ? AND tenant_id = ?`
- `mark_fired` (line 152): `UPDATE role_duty SET last_fired_at = ? WHERE id = ? AND tenant_id = ?`
- `link_fire` (line 174): `INSERT OR IGNORE INTO duty_fired_session ... SELECT ?, ?, ?, ? FROM role_duty WHERE id = ? AND tenant_id = ?` — the SELECT-from-role_duty pattern is correct because `duty_fired_session` table has no own tenant column; the JOIN-on-write enforces cross-tenant refusal.
- `sessions_for_duty` (line 205): `INNER JOIN role_duty rd ON rd.id = dfs.duty_id WHERE dfs.duty_id = ? AND rd.tenant_id = ?` — JOIN-on-read pattern, same intent.

### Anti-theater check

Two-tenant fixture (`two_duty_tenants`, lines 36-77 of test file) seeds a real `link_fire` attribution row in tenant_a. Without this seed, cross-tenant and nonexistent branches of `sessions_for_duty` would BOTH return `Vec[]` regardless of whether scoping was correct or broken — the test would falsely pass. Seed is load-bearing.

Each test uses `format!("{cross:?}")` vs `format!("{nonexistent:?}")` for byte-identical `Debug` repr — same pattern as #2068 existence-leak fix. This is stronger than "returns Ok(None)"; it pins that the cross-tenant call cannot leak the "id exists in some other tenant" bit through error variants, timing, or structural differences.

Sanity assertion at line 94 confirms `tenant_a.get(duty_a.id)` still returns the duty — proves the AND-tenant filter didn't over-correct into a "never returns anything" bug.

### Verdict

🟢 Ready to merge. Closes round-16 P2 finding. The DutyStore surface is now self-defending — a future caller cannot accidentally pass an `id` without a tenant.

### PR's own sensible deviations

PR body documents 4 deviations from the brief, all reasonable:
1. CLI callsites (5 sites in `argo-cli/src/handlers/`) updated alongside the brief's named "single production caller"; without these, workspace doesn't compile. Standard SWARM_PROTOCOL "effective worker habit #6".
2. `tenant_id: TenantId` (by value, not `&TenantId`) — matches every other tenant-scoped surface in the crate.
3. 5 new tests in a sibling file (`cross_tenant_duty_store_test.rs`, 227 LOC), not appended to `cross_tenant_handoff_test.rs` (would push parent over 500-LOC ceiling).
4. `DutyScheduler::fire_now` also gains `tenant: TenantId` (internally calls `get`/`link_fire`/`mark_fired` — keeping it tenant-blind would defeat the brief's intent).

All four are correctly justified.

---

## Scenario C — #2097 token-cache escalation premise VERIFIED REAL

**Escalation**: tick-87 reported `mission-mcp-connector-token-cache-tenant-keyed` as P0 fix for 3 cross-tenant credential leaks surfaced by #2097 (gcal/gmail/salesforce). Worker just claimed the brief (commit `3e1611d2f`). Verifying the premise before LOC burns.

### Setup

Read the three connector source files for OAuth-token-cache shape:

```
crates/argo-mcp-connector-gmail/src/auth.rs
crates/argo-mcp-connector-salesforce/src/auth.rs
crates/argo-mcp-connector-gcal/src/client.rs
```

### Observed — all 3 leaks REAL

**Gmail** (`auth.rs:113-116`):
```rust
fn cache_path() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    Ok(CACHE_PATH.iter().fold(cwd, |p, part| p.join(part)))
}
```
`CACHE_PATH` is `[".argo", "connectors", "gmail", "access_token.json"]` (line 11). Process-cwd-shared, no tenant key. In a single multi-tenant gateway process, every tenant's Gmail tool invocation reads + writes the same file.

**Salesforce** (`auth.rs:47-49` + `140-141`):
```rust
pub async fn access_token_from_env() -> Result<SalesforceToken, String> {
    let cfg = SalesforceConfig::from_env()?;
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    if let Some(token) = read_cached_token(&cwd)? { ... }
    ...
}

fn cache_path(cwd: &Path) -> PathBuf {
    cwd.join(".argo/connectors/salesforce/token.json")
}
```
Same shape — `cwd` is sourced via `std::env::current_dir()`, no tenant key.

**Gcal** (`client.rs:339-348`):
```rust
fn token_cache_path() -> McpResult<PathBuf> {
    std::env::current_dir()
        .map(|cwd| {
            cwd.join(".argo")
                .join("connectors")
                .join("gcal")
                .join("access_token.json")
        })
        .map_err(|e| McpError::Io(format!("current_dir: {e}")))
}
```
Same shape.

### Failure mode

In a multi-tenant gateway running with one `cwd`:
- Tenant_a invokes Gmail tool → writes its OAuth access token to `<cwd>/.argo/connectors/gmail/access_token.json`
- Tenant_b invokes Gmail tool → reads same file → **uses tenant_a's access token**, gains access to tenant_a's Gmail account
- Tenant_b's refresh flow then OVERWRITES tenant_a's cached token with its own — last-writer-wins across tenants, accounts get mixed

This is structurally the same class as round-9's `argo-credentials` cross-tenant finding (#2080 §5), but at the per-connector layer the global-credentials fix doesn't cover.

### Anti-theater check

The brief proposes:
```rust
fn cache_path(tenant_id: &TenantId) -> PathBuf {
    argo_config::workspace_root()
        .join(".argo/connectors").join(tenant_id.as_str())
        .join("gmail").join("access_token.json")
}
```

This is the right shape — tenant_id segment in the path before the connector segment, sourced from `argo_config::workspace_root()` (not cwd, which is a separate fragility — cwd can change at runtime). Migration on first read (`legacy/` subdir + `tracing::warn!`) is load-bearing — without it, operators upgrading lose OAuth state until re-authentication, but at least don't have unauthorized cross-tenant access during the transition.

### Verdict

🟢 Brief premise VERIFIED. Worker proceeding with the right shape. P0 escalation justified — this is a live credential leak that any multi-tenant deployment would hit on first cross-tenant tool invocation.

---

## Operational sweep

- **Gateway**: `curl -sf http://127.0.0.1:7400/health` → UNREACHABLE. Browser scenarios skipped this round (Chrome MCP unavailable). No regression — gateway has been down across the last several scheduled rounds.
- **Unread handoffs**: `scripts/check-unread-handoffs.sh` → ✅ NONE (cutoff 16:10Z, conductor tick-87 actioned everything since round 17).
- **FINDINGS-ONLY fix-by deadlines**: 5 within budget (closest = `mission-cross-tenant-test-audit-export` at 2 days remaining); 0 overdue.
- **Open PR cohort** (7):
  - #2087 `mission-cred-get-tenant-parameter` — STILL OPEN at ~25h+ (per auditor T03-39 note); rebase-cluster brief authored tick-86 but didn't fix; second-round fix-request posted tick-87. P0 risk.
  - #2088 `fix-duty-scheduler-invokes-agent-runner` — MERGEABLE per `gh pr list` but still CHANGES-REQUESTED.
  - #2090 `mission-credentials-per-tenant-master-key` — UNKNOWN mergeable, CHANGES-REQUESTED.
  - #2092 `mission-skill-table-add-tenant-id-column` — UNKNOWN, was CHANGES-REQUESTED tick-86.
  - #2098 `web-workflow-runs-table-pagination-filters`
  - #2100 `mission-orchestrator-duty-store-tenant-args` (this round's Scenario B — clean to merge)
  - #2101 `mission-skill-ab-test-run-table-add-tenant-id` (MERGEABLE, fresh)
- **Cargo target staleness**: noted PR-2100 verification took 20s to compile delta — shared `/tmp/argo-shared-target` cache hot enough that the round didn't burn excessive wall time.

## Coexistence with concurrent round 18

A second evaluator session is in flight focused on **drafting LIVE criteria for claims 4/5/6** (file `draft-live-criteria-claims-4-5-6.md` + their heartbeat labeled round 18 with a different `run_notes`). That work is complementary to mine — mine is verification, theirs is criteria-spec authoring. I am NOT overwriting their heartbeat or their file; this round numbers as 19 and appends additively to LATEST.md + SHARED.md.

## Brief candidates (none new from this round)

All three scenarios verdict 🟢. No new briefs proposed. Existing items still in flight cover any follow-ups:
- `eval-claim-1-live-memory-roundtrip-extension` (P2, ~150 LOC) — exists as a docstring reference in the round-17 test; conductor can promote to ledger row when queue starves.
- `claude-rebase-changes-requested-cluster-tick85` (P0) — was authored tick-86 but worker no-op'd; tick-87 posted second fix-request. May need a manual re-author or operator escalation if not actioned in next 2 ticks.

## Round 19 contribution

3 verifications shipped (claim 1 LIVE test ✅, #2100 ✅, #2097 escalation premise ✅). No theater. No new briefs. Operational state captured for tick-88 conductor.
