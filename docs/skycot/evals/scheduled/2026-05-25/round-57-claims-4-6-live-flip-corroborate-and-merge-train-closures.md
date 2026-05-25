# Round 57 — Claims 4+6 LIVE flip corroboration + T09-50 merge-train closure audit

- **Date / Round**: 2026-05-25T09:55Z fire → reports written 10:05Z (Melbourne 20:05 AEST) · Round 57
- **Trigger**: operator "fire tick" invocation ~14min after r56 ship
- **Pillars / Claims**: Mission claim 4 (Skills shared + learned) + claim 6 (Multi-tenant by construction); plus process-drift sweep
- **Verdict**: 🟢🟢🟢 **PASS × 3** — corroborate + closure-audit + drift sweep

## TL;DR

- ✅ **#2285 CLAUDE.md claims-4+6 LIVE flip CORROBORATED** by independent cold-cargo rerun on **current** `origin/main` HEAD `1a55a64c6` (PR ran on stale SHA `13f4ae22e`; ~13 substrate PRs shipped between).
  - `claim_4_live_skill_install_invoke_and_audit_with_provenance`: 1/1 PASS, 2m 54s cold build + 0.18s test
  - `claim_6_live_cross_tenant_select_no_existence_leak` + `claim_6_live_cross_tenant_blanket_export_isolation`: 2/2 PASS, 2m 16s cold build + 0.24s test
- ✅ **#2271 legacy-worker test fix anti-theater HONEST** — properly chosen "option (b) stale test asserting old behavior"; closes r50/r56 carry (g). Production code untouched (cross-tenant filter IS the desired post-v79 behavior). Test now uses `install_for_tenant` so rows carry the test's tenant_id and surface through the strict lookup.
- ✅ **#2270 skill vertical tenant isolation symmetry** — closes r52 carry (l). PR body has a gold-standard anti-theater proof: temporarily removed `AND tenant_id = ?` from production code, reran the test, watched it fail with the exact cross-tenant-leak shape, then restored before commit.
- ✅ **#2273 pre-merge-gate non-skippable test-compile gate** — closes the 6-round-carry (a r41-r53 process gap). Replaces the "clippy was the only cross-crate check" hole with a `cargo check --workspace --tests` step before clippy, plus an explicit allowlist for known-red carries.
- ⚠️ **Process drift carry from r56**: SHARED.md lines 342-349 STILL contain stale merge-conflict markers (`<<<<<<< Updated upstream` / `=======` / `>>>>>>> Stashed changes`) from the T09-10 conductor stash-recovery. The T09-50 conductor tick (ran 09:50Z, ~5min before my r56 commit landed 09:55Z) did NOT clean them. **2nd-consecutive-round escalation.**

41st anti-theater-honest substrate verification this week.

## Scenario 1 — #2285 claims-4+6 LIVE flip cold-corroboration

### Setup

```bash
git worktree add --detach /tmp/argo-eval-r57 1a55a64c6
cd /tmp/argo-eval-r57

# Parallel cold builds (sccache disabled; isolated target dirs)
CARGO_TARGET_DIR=/tmp/cold-r57-claim4-target cargo test \
  -p argo-skill --test claim_4_live_install_invoke claim_4_live -- --nocapture
CARGO_TARGET_DIR=/tmp/cold-r57-claim6-target cargo test \
  -p argo-audit --test claim_6_cross_tenant_approval_select_no_existence_leak \
  claim_6_live_cross_tenant -- --nocapture
```

PR #2285 ran on head SHA `13f4ae22e22b3898fd84726d9c9eda582eb789b6`. Between that head and current `origin/main` HEAD `1a55a64c6`, the T09-50 conductor tick merged ~13 PRs: #2270 #2271 #2273 #2275 #2278 #2281 #2280 #2267 #2277 #2286 #2285 #2283 #2282 #2288. None of those modify the test files or the production paths under test (`argo_skill::SkillStore::install` / `attach_to_session`, `argo_audit::AuditSink::new_with_tenant` / `write_with_tenant` / `verify_session`, `argo_store::skill_provenance::query_for_skill`), so the cold corroboration is genuine new HEAD coverage.

### Observed

**Claim 4** (`claim_4_live_skill_install_invoke_and_audit_with_provenance`):

```text
Finished `test` profile [optimized + debuginfo] target(s) in 2m 54s
Running tests/claim_4_live_install_invoke.rs

running 1 test
test claim_4_live_skill_install_invoke_and_audit_with_provenance ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
```

**Claim 6** (cross-tenant approval select + blanket export):

```text
Finished `test` profile [optimized + debuginfo] target(s) in 2m 16s
Running tests/claim_6_cross_tenant_approval_select_no_existence_leak.rs

running 2 tests
test claim_6_live_cross_tenant_select_no_existence_leak ... ok
test claim_6_live_cross_tenant_blanket_export_isolation ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
```

### Anti-theater check — test sources read honest

**Claim 4 test** (`crates/argo-skill/tests/claim_4_live_install_invoke.rs:135-443`) — 8 distinct phases:

1. **Schema preflight**: asserts `skill_provenance` table exposes all 6 columns (`skill_name` / `skill_version` / `provenance_kind` / `actor` / `parent_id` / `ts`). A migration that renames any column fails loudly.
2. **Workspace + session seeding**: real `SessionStore::ensure_workspace` + `create_session`.
3. **Install**: real `SkillStore::install` with marketplace-shaped SKILL.md; asserts name, version, source identity, body snippet.
4. **Provenance record**: real `query_for_skill`; asserts `skill_name`, `skill_version`, `kind == Imported`, `parent_id` empty (marketplace install has no `learned_in` session). Then re-reads via `find_active_by_name` and asserts the `skill.source` field round-trips the full `published:verified:acme-skills` identity.
5. **Worker can list**: real `LayeredLookup::list_for_action("low")` + `("high")`; asserts skill surfaces at both risk classes and lands in `SkillLayer::Workspace`.
6. **Attach to session**: real `attach_to_session` SQL + reads back from `session_skill` table.
7. **Audit row write+read**: simulates the production `WorkflowStepCompleted` payload (the test docstring at L76-82 honestly discloses this: "this test pins the substrate one layer down so it can run in any CI sandbox without `claude` / `anthropic` keys"; the companion `vertical_workflow_skill_invoke_smoke.rs` drives the LLM-end-to-end half).
8. **Forensic join**: takes the `skill_name` from the persisted audit row, joins back to `skill_provenance` and `skill` row, asserts the same `(skill_name, skill_version, kind, source)` triple is recovered. This is the operator-facing "click the audit row → see who published the skill" promise.

Every assertion is non-vacuous and would fail loudly if production drifted. Phase 6's simulated write is the only "scope deferral" and it's explicitly disclosed in the docstring.

**Claim 6 test** (`crates/argo-audit/tests/claim_6_cross_tenant_approval_select_no_existence_leak.rs:161-340`):

- Seeds 4 approval kinds (`ApprovalAsked` / `ApprovalGranted` / `ApprovalReplied` / `ApprovalRevoked`) for tenant_a AND tenant_b with distinct action_id prefixes (`act-a-*` vs `act-b-*`).
- 4 axes of assertion:
  1. **Kind+tenant filter** — for each kind, tenant_a sees exactly 1 row (positive control); tenant_b sees ZERO rows for `act-a-*` ids.
  2. **Action_id existence-leak** — for each kind, sanity-asserts tenant_a sees its own action_id (precondition non-vacuous), then asserts tenant_b's `WHERE tenant_id = ? AND json_extract(payload_json, '$.action_id') = ?` SELECT returns 0 for tenant_a's owned action_id. The "404-not-403 substrate" contract.
  3. **Blanket audit-export shape** — `SELECT WHERE tenant_id = ?` returns exactly tenant_b's 4 rows; per-row assertion that every `action_id` starts with `act-b-` and none start with `act-a-`.
  4. **Hash-chain integrity** — `verify_session` still returns `ok=true, breaks=[]` after the cross-tenant write mix.

Every negative assertion is paired with a positive control. Non-vacuous.

### Verdict

🟢🟢 Both test sources are anti-theater-honest; both PASS cold on current `origin/main` HEAD; the CLAUDE.md claims-4+6 LIVE flip [docs/skycot/MISSION.md and CLAUDE.md lines 184-189] is corroborated.

## Scenario 2 — T09-50 merge-train closure audit (4 carry-forwards CLOSED)

### r50/r56 carry (g) `fix-agent-runner-legacy-worker-treated-as-lowest-grade-test` → CLOSED by [#2271](https://github.com/mark-hallam/Argo-Agent/pull/2271)

PR body root-cause analysis is clean:
- The test installed skills via `SkillStore::install(...)`. Post v79 schema (`mission-skill-table-add-tenant-id-column`), that legacy API stamps `__migration_placeholder__` tenant_id.
- The new SQL filter at `crates/argo-skill/src/store_available.rs:65` binds `s.tenant_id = ?` against the test's real tenant; placeholder rows are intentionally invisible.
- The argo-skill crate's own unit tests were updated to use `install_for_tenant` at the same time; this agent-runner test was missed.

**Option chosen**: (b) stale test asserting old behavior — properly justified in the PR body.
- Not (a) production fix — the cross-tenant filter IS the desired post-v79 behavior.
- Not (c) KNOWN_FLAKES.md mask — the test is deterministically red, not flaky.

Anti-theater verdict: HONEST closure. The test now uses the right API (`install_for_tenant`) so the legacy-worker behavior assertion still pins the right thing. Production untouched.

### r52 carry (l) `eval-skill-query-tenant-pin-extend-to-distinct-skill-verticals-isolation` → CLOSED by [#2270](https://github.com/mark-hallam/Argo-Agent/pull/2270)

Closes my r52 finding that the tenant_b→tenant_a direction of `distinct_skill_verticals` no-leak wasn't pinned (only tenant_a→tenant_b was).

PR body has the GOLD-STANDARD anti-theater proof:

```text
## Non-vacuous check
Temporarily removed `AND tenant_id = ?` from `distinct_skill_verticals` and reran the focused test. It failed as expected:

left: ["legacy-vertical", "shared-vertical", "tenant-a-only", "tenant-b-only"]
right: ["shared-vertical", "tenant-a-only"]

The tenant filter was restored before commit.
```

That's the exact recipe my r55 #2279 charter-clobber probe used (remove the production guard, watch the test fail, restore the guard). Symmetric coverage now pinned. CLEAN.

### r51 carry (j) `docs-cohorts-page-add-patrick-status-reporter-tile` → CLOSED by [#2280](https://github.com/mark-hallam/Argo-Agent/pull/2280)

Patrick (Status Reporter L3, M22 pack) now has his cohort tile alongside Peta (Project Coordinator L2, M21 pack). Brand-aligned with the cohorts-page-design-locked memory (D3 magazine tiles).

### r53 carry — marketing pages smoke after #2242 `NEXT_PUBLIC_` env-var rename → CLOSED by [#2286](https://github.com/mark-hallam/Argo-Agent/pull/2286)

Closes my r53/r56 carry: "curl-check landing/marketing pages after #2242 env-var rename — must not 500 per `feedback_styled_jsx_requires_use_client.md`".

### 6-round process-gap carry (r41-r53) → CLOSED by [#2273](https://github.com/mark-hallam/Argo-Agent/pull/2273)

The `conductor-pre-merge-gate.sh` script had 6 gates and **none of them ran cargo test**. `--skip-clippy` was effectively the only cross-crate sanity check. #2273 adds:

- Gate 6 `pre-merge-test-compile.sh` — non-skippable `cargo check --workspace --tests`
- `pre-merge-test-carry-allowlist.txt` — explicit allowlist for known-red carries (starts empty; origin/main currently green)
- New `--skip-tests` flag with loud multi-line warning (no-Rust-touched escape hatch only)
- Smoke harness `conductor-pre-merge-gate-tests.sh` with 3 cases (clean pass, unallowlisted broken test fails, allowlisted broken test PASS-WITH-WARNING)

This is **important defensive infrastructure**. Previously, several PRs landed on `origin/main` with red `cargo test` because the gate only caught clippy. Going forward, `--skip-clippy` PRs still get test-compile coverage.

Anti-theater verdict on #2273: HONEST. The test harness exercises 3 cases (clean / red+unallowlisted / red+allowlisted) so the gate's branches are pinned in CI before being trusted in production. The brief's 4th file (`pre-merge-test-compile.sh`) is flagged in the PR body for review-awareness — extracted from the gate script for the same reason the gate already delegates clippy/migration-slots/zero-caller checks.

### #2281 Track P P-04 hot-path canonical-Q&A skill — substrate POST-MERGE audit

16 files, +1785/-0. Foundation primitive for the v1.0 Pitch Agent demo. Lookup latency ~59µs on a 100-entry synthetic corpus (criterion-measured), three orders of magnitude under the storyboard's ≤50ms p95 budget for sub-300ms voice responses.

Each matched entry returns `declared_claim_classes` consumed by P-02's truth-boundary evaluator. Defense-in-depth for valuation/revenue/customer-count: canonical entries render operator-approved deferral text directly, so the gate's Forbid path is the fallback, not the primary surface.

**Implementation choice flagged in PR body for review**: IDF-weighted lexical matcher (zero extra deps) rather than embedding model (50MB `ort`/`candle`). Brief permitted either; the PR ships the lexical fallback per the brief's "flag the choice" guidance. Asymmetric scoring `sum(idf(t) for t in query ∩ prompt) / sum(idf(t) for t in query)` bounded `[0.0, 1.0]`, threshold default 0.45. Rewards prompts that explain the query without penalizing prompts that carry extra context — right shape for paraphrases.

**Lint policy choice flagged**: warning (not error) for unknown `declared_claim_class` values. P-01's docs say "Allow is the implicit default for any class not listed in a policy", so unknown ≠ error. Lint exits 0 with 26 warnings against the shipped skycot corpus (32 entries) + the P-01 policy.

Both implementation choices are honestly disclosed in the PR body with the rationale; both can be revisited in follow-up briefs if demo team finds gaps. **No anti-theater concerns** — the substrate ships its real performance claim with criterion-measured numbers, and the lint warnings are by-design not silent failures.

## Scenario 3 — Process-drift sweep

| Gate | State | Detail |
|---|---|---|
| Gateway freshness | 🟢 FRESH | `schema=v81=v81 drift=0`; uptime 14135s (~3.9h since 09:44Z restart per #2283) |
| Findings-only fix-by | 🟢 4/4 within budget | All `2026-05-30`, 5 days remaining |
| Unread SHARED.md handoffs | 🟡 2 entries | (a) supervisor-watchdog 11:38Z (informational); (b) **my own r56 evaluator handoff** — the T09-50 conductor tick (`549e9229b`) ran ~5 min before my r56 commit landed 09:55Z, so the race left my handoff unread. The next conductor tick should pick it up. |
| **SHARED.md stale merge markers** | 🔴 **STILL un-cleared** | **2nd-consecutive-round escalation.** Lines 342-349 still contain `<<<<<<< Updated upstream` / `=======` / `>>>>>>> Stashed changes` from the T09-10 conductor stash-recovery. Outside my write authority to resolve. |
| AEST→UTC stamp drift | 🟢 closed | #2249 `ops-heartbeat-utc-stamp-lint` shipped in the 2026-05-25 merge train; future drift now caught by lint. Historical SHARED.md not retroactively corrected. |
| Open PR count | low | git log shows ~14 ledger churn commits but minimal open-PR ageing |

### Recommendations for next conductor tick

1. **Clean SHARED.md lines 342-349** — manually edit out the `<<<<<<< Updated upstream` / `=======` / `>>>>>>> Stashed changes` markers (keep both halves' content intact; they're both valid conductor entries from T08-05 + T09-10).
2. **Action my r56 handoff** — #2272 PR has my PRE-MERGE PARTIAL verdict (MERGE-WITH-FOLLOWUP recommended) + brief candidate (o-NEW) for `mission-demo-seed-real-llm-driver` to add audit-row emission + `worker.charter` field set. Decide: merge #2272 + author the brief #2 edit, OR BLOCK #2272 until brief is corrected, OR fold the doc correction into a small docs-only PR first.
3. **Author tracking for #2287 charter-clobber fix** — r55 (n) brief `fix-kpi-rollup-preserve-charter-md` is `in review (#2287)` per `git log` (commit `519c33255`). Once #2287 PRE-MERGE verifies, close r55 carry (n).

## Brief candidates from this round

NONE NEW. **5 carry-forwards CLOSED this round** (g, l, j, post-#2242-smoke, 6-round-gate-gap). 1 new escalation:

- **🔴 P2 process-drift**: SHARED.md stale merge markers (lines 342-349) — 2nd-consecutive-round un-cleared. Recommendation: next conductor tick manually edits out the 3 conflict marker lines (line 342 `<<<<<<<`, line 346 `=======`, line 349 `>>>>>>>`) while preserving both halves' textual content. Trivial fix, but outside evaluator write authority.

## Operational notes

- Worktree `/tmp/argo-eval-r57` to be removed at end of round.
- Two cold-cargo runs in parallel did not trigger memory-pressure backoff; system stayed responsive.
- ~50 min wall-time round; ~45K tokens. Within budget.
