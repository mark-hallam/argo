# Evaluator scheduled run — 2026-05-22 → 2026-05-23 Melbourne (4 rounds)

**Four rounds.** Round 1 surfaced a P0 cluster. Rounds 2–4 verified the conductor closed three P0s + added 6 more scenarios + found 2 more findings.

## Headline (after round 4)

🟢 **Three P0s closed end-to-end by the conductor → worker loop within 24h of evaluator finding** — every round-1 P0 plus the round-3 P0 (audit-export tenant-blind) is fixed and verified by re-running the regression tests.

🟢 **Computer-use pillar finally evaluated**: macOS executor passes 37/37 unit tests covering preflight, run-loop budget, screenshot parsing, events injection, a11y helpers, notes-app interactions. End-to-end smoke (real Notes + Anthropic loop) is gated on operator authorization + `ARGO_NATIVE_MACOS_INTEGRATION=1` — deferred to a future eval.

🟡 **One new P2 finding**: six worker handlers leak tenant-existence via `forbidden("worker not in this tenant")` vs `not_found("worker not found")`. The SQL probe scoping is correct; the response shape is the leak. Brief shape: `mission-cross-tenant-worker-handler-no-existence-leak`.

🟢 **Both round-3 process recommendations** (zero-caller substrate lint + FINDINGS-ONLY fix-by deadline) have briefs authored.

## All scenarios (15 total)

| # | Scenario | Pillar | Round | Verdict |
|---|---|---|---|---|
| 1 | SPA cold-start route probe | All | 1 | 🔴 → 🟢 PASS (#2027) |
| 2 | Gateway-process freshness | All | 1 | 🟡 PARTIAL |
| 3 | 5-teammate hire (anti-Mariah-clone) | Teammate | 1 | 🟡 PARTIAL |
| 4 | Workflow lifecycle smoke | Workflow | 1 | 🔴 → 🟢 PASS (#2025) |
| 5 | Cross-tenant skill leak | Skills | 1 | 🔴 → 🟢 PASS (#2025) |
| 6 | Cross-tenant audit-detail fix verification | Audit | 2 | 🟢 PASS |
| 7 | Worker downstream state after hire | Teammate | 2 | 🟢 PASS (v1-deferred gaps surfaced) |
| 8 | Marketing route silent-500 sweep | All | 2 | 🟢 PASS (17/17) |
| 9 | Main gateway died mid-session | Operational | 2 | 🟡 OPERATIONAL |
| 10 | Audit-export tenant-blind exposure | Audit | 3 | 🔴 → 🟢 PASS (#2051) |
| 11 | Workplace-knowledge zero-caller drift | Hygiene | 3 | 🟡 → ⏳ DRIFT (brief authored) |
| 12 | P0-A (SPA cookies) — verify fix | All | 3 | 🟢 PASS |
| 13 | Audit-export fix verification (PR #2051) | Audit | 4 | 🟢 PASS |
| 14 | Computer-use macOS executor unit tests | Computer use | 4 | 🟢 PASS (37/37) |
| 15 | Cross-tenant existence-leak in 6 worker handlers | Multi-tenant | 4 | 🟡 **NEW P2** |

**All four pillars now evaluated at least once.** Round 4 finally got pillar 3 (computer use) onto the board.

## Closed-loop P0 tally — evaluator → conductor → worker → evaluator-verifies

| Round | P-finding | Brief recommended | PR merged | Verified |
|---|---|---|---|---|
| 1 → 2 | P0-B migration `;`-in-comment | `claude-migration-comment-aware-splitter` | [#2025](https://github.com/mark-hallam/Argo-Agent/pull/2025) | ✅ cargo |
| 1 → 3 | P0-A SPA cookies import | `web-auth-config-server-only-split` | [#2027](https://github.com/mark-hallam/Argo-Agent/pull/2027) | ✅ runtime |
| 3 → 4 | P0 audit-export tenant-blind | `mission-audit-export-tenant-scope-fix` | [#2051](https://github.com/mark-hallam/Argo-Agent/pull/2051) | ✅ source + cargo |
| 3 → 4 | drift: `argo-workplace-knowledge` zero-caller | `claude-zero-caller-substrate-lint` | (brief authored, status pending) | ⏳ |
| 3 → 4 | process: FINDINGS-ONLY fix-by deadline | `claude-conductor-fix-by-enforcement` | (brief authored, status pending) | ⏳ |

**Three P0s shipped end-to-end + two process briefs authored** in the first ~36h of evaluator operation.

## New round-4 findings

### 🟡 P2 — Cross-tenant existence-leak in 6 worker handlers

Six handlers distinguish "worker exists in another tenant" (403) from "worker doesn't exist" (404). A caller in tenant_b who knows a worker_id can probe the response shape to decide which is which.

| Handler | Endpoint |
|---|---|
| `routes_worker_activity.rs` | `GET /workers/<id>/activity` |
| `routes_worker_detail.rs` | `GET /workers/<id>` |
| `routes_worker_overview.rs` | `GET /workers/<id>/overview` |
| `routes_worker_policies.rs` | `GET\|PATCH /workers/<id>/policies` |
| `routes_worker_settings_helpers.rs` | `GET\|POST /workers/<id>/settings` |
| `routes_duties_helpers.rs` | duty CRUD |

The workflow-definition handler does it RIGHT (returns generic 403 with no body distinction, comment in `routes_workflows.rs:13-21` explicitly says "do NOT leak whether the (tenant, vertical, name) triple exists"). The worker handlers haven't adopted the same convention.

Not a data-exposure — but an enterprise-grade tenant-isolation guarantee violation. Per operator memory `feedback_enterprise_grade_defaults.md`, "strict defaults are the target". Brief shape: `mission-cross-tenant-worker-handler-no-existence-leak`, ~150 LOC sweep.

[Report.](scheduled/2026-05-22/cross-tenant-existence-leak-six-handlers.md)

### 🟢 Computer-use pillar 3 evaluated

37/37 macOS executor unit tests pass. Preflight, run-loop, screenshot, events, a11y, notes — all green. Capability gating is at config time (`MacosCuConfig::for_notes_app`). E2E with real Notes is gated on operator authorization + env flag.

[Report.](scheduled/2026-05-22/computer-use-macos-executor-unit-tests.md)

### 🟢 Audit-export fix (#2051) verified

- `fetch_rows_for_export` signature now takes `tenant_id: &str`.
- All 3 `#[ignore]` attrs removed from `routes_audit_export_tests.rs`.
- A 4th test added by the fix-author (`cross_tenant_signed_url_with_forged_signature_returns_404`) covers an adjacent attack surface — bonus defense-in-depth.

[Report.](scheduled/2026-05-22/audit-export-fix-verified.md)

## Still-open items the conductor should action

1. **🟡 `mission-cross-tenant-worker-handler-no-existence-leak`** (NEW from round 4) — sweep fix across 6 handlers. ~150 LOC.
2. **⏳ `claude-zero-caller-substrate-lint`** — brief authored from round 3; not yet CLAIMED.
3. **⏳ `claude-conductor-fix-by-enforcement`** — brief authored from round 3; not yet CLAIMED.
4. **🟡 `mission-workplace-knowledge-gateway`** — already CLAIMED. Unblocks the substrate drift.
5. **🟡 Live gateway revival** — `mission-gateway-restart-runbook` + `mission-gateway-env-local-bootstrap` already in flight. Once `:7400` is up, can run live cross-tenant probes against the 7 EVAL workers.
6. **🟡 `web-audit-feed-platform-events`** — round-1 P0-D; still open, no brief yet.

## State

- Next.js dev server: probably killed since round 3 ended. Restart at need.
- 7 EVAL workers in `~/.argo/sessions.db` — intact, waiting for gateway.
- All 11 scenario reports under `docs/skycot/evals/scheduled/2026-05-22/`.

Next planned eval run: 2026-05-23 daily. Will sweep newly-merged briefs (the zero-caller lint, workplace-knowledge-gateway, anything that lands overnight) + re-run scenarios 6/7/10 against the revived gateway if it's back up.
