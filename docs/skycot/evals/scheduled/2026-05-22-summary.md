# Evaluator scheduled run — 2026-05-22 (3 rounds, spans into 2026-05-23 Melbourne)

**Three rounds.** Round 1 fired a P0 cluster + 1 partial. Round 2 verified the conductor closed the biggest P0 (#2025) within 8h + added 4 scenarios. Round 3 verified the SECOND round-1 P0 also closed (#2027) + found one new P0 + one drift finding.

## Headline (after round 3)

🟢 **The evaluator → conductor → worker loop closed BOTH round-1 P0s within 24 hours.** Briefs I recommended in round-1 were authored by the conductor (same names) and shipped as:

- PR [#2025](https://github.com/mark-hallam/Argo-Agent/pull/2025) → `claude-migration-comment-aware-splitter` ← round-1 P0-B
- PR [#2027](https://github.com/mark-hallam/Argo-Agent/pull/2027) → `web-auth-config-server-only-split` ← round-1 P0-A

Both verified at runtime / cargo-test level. The migration fix is the broadest — it unblocked every `:memory:` integration test that needed `migrate()` from v0, plus every fresh `~/.argo/sessions.db` install path. The SPA fix re-opened the entire `/app/(app)/*` route tree.

🔴 **One new P0 discovered**: `POST /api/v1/audit/export` is tenant-blind. The swarm's own brief [#2046](https://github.com/mark-hallam/Argo-Agent/pull/2046) confirmed it and shipped 3 `#[ignore]`'d regression tests as MERGED-FINDINGS-ONLY pending a fix-brief that hasn't been authored.

🟡 **One drift finding**: `argo-workplace-knowledge` crate ships in the workspace with zero `use` consumers across the codebase. Substrate + audit-kinds landed; the gateway that wires them is BLOCKED. The brief-pre-flight gate catches the downstream BLOCKED state, but the substrate's merge wasn't gated on consumer-existence.

## All scenarios this session (12 total)

| # | Scenario | Pillar | Round | Verdict |
|---|---|---|---|---|
| 1 | SPA cold-start route probe | All | 1 | 🔴 FAIL → 🟢 PASS (round 3) |
| 2 | Gateway-process freshness | All | 1 | 🟡 PARTIAL |
| 3 | 5-teammate hire (anti-Mariah-clone) | Teammate | 1 | 🟡 PARTIAL |
| 4 | Workflow lifecycle smoke | Workflow | 1 | 🔴 FAIL → 🟢 PASS (round 2) |
| 5 | Cross-tenant skill leak | Skills | 1 | 🔴 FAIL → 🟢 PASS (round 2) |
| 6 | Cross-tenant audit-detail fix verification | Audit | 2 | 🟢 PASS |
| 7 | Worker downstream state after hire | Teammate | 2 | 🟢 PASS (v1-deferred gaps surfaced) |
| 8 | Marketing route silent-500 sweep | All | 2 | 🟢 PASS (17/17) |
| 9 | Main gateway died mid-session | Operational | 2 | 🟡 OPERATIONAL |
| 10 | Audit-export tenant-blind exposure | Audit | 3 | 🔴 **NEW P0** |
| 11 | Workplace-knowledge zero-caller drift | Hygiene | 3 | 🟡 DRIFT |
| 12 | P0-A (SPA cookies) — verify fix | All | 3 | 🟢 PASS (verified #2027) |

**Computer-use (pillar 3)** — still not evaluated. Needs macOS desktop driver session, deferred to a future run.

## What's now open vs closed

### 🔴 NEW P0 — audit-export tenant-blind

`POST /api/v1/audit/export` → `fetch_rows_for_export(store, window)` scans `audit_record` globally. No `tenant_id` filter. Caller at tenant_a can request a signed bundle covering any time window and receive every tenant's audit rows.

PR [#2046](https://github.com/mark-hallam/Argo-Agent/pull/2046) shipped as `MERGED-FINDINGS-ONLY` with 3 `#[ignore]`'d regression tests. No fix-brief in the queue yet.

**Brief I recommend**: `mission-audit-export-tenant-scope-fix` — same shape as #1992's audit_detail fix. Add `tenant_id` to `fetch_rows_for_export` + `fetch_preview` signatures, bind it from `request_tenant_id(req)`, and remove the 3 `#[ignore]` attrs. Acceptance: all 3 tests pass + a 4th test for cross-tenant signed download URL.

**Process recommendation**: every `MERGED-FINDINGS-ONLY` commit should carry a `fix-by <date>` line; conductor's tick should fail-loud if a FINDINGS-ONLY merge passes its fix-by without a fix-PR.

[Report.](scheduled/2026-05-22/audit-export-tenant-blind-exposure.md)

### 🟡 NEW Drift — `argo-workplace-knowledge` zero-caller

```
crates/argo-workplace-knowledge/src/lib.rs           (substrate — exists)
crates/argo-audit/src/event_kind.rs                  (variants — defined, never emitted)
crates/argo-store/migrations/129__workplace_knowledge.sql  (tables — created, never read)
crates/argo-cli/src/handlers/                        (handlers — ABSENT)
```

Substrate brief MERGED. Audit-kinds brief MERGED. Gateway brief BLOCKED on its own pre-flight refusing to claim because no consumer exists yet (irony — it IS the would-be consumer). Editor SPA brief BLOCKED on gateway.

Recommend `claude-zero-caller-substrate-lint` — a CI script that flags any workspace crate with zero `use <crate>` consumers across other crates.

[Report.](scheduled/2026-05-22/workplace-knowledge-zero-caller-drift.md)

### 🟢 P0-A — SPA cookies import CLOSED

PR [#2027](https://github.com/mark-hallam/Argo-Agent/pull/2027) split `auth.config.ts` exactly as the recipe specified. All round-1 failing routes now return 200 on cold dev start:

```
/landing 200 · /auth/signup 200 · /audit 200 · /workflows 200 · /hire 200 · / 200
/hire/step/1 404 (by-design — wizard's step-1 is /hire) · /onboarding 404 (stale)
```

[Report.](scheduled/2026-05-22/p0-a-spa-cookies-fix-verified.md)

### 🟢 P0-B — Migration semicolon-in-comment CLOSED

PR [#2025](https://github.com/mark-hallam/Argo-Agent/pull/2025) introduced `split_sql_statements` (line-based, comment-aware) and swapped 9 callsites — 3 versioned migrations plus 6 crate-scoped helpers I hadn't even identified in round 1. Verified by:

- `cargo test -p argo-skill --test end_to_end_claim4_loop ...` → ✅ 1 passed
- `cargo test -p argo-cli --test workflow_edit_api_smoke ...` → ✅ 1 passed

[Original report.](scheduled/2026-05-22/skill-cross-tenant-leak-blocked-by-migration-bug.md)

### 🟢 Cross-tenant audit-detail fix CLOSED

PR [#1992](https://github.com/mark-hallam/Argo-Agent/pull/1992)'s unit test passes (`detail_returns_404_for_cross_tenant_action_id` → ok). Mission claim 6 closed-loop verified at the audit_detail surface.

[Report.](scheduled/2026-05-22/audit-cross-tenant-fix-verification.md)

### 🟡 Main gateway still dead

`:7400` still refuses connections. The conductor has authored `mission-gateway-restart-runbook` (per `5df7add84` brief auto-fire from auditor T09-40) and `mission-gateway-env-local-bootstrap` (per `7293b7c39` — "close the 5-footgun cold-start trap") and these are now CLAIMED. Not blocking — once a fresh gateway is up, I can probe the 7 EVAL workers from round 1 + verify the audit-export bug with a real bundle download.

[Report.](scheduled/2026-05-22/gateway-died-mid-session.md)

### 🟡 P0-D — Audit feed missing platform events (round 1)

Still open. Brief candidate `web-audit-feed-platform-events` not yet in the queue. Workers in flight on related work (`web-audit-feed-platform-events` flagged in commits — let me verify).

## What I'd ask the conductor to action next (ranked)

1. **🔴 Author `mission-audit-export-tenant-scope-fix`** — fix the actual data-exposure that #2046 documented but didn't fix. Removes 3 `#[ignore]` attrs. ~50 LOC + tests.
2. **🟡 `claude-merged-findings-only-fix-by-deadline`** — CI rule that flags FINDINGS-ONLY commits aged past their fix-by date. Small.
3. **🟡 `claude-zero-caller-substrate-lint`** — CI rule that flags workspace crates with no `use` consumers. Would have caught the workplace-knowledge drift at merge time.
4. **🟢 (already in flight) `mission-gateway-restart-runbook` + `mission-gateway-env-local-bootstrap`** — bring `:7400` back up against the correct binary path. Once up, re-run scenarios 6/7/10 against live state.
5. **🟢 (already in flight) `mission-workplace-knowledge-gateway`** — unblock the substrate drift by shipping the consumer. Already P0 on the auditor's 3rd-consecutive-escalation list.

## State left behind

- Next.js dev server running at `:3000` (PID per `/tmp/eval-web-dev-r3.log`). Safe to kill.
- 7 EVAL workers still in `~/.argo/sessions.db`. Wait for fresh gateway.
- All 8 scenario reports in `docs/skycot/evals/scheduled/2026-05-22/`.

Next planned eval run: 2026-05-23 daily.
