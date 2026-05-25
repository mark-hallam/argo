# Round 68 — PR #2317 EXPECTED_SCHEMA_VERSION still not bumped (P0 escalation of r67 P1)

**Date**: 2026-05-25 (Melbourne 2026-05-26 03:15 AEST)
**Round**: 68
**Pillar/Claim**: Substrate / Mission Claim 6 (Multi-tenant by construction — but also a v1-demo blocker)
**Verdict**: 🔴 **FAIL · P0** (escalated from r67 P1 — same finding, +1h elapsed without remediation)

---

## Setup

Round 67 (2026-05-25T16:09Z, +1h08m ago) reported that PR #2317 `fix(gateway): deny legacy audit rows by default` (commit `dc6d224eb`, merged 2026-05-25T15:04:51Z) added migration `v83.rs` but did NOT bump `EXPECTED_SCHEMA_VERSION` from 82→83 at [`crates/argo-store/src/lib.rs:364`](crates/argo-store/src/lib.rs:364). Live impact: `/health` returns HTTP 503 `schema_version_mismatch` on the operator's running gateway, plus 7 broken tests on `origin/main`.

Recommendation in r67: author brief `fix-expected-schema-version-bump-to-83-after-2317-v83-migration` (α-r67, 1-LOC change, P1) + companion CI-lint `ops-add-expected-schema-version-vs-tip-migration-ci-lint` (β-r67, P3).

This round, I re-checked state to see what action the conductor + workers took in the ~1h gap.

Commands run:

```bash
grep -n EXPECTED_SCHEMA_VERSION crates/argo-store/src/lib.rs
git log -5 --oneline -- crates/argo-store/src/lib.rs
git log -3 --oneline -- crates/argo-store/src/migrations/v83.rs
ls crates/argo-store/src/migrations/ | grep '^v8[0-9]'
curl -s -w 'HTTP %{http_code}\n' http://127.0.0.1:7400/health
curl -sf -w '\nHTTP %{http_code}\n' 'http://127.0.0.1:7400/api/v1/audit?limit=1'
bash scripts/check-gateway-freshness.sh
grep -n 'expected-schema-version\|EXPECTED_SCHEMA_VERSION\|2317\|fix-expected' docs/build-plan/BUILD_STATUS.md
ls docs/build-plan/streams/ | grep -i 'expected-schema\|schema-version\|v83\|2317'
```

## Observed

**1. Source code: unchanged.**

```
crates/argo-store/src/lib.rs:364:pub const EXPECTED_SCHEMA_VERSION: u32 = 82;
```

`git log -5 -- crates/argo-store/src/lib.rs` head is `12401dd0d mission-claim-2 — KPI rollup, take 2 (supersedes #2279) (#2290)` — predates r67. No edit since r67's finding.

`git log -3 -- crates/argo-store/src/migrations/v83.rs` head is `dc6d224eb fix audit list tenant scope (#2317)` — the original offending PR, unchanged.

Migration files present: `v80.rs`, `v81.rs`, `v82.rs`, `v83.rs`. v83 is the registered tip; expected-version constant lags by 1.

**2. Live gateway: still degraded.**

```
$ curl -s -w 'HTTP %{http_code}\n' http://127.0.0.1:7400/health
HTTP 503
{"status":"degraded","reason":"schema_version_mismatch"}
```

Other endpoints partially serve — `GET /api/v1/audit?limit=1` returns HTTP 200 with one real audit row (`019e5dbaec03…7f3f41e`, `hipaa_compliance_check`, succeeded, timestamp 2026-05-25T06:03:11Z). The mismatch is non-fatal to existing routes but the `/health` probe is broken, which means external observability (load balancers, ops dashboards, cron pingers) believes the service is down. `scripts/check-gateway-freshness.sh` reports `STATUS: UNREACHABLE  (no response from http://127.0.0.1:7400/health — gateway not running?)` — a false negative: the gateway IS running, the health probe is returning 503 not 200, and the freshness script's curl flags (`-sf`) cause it to silently treat any non-2xx as unreachable.

**3. BUILD_STATUS.md: no follow-up brief authored.**

`grep -n expected-schema-version docs/build-plan/BUILD_STATUS.md` returns no rows. `ls docs/build-plan/streams/ | grep schema-version` returns only the pre-existing `mission-store-schema-version-fixture-refresh.md` (a separate, older brief, not a fix for this).

The conductor's T16-13 + T19-08 ticks ran in the ~1h gap but neither flipped a ledger row to author this brief. Two conductor sessions saw r67's SHARED.md handoff and did not action it.

**4. Test state on main: 7 broken tests (re-confirmed from r67).**

I am not re-running cargo this round to conserve token budget — r67 proved the broken state with cold cargo against isolated `/tmp/argo-r67-schema-target`:

- `cargo test -p argo-store --test migration_upgrade v22_populated_db_upgrades_to_tip` → 1/1 FAILED, literal `assertion left == right failed: post-upgrade schema_version != EXPECTED_SCHEMA_VERSION · left: 83 · right: 82`
- `cargo test -p argo-store --lib migration_reconciliation` → 6/7 FAILED with same signature (`fresh_migrate_reaches_v40`, `partial_legacy_19`, `partial_legacy_28`, `raw_v12_to_v29`, `rollback_to_v16`, `rollback_to_v22`)

The production code path that drives these failures is intact and unchanged on main, so the tests' broken state is mechanically guaranteed by inspection: every test asserting `assert_eq!(store.schema_version().await.unwrap(), EXPECTED_SCHEMA_VERSION)` against a freshly-migrated DB will see actual `83` vs expected `82`.

## Anti-theater check

This is not a flaky-test artifact. The failure is mechanically derivable:

1. The migration runner (`crates/argo-store/src/migrations/registry.rs`, `pub(crate) async fn run`) iterates `all_migrations()` and writes each version into `schema_version` table after applying. v83 is in `all_migrations()` so a fresh migrate writes `83`.
2. `EXPECTED_SCHEMA_VERSION` is a literal `u32 = 82` in source at lib.rs:364.
3. Six tests assert equality between actual and expected. Two of those tests are in [`crates/argo-store/src/migration_reconciliation_tests.rs`](crates/argo-store/src/migration_reconciliation_tests.rs:15) lines 15, 122, 179, 228, 253, 330; one is at [`crates/argo-store/src/lib_tests.rs:11`](crates/argo-store/src/lib_tests.rs:11); the upgrade test is at [`crates/argo-store/tests/migration_upgrade.rs:38`](crates/argo-store/tests/migration_upgrade.rs:38) + line 77.
4. `crates/argo-store/src/store_open.rs:279` runs `actual.cmp(&EXPECTED_SCHEMA_VERSION)` at startup health-check and returns `degraded` if they mismatch — which is exactly the 503 the operator's gateway is now serving.

No test was rewritten to be green. The tests are correct; the source constant is out of sync with the registry tip. The fix is a 1-character source edit (`82` → `83`).

## Severity escalation rationale (P1 → P0)

Three reasons:

1. **It's been ~1h08m since r67 flagged this and no action taken.** Two conductor ticks (T16-13, T19-08) ran in the gap and merged 4 web PRs + acknowledged the unread-handoffs sweep without picking this up. The brief has not been authored. r67's recommendation was treated as not-urgent. Operator's overnight Genesis+Track P merge-aggressive window is now ~12h in and this is the only known substrate-side blocker.
2. **The /health 503 is visible to anyone the operator points at the live gateway.** Investor-facing demo viability depends on the canonical gateway port (7400) responding green. Operator's `Remaining for v1.0 launch` item (b) was closed in r67-era saying `/health = 200, schema v81 FRESH`; THAT IS NO LONGER TRUE on main. The status block in CLAUDE.md is stale.
3. **7 broken tests on main means the next `cargo test -p argo-store` run by any worker, eval, or CI invocation will RED.** This is a process-loop hazard — workers picking up unrelated argo-store briefs will see RED and may incorrectly attribute it to their own work, wasting cycles.

## Gap / follow-up brief candidates

**α-r68 (was α-r67, ESCALATED P0)** — `fix-expected-schema-version-bump-to-83-after-2317-v83-migration`

- Scope: 1 LOC change to `crates/argo-store/src/lib.rs:364`. Bump `82` → `83`.
- Validation: `cargo test -p argo-store --test migration_upgrade` + `cargo test -p argo-store --lib migration_reconciliation` both go green. Restart gateway, confirm `/health` returns 200.
- Authorization: this brief qualifies for overnight `--skip-clippy` window IF authored as `claude-fix-expected-schema-version-bump-to-83` — but the operator's overnight cluster grant per memory `feedback_overnight_genesis_track_p_merge_authorization_2026_05_25.md` explicitly covers `mission-genesis-*`, `pitch-*`, and `fix-pr-22*-rebase*`; not generic `claude-fix-*`. **Conductor should treat this as P0 carve-out under "operator-trust-grant" spirit since the v1.0 demo gate is gated on the live gateway responding green, even if the literal brief name doesn't pattern-match.** When in doubt, conductor asks operator inline at next interactive opportunity; do not silently merge outside the literal authorization category.
- Why not just bump it in this evaluator session: charter rule. Evaluator can NEVER author briefs or commit production code. This is for conductor + workers.

**β-r68 (was β-r67, P3 carry)** — `ops-add-expected-schema-version-vs-tip-migration-ci-lint`

- Scope: ~30-LOC bash script `scripts/check-expected-schema-version.sh` — derive tip from `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1`, compare to `grep -oP 'EXPECTED_SCHEMA_VERSION:\s*u32\s*=\s*\K\d+' crates/argo-store/src/lib.rs`, fail on mismatch.
- Sibling to existing `scripts/check-migration-slots.sh`. Add to CI gate after α-r68 lands.
- Priority: P3 (preventive, post-fix).

**γ-r68 (NEW)** — `fix-check-gateway-freshness-sh-distinguish-503-from-unreachable`

- Scope: ~5-LOC edit to `scripts/check-gateway-freshness.sh`. Drop `-sf` from the curl invocation, capture HTTP code separately, distinguish HTTP 503 (degraded) from connection-refused (truly unreachable). The current false-negative is hiding real degraded-gateway state from conductor ticks.
- Priority: P2 (ops-loop reliability).

---

**Cost note**: this scenario consumed ~6K tokens. Most of the verification was already paid for in r67; this round was a re-confirm + escalation pass. Anti-theater check is mechanical (source-derivable) — no cargo run needed this round.
