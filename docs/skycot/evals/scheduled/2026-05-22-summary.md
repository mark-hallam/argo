# Evaluator scheduled run — 2026-05-22 (bootstrap)

**First-ever evaluator run.** Five scenarios executed against the live gateway (`:7400`, PID 28198, 92h uptime), the Next.js web dev server (cold-started here), and the cargo integration test suite.

## Headline

🔴 **Four P0 findings.** Three of them block the v1.0 demo MP4 ship-gate as currently authored.

- **P0-A — SPA-side**: `web/auth.config.ts:3` imports `cookies` from `next/headers`. The import chain `auth.config → auth.ts → lib/api/client.ts → use-approval-bundles.ts → app/(app)/page.tsx` pulls a server-only API into a Client Component bundle. **5 of 6 in-product routes return HTTP 500 on cold dev start** (`/auth/signup`, `/hire/step/1`, `/audit`, `/workflows` — only `/landing` works). Landed in [#1901](https://github.com/anthropics/argo/pull/1901). [Report.](scheduled/2026-05-22/spa-cookies-import-app-routes-500.md)

- **P0-B — migration-side**: PR [#1988](https://github.com/anthropics/argo/pull/1988) ("SAFETY: comments on 12 global tables", merged today) added `;` characters inside `-- ` line comments across **48 migration SQL files**. The fragile `split_terminator(';')` pattern in `crates/argo-store/src/migrations/v43.rs` (and v47, v51) hands raw English text to SQLite and dies with `near "operator": syntax error`. Concretely:
  - 🔴 [Claim 4 cross-tenant skill test](scheduled/2026-05-22/skill-cross-tenant-leak-blocked-by-migration-bug.md) (`claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak`) panics at line 192 before any cross-tenant assertion runs.
  - 🔴 [Workflow lifecycle smoke](scheduled/2026-05-22/workflow-build-save-publish-rollback.md) (`workflow_edit_api_lifecycle`) crashes its child gateway (`GatewayCrashed`) on fresh-from-zero migration boot.
  - 🔴 **Any operator who installs SKYCOT today** and boots a fresh `~/.argo/sessions.db` will see the gateway refuse to start.

- **P0-C — gateway staleness**: the running gateway (PID 28198) reports `schema_version: 70` but the codebase is at v76. ~30 PRs of new functionality (workflow rollback, sticky notes, sub-workflows, http+code nodes, hire-banner) are not in this binary. The conductor's PR-merge gate doesn't restart the gateway. The audit page surfaces 4 stale `hipaa_compliance_check` rows from 2026-05-18 and zero new events. [Report.](scheduled/2026-05-22/gateway-process-stale-vs-codebase.md)

- **P0-D — audit feed misses platform events**: the production audit feed at `GET /api/v1/audit` is scoped to action records (the 7 dim filters from v67) and does NOT surface `WorkerHired`, `WorkflowHeadChanged`, `WorkflowRolledBack`, etc. After 7 successful hires the audit page still showed 4 stale rows. The hire handler DOES write to `audit_record` (verified by source-read of `routes_workers_hire.rs`), but the operator's `/audit` view doesn't read those rows. [Report.](scheduled/2026-05-22/teammate-hire-five-distinct-personas.md)

## Verdict matrix (5 scenarios)

| # | Scenario | Pillar | Verdict | P-level |
|---|---|---|---|---|
| 1 | SPA cold-start route probe | All | 🔴 FAIL | P0-A |
| 2 | Gateway-process freshness | All | 🟡 PARTIAL | P0-C |
| 3 | 5-teammate hire (anti-Mariah-clone) | Teammate | 🟡 PARTIAL | P0-D |
| 4 | Workflow lifecycle smoke | Workflow | 🔴 FAIL | P0-B |
| 5 | Cross-tenant skill leak negative test | Skills | 🔴 NOT-COVERED | P0-B |

**Computer-use (pillar 3)** — not evaluated this run. Needs a macOS desktop driver session; logging as scope for a future run.

## What's actually GOOD (not buried)

- ✅ The hire API itself is solid. 5 distinct workers created with 5 distinct UUID v7s and 5 distinct persona names — **the anti-Mariah-clone gate passed cleanly**. Idempotency-key replay returns 200 with `idempotent: true`; same key + different body returns 409. Real, not theater.
- ✅ The workflow routes (read in source) look architecturally healthy — `routes_workflows.rs` covers all 8 verb/path combinations, wires audit emissions for all 4 mutating verbs, and has unit tests for the route parser. Only blocker is the fresh-DB migration.

## P0 fix-recipe summary for the conductor

Three briefs unblock everything below the SPA seam:

1. **`claude-migration-comment-aware-splitter`** — strip `-- ...\n` lines before `split_terminator(';')` in `v43.rs`, `v47.rs`, `v51.rs`. Add a one-shot unit test pinning a `;-in-comment` scenario. **This single fix unblocks both #4 and #5, and unblocks every operator's fresh-install path.**
2. **`web-auth-config-server-only-split`** — move the `cookies()` call out of `auth.config.ts` into a server-only sibling. Unblocks the entire `/app/(app)/*` route tree.
3. **`mission-gateway-restart-on-merge`** *(operator-decisional)* — decide policy for the running gateway: either auto-restart on merge, or surface `schema_version` drift in the conductor tick prompt so it's actioned manually. Without this, even after #1 + #2 land, the demo MP4 path still 404s on workflow routes.

Smaller follow-ups (P1):
- `web-audit-feed-platform-events` — UNION platform events into the audit feed so hires/publishes show up where operators look.
- `web-pnpm-install-missing-on-cold-start` — `@radix-ui/react-tooltip@1.2.8` was declared in `package.json` but missing from `node_modules`. `pnpm install` fixed it; investigate why CI didn't catch.
- `claude-vertical-install-greeter-tool-allowlist-fix` — separate failing test (`cli_install_then_workflow_run_round_trip`) hits a vertical's `[tools].denied` list.

## State left behind

- **7 EVAL-prefixed workers** created in the cwd-derived workspace tenant via the live gateway. Names: `EvalEva-DataEntry`, `EvalCleo-Support`, `EvalHarold-Bookkeep`, `EvalMaya-Onboard`, `EvalRina-LeadResearch`, `EvalDup-First`, `EvalReplay`. Workspace tenant was empty before this eval. Easy to filter via `name LIKE 'Eval%'`.
- Next.js dev server still running at `:3000` (PID 72330, started this session). Safe to kill — only used by this evaluator.
- Gateway (PID 28198, 92h) untouched.

## Bookkeeping

- `docs/skycot/agents/heartbeats/evaluator.json` written.
- `docs/skycot/agents/SHARED.md` append-only entry made at session start.
- All 5 scenario reports under `docs/skycot/evals/scheduled/2026-05-22/`.
- This summary lives at `docs/skycot/evals/scheduled/2026-05-22-summary.md`.
- `LATEST.md` copy of this summary at `docs/skycot/evals/scheduled/LATEST.md`.

Next planned run: 2026-05-23 daily. Will rerun the two cargo-test scenarios first thing — if P0-B has been fixed, the rest of the workflow + skill scenarios open up.
