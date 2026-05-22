# Evaluator scheduled run — 2026-05-22

**Two rounds.** Round 1 fired a P0 cluster + a partial. Round 2 verified the conductor closed the biggest P0 within 8 hours and added 4 more scenarios.

## Headline (after round 2)

🟢 **The evaluator → conductor → worker loop is working as designed.** Round 1's P0-B (migration semicolon-in-comment) was authored as the brief I recommended (`claude-migration-comment-aware-splitter`), claimed, shipped, and merged as PR [#2025](https://github.com/mark-hallam/Argo-Agent/pull/2025) in <8 hours. After the fix, the two cargo tests that round 1 caught failing (`claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak` + `workflow_edit_api_lifecycle`) both pass cleanly.

**Remaining P0**: still the SPA cookies() import bug (P0-A) — `web/auth.config.ts:3` pulls server-only `next/headers` into a Client Component bundle, breaking every `/app/(app)/*` route. Brief `web-auth-config-server-only-split` (or similar). Same recipe as P0-B: split server-only from client-safe.

## Scenarios this session (9 total)

| # | Scenario | Pillar | Round | Verdict |
|---|---|---|---|---|
| 1 | SPA cold-start route probe | All | 1 | 🔴 FAIL — P0-A |
| 2 | Gateway-process freshness | All | 1 | 🟡 PARTIAL |
| 3 | 5-teammate hire (anti-Mariah-clone) | Teammate | 1 | 🟡 PARTIAL |
| 4 | Workflow lifecycle smoke | Workflow | 1 | 🔴 FAIL (now ✅ after #2025) |
| 5 | Cross-tenant skill leak | Skills | 1 | 🔴 FAIL (now ✅ after #2025) |
| 6 | Cross-tenant audit fix verification | Audit | 2 | 🟢 PASS |
| 7 | Worker downstream state after hire | Teammate | 2 | 🟢 PASS (v1-deferred gaps surfaced) |
| 8 | Marketing route silent-500 sweep | All | 2 | 🟢 PASS (17/17) |
| 9 | Main gateway died mid-session | Operational | 2 | 🟡 OPERATIONAL |

**Computer-use (pillar 3)** — not evaluated. Needs a macOS desktop driver session.

## Findings ranked by what's left to do

### 🔴 Still P0 — SPA cookies() import

`web/auth.config.ts:3` — `import { cookies } from "next/headers"` — pulled into a Client Component bundle via `auth.config.ts → auth.ts → lib/api/client.ts → use-approval-bundles.ts → app/(app)/page.tsx`. Result: `/auth/signup`, `/hire/step/1`, `/audit`, `/workflows` all return HTTP 500 on cold dev start. Landed in PR [#1901](https://github.com/mark-hallam/Argo-Agent/pull/1901).

**Concrete fix recipe**: split `auth.config.ts` into a client-safe `authConfig` constant + a server-only sibling that owns the `cookies()` call. Acceptance: cold `pnpm dev` returns 200 (or 302 to /auth/signin) on all 6 probe routes.

[Report.](scheduled/2026-05-22/spa-cookies-import-app-routes-500.md)

### 🟢 Round-2 follow-on: P0-B closed in 8h (the win story)

Round 1 found:
- 48 migration SQL files have `;` characters inside `-- ` line comments (added by PR #1988 today).
- The `split_terminator(';')` pattern in `v43.rs`, `v47.rs`, `v51.rs` chops comments on the embedded semicolons.
- Symptom: `near "operator": syntax error` from SQLite during fresh-from-zero migrations.
- Blast radius: every fresh install + every `:memory:` integration test.

Round 2 verified PR [#2025](https://github.com/mark-hallam/Argo-Agent/pull/2025) (`store(migrations): comment-aware SQL splitter`) merged today:
- Introduces `argo_store::migrations::split_sql_statements`.
- Swaps **9 callsites** (3 versioned migrations + 6 crate-scoped helpers I didn't even surface in round 1: `worker_archive`, `worker_deps`, `worker_kpi`, `worker_okr`, `learned_plan_template`, plus one in `migrations/mod.rs`).
- Both regression tests now pass:
  - `cargo test -p argo-skill --test end_to_end_claim4_loop ...` → 1 passed.
  - `cargo test -p argo-cli --test workflow_edit_api_smoke ...` → 1 passed.

[Reports.](scheduled/2026-05-22/skill-cross-tenant-leak-blocked-by-migration-bug.md) [And.](scheduled/2026-05-22/workflow-build-save-publish-rollback.md)

### 🟢 Audit cross-tenant scoping verified (mission claim 6 closed loop)

PR [#1992](https://github.com/mark-hallam/Argo-Agent/pull/1992) scoped `audit_detail` + `audit_replay` + `retros` row-fetches by tenant. The bug was: `SELECT … FROM audit_record WHERE id = ?` (no tenant scoping). The fix: `SELECT … FROM audit_record WHERE id = ? AND tenant_id = ?`.

Unit test that pins this: `detail_returns_404_for_cross_tenant_action_id` — seeds a row under tenant A, queries under tenant B, asserts 404. ✅ Passes.

This closes the marketing-vs-reality gap the auditor flagged 2026-05-21: the Friday founder-post `2026-05-22/linkedin-personal-founder-audit-found-real-bugs.md` can now ship truthfully — audit found bugs → workers shipped fixes within hours.

[Report.](scheduled/2026-05-22/audit-cross-tenant-fix-verification.md)

### 🟡 Operational — main gateway died mid-session

`http://127.0.0.1:7400` no longer accepts connections. PID 28198 gone. `~/.argo/gateway.pid` removed. `~/.argo/gateway.log` shows a failed restart attempt against `./target/release/argo` — but the gateway runs from `/tmp/argo-shared-target/release/argo`, so the watchdog command was wrong.

Good news: the conductor already authored `gateway-restart-runbook` brief (per `5df7add84` in git log — "from auditor T09-40"). Coordinated with this finding.

The 7 EVAL workers from round 1 + the conductor's workspace state are intact in `~/.argo/sessions.db` (3.3 MB file untouched). They re-appear as soon as someone brings up a gateway with the correct binary path.

[Report.](scheduled/2026-05-22/gateway-died-mid-session.md)

### 🟡 Marketing routes — all 17 healthy

`/landing`, `/compare`, `/pricing`, `/customers`, `/docs`, `/changelog`, `/security`, `/privacy`, `/terms`, `/sla`, `/help`, plus `/for/{finance,marketing,ops,recruiting,sales,support}` — all return HTTP 200 on cold `pnpm dev`. The customer-acquisition surface is solid up to the "Hire your first worker" call-to-action. The breakage starts the moment you cross into the auth-gated tree (P0-A).

[Report.](scheduled/2026-05-22/worker-downstream-state-after-hire.md) [Side notes.](scheduled/2026-05-22/teammate-hire-five-distinct-personas.md)

### 🟢 Worker downstream state — real, not theater

Hire writes 4 SQL rows in a single transaction (persona / role / worker / duty). Memory namespace allocated. Trust grade defaulted to 0. Channels + KPIs intentionally empty at hire (v1-deferred to post-hire flows). The Mariah-card mockup's animated KPIs are "what it looks like after a few days", not "immediately after hire" — would be worth a one-line empty-state copy on fresh hires.

All 6 `argo-identity` inline tests pass against current `main`.

[Report.](scheduled/2026-05-22/worker-downstream-state-after-hire.md)

## State left behind by round 2

- Next.js dev server running at `:3000` (restarted in this session, PID printed in /tmp/eval-web-dev-round2.log). Safe to kill.
- 7 EVAL workers still in `~/.argo/sessions.db`. Wait for a fresh gateway to query them.
- All scenario reports in `docs/skycot/evals/scheduled/2026-05-22/`.

## What I'd ask the conductor to action next

1. **Author `web-auth-config-server-only-split`** to close P0-A. Recipe in [the round-1 SPA report](scheduled/2026-05-22/spa-cookies-import-app-routes-500.md#gap--follow-up-brief-candidates).
2. **Run `mission-gateway-restart-runbook`** (already on the queue) and bring `:7400` back up against `/tmp/argo-shared-target/release/argo`. Once up, I can run scenarios 6 + 7 + the audit-feed surface check against the real DB.
3. **Author `web-audit-feed-platform-events`** to UNION platform events (WorkerHired, WorkflowHeadChanged, WorkflowRolledBack) into the operator's `/audit` view — round-1 P0-D, still open.

Next planned eval run: 2026-05-23 daily.
