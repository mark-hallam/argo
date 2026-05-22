# Evaluator scheduled run — 2026-05-22 → 2026-05-23 Melbourne (5 rounds)

**Five rounds.** First-ever evaluator session, ran ~36h. Now driving a real Chrome (round 5) in addition to the prior curl/cargo/Playwright layers.

## Headline

🟢 **Three P0s closed end-to-end by the conductor → worker loop, each in <24h of evaluator finding.**

🔴 **Two new P0s discovered in round 5 via Chrome MCP** — both invisible to curl/cargo/HTTP-status checks:
- **P0-E1 — `layout.ts` naming collision blanks the workflow builder canvas.** `web/app/(app)/workflows/[verticalId]/[name]/build/layout.ts` is a React Flow positioning utility, but Next.js App Router treats any `layout.ts` as a route layout and demands a default-exported React component. None exists → `"default export is not a React Component"` → blank page on the v1.0 demo MP4 segment 4 anchor.
- **P0-E2 — `@monaco-editor/react` + 6 transitive deps missing from `node_modules`** (declared in package.json). Turbopack dev-error overlay masks every page in dev. Same pattern as round 1's `@radix-ui/react-tooltip`. Fixed locally with one `pnpm install`.

🟡 **One new finding** — `tests/e2e/onboarding.spec.ts` requires `localhost:3001` (a sibling marketing site) that doesn't exist at `../marketing/`. The spec was broken by a sibling-repo move that never re-checked the e2e suite.

🟢 **Pillar 3 (computer-use) finally evaluated** in round 4. 37/37 macOS executor unit tests pass.

## All scenarios (17 total across 5 rounds)

| # | Scenario | Pillar | Round | Verdict |
|---|---|---|---|---|
| 1 | SPA cold-start route probe (HTTP-status) | All | 1 | 🔴 → 🟢 PASS (#2027) |
| 2 | Gateway-process freshness | All | 1 | 🟡 PARTIAL → still dead |
| 3 | 5-teammate hire (anti-Mariah-clone) | Teammate | 1 | 🟡 PARTIAL |
| 4 | Workflow lifecycle smoke (cargo) | Workflow | 1 | 🔴 → 🟢 PASS (#2025) |
| 5 | Cross-tenant skill leak (cargo) | Skills | 1 | 🔴 → 🟢 PASS (#2025) |
| 6 | Cross-tenant audit-detail fix verify | Audit | 2 | 🟢 PASS |
| 7 | Worker downstream state after hire | Teammate | 2 | 🟢 PASS |
| 8 | Marketing route silent-500 sweep | All | 2 | 🟢 PASS (17/17) |
| 9 | Main gateway died mid-session | Operational | 2 | 🟡 OPERATIONAL |
| 10 | Audit-export tenant-blind exposure | Audit | 3 | 🔴 → 🟢 PASS (#2051) |
| 11 | Workplace-knowledge zero-caller drift | Hygiene | 3 | 🟡 → brief authored |
| 12 | P0-A (SPA cookies) — verify fix | All | 3 | 🟢 PASS (#2027 verified) |
| 13 | Audit-export fix verification (PR #2051) | Audit | 4 | 🟢 PASS |
| 14 | Computer-use macOS executor unit tests | Computer use | 4 | 🟢 PASS (37/37) |
| 15 | Cross-tenant existence-leak in 6 handlers | Multi-tenant | 4 | 🟡 → brief authored |
| 16 | Playwright suite cold-start (49 specs) | All | 5 | 🔴 47/49 FAIL — masked by P0-E1+E2 |
| 17 | Chrome-driven workflow build canvas | Workflow | 5 | 🔴 **NEW P0** — blank page |

**All four pillars now covered.** Browser-driven is the new third layer (alongside cargo + curl).

## Closed-loop P0 tally

| P-finding | Brief | PR | Verified |
|---|---|---|---|
| P0-B migration `;`-in-comment | `claude-migration-comment-aware-splitter` | [#2025](https://github.com/mark-hallam/Argo-Agent/pull/2025) | ✅ |
| P0-A SPA cookies import | `web-auth-config-server-only-split` | [#2027](https://github.com/mark-hallam/Argo-Agent/pull/2027) | ✅ |
| P0 audit-export tenant-blind | `mission-audit-export-tenant-scope-fix` | [#2051](https://github.com/mark-hallam/Argo-Agent/pull/2051) | ✅ |
| drift: zero-caller crate | `claude-zero-caller-substrate-lint` | brief authored | ⏳ |
| process: FINDINGS-ONLY fix-by | `claude-conductor-fix-by-enforcement` | brief authored | ⏳ |
| P2 existence-leak in 6 handlers | `mission-cross-tenant-worker-handler-no-existence-leak` | brief authored | ⏳ |
| **(NEW round 5)** P0-E1 layout.ts | `web-workflow-build-layout-ts-rename` | — | TODO |
| **(NEW round 5)** P0-E2 monaco missing | `claude-dev-safe-check-deps-present` | — | TODO |

**3 P0s closed end-to-end + 3 lower-priority briefs authored + 2 new P0s surfaced** in first 36h of evaluator operation.

## What's left to action — ranked by demo-MP4 leverage

1. **🔴 `web-workflow-build-layout-ts-rename`** (round 5) — 5-LOC rename + 2 import updates. **Unblocks the workflow builder canvas, the v1.0 demo MP4 segment-4 anchor.** Most urgent.
2. **🟡 `claude-dev-safe-check-deps-present`** (round 5) — extend dev-safe-check to fail when `package.json` declares a dep that's missing from `node_modules`. Round 1 + round 5 both hit this pattern.
3. **🟡 `mission-cross-tenant-worker-handler-no-existence-leak`** — already authored, still pending.
4. **🟢 (in flight) `mission-workplace-knowledge-gateway`** + `mission-gateway-env-local-bootstrap` + `mission-gateway-restart-runbook`.

## What visual evaluation also confirmed (post-fix)

Design system holds at runtime. After `pnpm install`, with the dev-overlay dismissed, the following design-locks all render correctly:

- **`/`** (Approvals): E-ops-dashboard with 5-KPI strip + 78% donut + "All clear" empty state — matches `project_approvals_page_design_locked.md`.
- **`/workflows`**: Brand-loud marketplace with Audiowide hero "How your teammates actually think.", hero-search + try-pills, 4 workflow cards — matches `project_skills_page_design_locked.md`.
- **`/workflows/sales/qualify_lead`**: Workflow-detail with "Open builder" CTA + 5 step types — matches `project_workflow_goldens_page_design_locked.md`.

The bug is naming-convention + missing dep. Both fixable in <100 LOC.

## State left behind by round 5

- Next.js dev server running at `:3000` (PID per `/tmp/eval-web-dev-r5b.log`). Restarted with `NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:1` to match Playwright env.
- 7 EVAL workers still in `~/.argo/sessions.db`. Wait for fresh gateway.
- 4 Playwright traces in `web/test-results/` — most useful: `approvals-inbox-approves-the-first-card-under-mock-fallback/error-context.md` shows the testid that's missing.
- Chrome MCP paired to this session. Next eval run can pick up the browser layer cheaply.

Next planned run: 2026-05-23 daily. Will sweep the new fix-briefs + re-verify the builder canvas.
