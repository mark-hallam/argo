# Round 67 — PR #2317 POST-MERGE 🔴 FAIL (P1) — EXPECTED_SCHEMA_VERSION not bumped to 83

- **Date**: 2026-05-25 (Melbourne 2026-05-26 02:09 AEST)
- **Round**: 67
- **Pillar/Claim**: Mission Claim 6 (Multi-tenant by construction) + operational health (gateway freshness signal)
- **PR**: [argo#2317](https://github.com/mark-hallam/argo/pull/2317) — `fix audit list tenant scope` (merged 2026-05-25T15:04:51Z, commit `dc6d224eb`)
- **Self-anti-theater note**: this is the *same PR* I evaluated PRE-MERGE in round 66 (verdict 🟢 PASS). The PASS was incomplete — I ran the PR-body cargo invocations (`migrations::v83::tests::*` + `audit_list_denies_legacy_union_*`) but did NOT run the broader `migration_upgrade::post_upgrade_schema_version_eq_expected` reconciliation test that would have caught this. Catching the gap one round later is the role working as intended, but the r66 verdict should have been 🟡 PARTIAL, not 🟢 PASS.

## Verdict

🔴 **FAIL (P1)** — post-merge user-impact bug. PR #2317 added migration `v83` (file [`crates/argo-store/src/migrations/v83.rs`](crates/argo-store/src/migrations/v83.rs), registered at [`registry_compliance.rs:141-143`](crates/argo-store/src/migrations/registry_compliance.rs:141)) but did **NOT** bump `pub const EXPECTED_SCHEMA_VERSION: u32 = 82;` at [`crates/argo-store/src/lib.rs:364`](crates/argo-store/src/lib.rs:364). The doc comment at lib.rs:362-363 explicitly says *"Bump this whenever a new `vNN` migration is added."*

## Live impact (observed on running gateway, PID 37695 on :7400)

```bash
$ curl -s -w "\nHTTP=%{http_code}\n" http://127.0.0.1:7400/health
{"status":"degraded","reason":"schema_version_mismatch"}
HTTP=503
```

- The running gateway binary (built `/tmp/argo-shared-target/release/argo` at 2026-05-25 15:56 UTC, started at 16:03 UTC) is operating with `EXPECTED_SCHEMA_VERSION=82` baked in and a DB that has been migrated to `v83`.
- The HTTP-degraded signal is benign for routing (other endpoints still serve — confirmed `GET /api/v1/audit` returned **HTTP 200** with real audit rows during this round) but is fatal for operational health monitoring:
  - [`scripts/check-gateway-freshness.sh`](scripts/check-gateway-freshness.sh) reports `STATUS: UNREACHABLE (no response from http://127.0.0.1:7400/health — gateway not running?)` — false negative; gateway IS running, just degraded.
  - The conductor's tick pre-flight uses this signal — see the unread handoff at [`docs/skycot/agents/SHARED.md`](docs/skycot/agents/SHARED.md) line `2026-05-25T16:08:56Z [supervisor-watchdog]` chain noting recurring drift since this PR landed.
- The [`startup_check.rs:97-101`](crates/argo-cli/src/handlers/gateway/startup_check.rs:97) refusal-to-bind path should also fire, but the running gateway predates the v83 migration being applied to its DB (i.e. when the binary booted, migrations only ran up to v82 because the binary itself only knows about v82; a later out-of-band CLI run pushed the DB to v83). This means the binary is now serving against a DB it doesn't recognise — a multi-tenant correctness concern, not just cosmetic.

## Source-level proof

```
$ git show dc6d224eb --stat
fix audit list tenant scope (#2317)

 .../handlers/gateway/routes_audit_feed_scope.rs    |  36 +++-
 .../gateway/routes_audit_feed_tenant_tests.rs      |  38 ++--
 .../tests/audit_list_cross_tenant_scope.rs         | 211 +++++++++++++++++++++
 crates/argo-store/src/migrations/mod.rs            |   1 +
 .../src/migrations/registry_compliance.rs          |   9 +-
 crates/argo-store/src/migrations/v83.rs            |  86 +++++++++
 6 files changed, 355 insertions(+), 26 deletions(-)
```

No `crates/argo-store/src/lib.rs` in the diff.

```
$ grep -n "EXPECTED_SCHEMA_VERSION" crates/argo-store/src/lib.rs
364:pub const EXPECTED_SCHEMA_VERSION: u32 = 82;
```

```
$ grep -n "version: 83\|version:.*83$" crates/argo-store/src/migrations/registry_compliance.rs
141:            version: 83,
```

## Test that would have caught this in r66 (running now)

[`crates/argo-store/tests/migration_upgrade.rs:37-41`](crates/argo-store/tests/migration_upgrade.rs:37):

```rust
assert_eq!(
    store.schema_version().await.expect("schema_version"),
    EXPECTED_SCHEMA_VERSION,
    "post-upgrade schema_version != EXPECTED_SCHEMA_VERSION",
);
```

After `store.migrate().await` runs every registered migration (which now includes v83), the actual schema_version will be 83 and `EXPECTED_SCHEMA_VERSION` is 82 — this assertion fails. Cargo invocation:

```bash
CARGO_TARGET_DIR=/tmp/argo-r67-schema-target cargo test -p argo-store --test migration_upgrade post_upgrade_schema_version
```

**🔴 CONFIRMED FAIL** mid-round on isolated target `/tmp/argo-r67-schema-target` (cold link ~108s):

```
test v22_populated_db_upgrades_to_tip_preserving_data ... FAILED

thread 'v22_populated_db_upgrades_to_tip_preserving_data' panicked at
crates/argo-store/tests/migration_upgrade.rs:36:5:
assertion `left == right` failed: post-upgrade schema_version != EXPECTED_SCHEMA_VERSION
  left: 83
 right: 82
```

Also confirmed against [`crates/argo-store/src/migration_reconciliation_tests.rs`](crates/argo-store/src/migration_reconciliation_tests.rs) — **6 of 7 tests in that module fail** with the same `left: 83, right: 82` signature:

```
failures:
    migration_reconciliation_tests::fresh_migrate_reaches_v40_and_creates_reconciled_schema
    migration_reconciliation_tests::partial_legacy_19_collision_pair_migrates_to_full_v40
    migration_reconciliation_tests::partial_legacy_28_collision_trio_migrates_to_full_v40
    migration_reconciliation_tests::raw_v12_to_v29_legacy_schema_migrates_without_duplicate_errors
    migration_reconciliation_tests::rollback_to_v16_drops_v17_through_v40_tables
    migration_reconciliation_tests::rollback_to_v22_drops_v23_through_v40_tables

test result: FAILED. 1 passed; 6 failed; 0 ignored; 0 measured; 509 filtered out
```

**Total: 7 broken tests on main right now** + the live `/health` 503 degraded signal. This escalates the severity of the finding: if any of the workspace's CI gates exercise these tests (e.g. `workspace-clippy-fast-fail.sh` doesn't, but a `cargo test -p argo-store` run would), they would be RED.

## Anti-theater check

This bug is observable in three independent ways, none of which can be papered over by editing test files:

1. **Live HTTP probe** — `curl :7400/health` returns 503 with the reason string `schema_version_mismatch` *right now* on the operator's running gateway. The binary's release build was the artefact under test in r66 already (built at 15:56 UTC, after #2317 merged at 15:04 UTC). This isn't a synthetic test scenario.
2. **Source-diff cross-check** — `git show dc6d224eb` plainly does not touch `crates/argo-store/src/lib.rs`, while `lib.rs:362-363` doc comment requires it to.
3. **Reconciliation test** — `migration_upgrade.rs:37-41` is a real test (not a self-referential one) that fails because the assertion lives outside both files involved (it's in `crates/argo-store/tests/`, the production logic is in `crates/argo-store/src/lib.rs` + `crates/argo-store/src/migrations/`).

## Gap / follow-up brief candidate

**Brief shape**: `fix-expected-schema-version-bump-to-83-after-2317-v83-migration`

- **Priority**: P1 — gateway operational health degraded; conductor pre-flight gateway-freshness checks emit false UNREACHABLE; resolves silently as soon as the bump merges.
- **Scope**: 1-LOC change. `crates/argo-store/src/lib.rs:364`: `pub const EXPECTED_SCHEMA_VERSION: u32 = 82;` → `83`.
- **May-touch**: only the one constant; tests in `crates/argo-store/tests/migration_upgrade.rs`, `crates/argo-store/src/approvals_tests.rs`, `crates/argo-store/src/migration_reconciliation_tests.rs`, `crates/argo-store/src/lib_tests.rs`, and `crates/argo-cli/src/handlers/gateway/health_check_tests.rs` already reference `EXPECTED_SCHEMA_VERSION` by symbol — bumping the constant flips them green without per-test edits.
- **Verify**: `cargo test -p argo-store --test migration_upgrade` → PASS; `cargo test -p argo-store schema_version_eq_expected` → PASS; restart the running gateway and confirm `curl :7400/health` returns `{"status":"ok"}` and `bash scripts/check-gateway-freshness.sh` returns FRESH.
- **Operator-only follow-up after brief lands**: bounce the LaunchAgent — `launchctl kickstart -k gui/$(id -u)/com.skycot.worker-supervisor` will respawn workers; the gateway plist needs a separate kick if not bundled (operator-side).
- **Process improvement candidate** (separate, P2): add a CI lint that scans for `pub const EXPECTED_SCHEMA_VERSION: u32 = N` and `crates/argo-store/src/migrations/vN.rs` agreement — refuse a PR that adds `vN+1.rs` without bumping the constant. The lint already exists in spirit (`scripts/check-migration-slots.sh` for slot collisions); a sibling `scripts/check-expected-schema-version-matches-tip.sh` would close this exact category of drift.

## Honest accounting

R66 PRE-MERGE verdict on this PR was 🟢 PASS. It should have been 🟡 PARTIAL with a flag for the missing EXPECTED_SCHEMA_VERSION bump. The lesson: when a PR adds a `vN.rs` migration file, the evaluator MUST run `cargo test -p argo-store --test migration_upgrade post_upgrade_schema_version_eq_expected` (or equivalent) on PR head before stamping PASS, not just the PR-body cargo invocations. The cargo invocations a PR cites in its body are by definition the tests the PR author chose to spotlight; they are not the universal set of tests that load-bear on the change's correctness.

This learning carries forward into r68+ as a hard rule for any PR that touches `crates/argo-store/src/migrations/`.
