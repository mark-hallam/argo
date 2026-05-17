# Claude conductor full audit — 2026-05-17

**Auditor:** Claude (conductor session, opus-4-7)
**HEAD inspected:** `9b3e45bc` (origin/main 2026-05-17)
**Method:** Read-only, evidence-anchored. Parallel sub-agent research (3 cells) + direct file/script checks. No code modified.
**Predecessor:** [2026-05-17-readonly-codebase-audit.md](2026-05-17-readonly-codebase-audit.md) (codex audit, validated separately).
**Scope this audit:** Categories the codex audit did not deeply cover — brand/UX coherence, mission-claim runtime trace, demo-recording readiness, swarm efficacy, documentation truth, and the operational/security blind-spots the codex audit flagged but didn't probe.

## Executive summary

13 findings (3 P0, 5 P1, 5 P2). The codex audit's 13 findings stand verified; this audit adds 13 more from adjacent surfaces.

The most consequential finding: **tonight's demo recording is blocked**. PR #1796 (magic-link auth recovery) is still DIRTY/CONFLICTING; the gateway has no signup/verify route on main. The dry-run script `scripts/demo-dry-run.sh` will red on scenes 2 through 8 if run against current main. Operator should not record a take tonight without #1796 landing first.

The swarm is producing genuine value at very high velocity (4hr brief→merge median, 2% follow-up rate, 0 silent regressions detected in last 50 PRs). The risk class has shifted from "is the swarm working" to "is the swarm out-running its safety nets." All 50 recent PRs admin-merged with conclusion=FAILURE across every CI check (GHA billing block); real CI signal is dark.

Brand, mission claims, and documentation are in better shape than I expected going in. Brand is 7.5/10 with minor easing/grid drift. 5 of 6 mission claims are runtime-PROVEN; claim 4 (skills) is PARTIAL — the wiring is in but the closure (CU-trajectory → skill propose → marketplace publish) hasn't been runtime-traced end-to-end. Docs have ~30% drift but mostly cosmetic (stale snapshot dates, incomplete CLI reference).

## P0 findings

### P0-1 — Tonight's demo recording is blocked on #1796 magic-link

The demo script's scene 1 starts with `https://skycot.com` → click "Hire your first worker" → SPA `/signup` → magic-link → `/auth/verify` → `/onboarding/welcome`. The dry-run script (`scripts/demo-dry-run.sh`) and Playwright spec (`scripts/demo-dry-run-fixtures/dry-run.mjs`) post to `${GATEWAY}/api/v1/auth/signup`.

**Evidence:**

- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` is 112 LOC and contains ONLY `check_bearer_auth` + `is_public_get`. No `signup`, no `verify`, no `magic_link`, no `MagicLink` references on main at `9b3e45bc`.
- `crates/argo-cli/src/handlers/gateway/auth_magic_link.rs` does not exist on main.
- PR [#1796](https://github.com/mark-hallam/Argo-Agent/pull/1796) (claude-rebase-pr-1756) carries the magic-link surface but is `mergeable=CONFLICTING, mergeStateStatus=DIRTY` (last update 2026-05-17T10:30Z).
- Web Next.js API routes (`web/app/api/auth/signup/route.ts`, `web/app/api/auth/verify/route.ts`) proxy to `${gatewayBase()}/api/v1/auth/signup` and `/api/v1/auth/verify`. Both will receive 404 from a current-main gateway.

**Impact:** Dry-run reds on scenes 2-8 (every signup-dependent scene). Operator-side: a recording take tonight would visibly fail in the first 90 seconds and cost a full setup.

**Needed:** Either (a) get #1796 rebased + merged before tonight (highest leverage — operator explicit-approved the as-is admin-merge once green), OR (b) postpone recording, OR (c) record only scenes 0 (cold-open) + a manual narration-over-static of the rest (degrades the "single take" claim materially).

### P0-2 — `ApprovalAsked` / `ApprovalReplied` are not first-class `AuditKind` variants

Mission claim 6 ("multi-channel approval gates") says the audit trail is `TurnStart → ToolDispatch → ApprovalAsked → ApprovalReplied → ActionExecuted → TurnComplete`. The `AuditKind` enum at `crates/argo-audit/src/event.rs` has `ApprovalGranted`, `ApprovalConsumed`, and `ApprovalExpiredByTimeout` — but no `ApprovalAsked` or `ApprovalReplied`.

**Evidence:**

```
$ grep -nE 'Approval(Asked|Replied)' crates/argo-audit/src/event.rs
# returns empty
$ grep -nE 'ApprovalGranted|ApprovalConsumed' crates/argo-audit/src/event.rs
crates/argo-audit/src/event.rs:48:    ApprovalGranted,
crates/argo-audit/src/event.rs:56:    ApprovalConsumed,
crates/argo-audit/src/event.rs:148:    ApprovalExpiredByTimeout,
```

Architecture diagram `21-audit-chain.md` shows `ApprovalAsked` + `ApprovalReplied` as documented variants. Architecture doc claims a chain shape that the code can't actually emit.

**Impact:** When an action lands in `ApprovalAsked` state (mission claim 6's hallmark), the audit chain doesn't capture the ask itself — only the eventual `ApprovalGranted`/`ApprovalConsumed` after operator response. Forensic-quality incident reconstruction for "an approval was requested at T, operator responded at T+5min" requires both rows.

**Needed:** Add `ApprovalAsked` + `ApprovalReplied` variants to `AuditKind` + emit at the existing fire-sites in `worker_actions_notify.rs::dispatch_notify` and the inbound-reply handlers. Mechanical — ~80 LOC + one migration if any persisted index relies on the kind label.

### P0-3 — SQLCipher off by default; SQLite writes plaintext pages

The DB at `~/.argo/db.sqlite` holds every audit row, memory row, worker config, tenant table, and (post-#1807) session JWT-bearing data. `crates/argo-store/src/encryption.rs` issues `PRAGMA key` via `apply_encryption_pragma`, but default cargo builds do **not** link SQLCipher — so the PRAGMA silently no-ops and the file is plaintext.

**Evidence:**

```
$ head -20 crates/argo-store/src/encryption.rs
//! At-rest encryption key resolution (Stream
//! `mission-encryption-at-rest-sqlcipher`, extended by
//! [`apply_encryption_pragma`] helper that issues `PRAGMA key` as the
//! [...]
//! # SQLCipher integration
//! is not linked against SQLCipher. Stock builds therefore silently
//! ignore `PRAGMA key` and write plaintext pages — the helper is the
//! [...] Enabling the `sqlcipher` Cargo feature is the operator-facing
//! toggle that downstream builds (FIPS / SQLCipher Commercial) flip;
```

The brief itself documents the gap. Default builds (including the one used in `cargo build --release --bin argo` per the demo setup playbook) are plaintext.

**Impact:** Anyone with filesystem access to the operator's machine OR a backup of `~/.argo/` can `sqlite3` the DB and read every tenant's data. For v1 self-hosted demo, low risk. For enterprise deployments, this is a SOC2/HIPAA/PCI-class issue.

**Needed:**
- For v1 demo: document explicitly that at-rest encryption is OFF unless cargo feature flipped.
- Before enterprise customers: stand up a SQLCipher-linked build path (`cargo build --features sqlcipher` against a SQLCipher-linked SQLite driver). Existing brief (`mission-encryption-at-rest-sqlcipher`) covers this; verify status.

## P1 findings

### P1-1 — 45 source files in the 450-500 LOC at-risk band

Post #1808 split-7-files, the loc-budget gate is GREEN. But the headroom is tight: 45 files sit in the 450-500 LOC band, with one (`cli_subcommands_skill.rs`) exactly at 500. Any new mount-line, new variant, or new test added to these files trips the gate.

**Evidence:**

```
$ find crates -name '*.rs' -exec wc -l {} \; | awk '$1 >= 450 && $1 <= 500 {c++} END {print c}'
45
$ find crates -name '*.rs' -exec wc -l {} \; | awk '$1 > 500 {c++} END {print c+0}'
0
$ # top 10 at-risk:
500 crates/argo-cli/src/cli_subcommands_skill.rs
495 crates/argo-executor-computer-use-macos/src/executor.rs
494 crates/argo-mcp/src/pool.rs
492 crates/argo-orchestrator/src/persist.rs
490 crates/argo-cli/src/handlers/worker_actions_auto_approval.rs
490 crates/argo-cli/src/handlers/gateway/sso_oidc.rs
488 crates/argo-skill/src/ir/types_tests.rs
488 crates/argo-audit/src/sink.rs
487 crates/argo-skill-sdk/src/builders.rs
486 crates/argo-cli/src/handlers/worker_actions.rs
```

**Impact:** The "claude-jwt-cookie-security-hardening" worker hit exactly this — pre-claim refusal because its may-touch list included a file near the ceiling. The boot-split (#1801) fixed boot.rs, but the band of 45 other files will produce identical refusals as workers try to add features there.

**Needed:** A second `codex-split-files-450-500-wave` brief that takes the top 10 (especially `cli_subcommands_skill.rs` at exactly 500 LOC) and drops them to ≤420. Pure mechanical, fast-mode. ~400 LOC.

### P1-2 — Brand execution drift: motion easing + spacing grid violations

Brand foundation is solid (tokens locked, EmptyState primitive shipped, Sonner toasts shipped, dark mode tokenized correctly). Execution has 3 specific drifts:

1. **Motion easing.** `web/components/ui/Button.tsx:11` uses `ease-out` (hardcoded); DESIGN_TOKENS.md mandates `ease-in-out` (cubic-bezier(0.4, 0, 0.2, 1)) for all transitions. `web/components/skycot/audit/ReplayProgress.tsx:58` uses `duration-300 ease-out` instead of `var(--motion-base)` + `var(--motion-ease)`.

2. **Spacing grid.** `gap-1.5` (= 6px, off the 8pt grid) appears in 8+ UI components — `Card.tsx:34,41`; `Breadcrumb.tsx:26,27`; others. DESIGN_TOKENS.md specifies space-1=4px, space-2=8px. 6px isn't a token.

3. **Responsive coverage asymmetry.** Marketing components use `sm:`/`md:` breakpoints extensively (LandingHero, Nav, KeyFact = 33 responsive uses). SPA UI primitives (Card, Button, Dialog, Toast) rarely use them. Mobile rendering of the SPA is untested + likely cramped.

**Needed:** One brief `mission-brand-execution-polish-pass` covering all 3 (easing fixes ~10 LOC, grid fixes ~30 LOC, mobile breakpoints ~100 LOC). Brand audit by sub-agent rated overall 7.5/10 — these 3 P1s would lift it to 9+.

### P1-3 — Mission Claim 4 (Skills shared + learned) is PARTIAL

Sub-agent runtime trace per mission claim:

| Claim | Status | Notes |
|---|---|---|
| 1: Teammate, not tool | ✅ PROVEN | Worker identity persists, persona/role/charter, memory namespace, auto-approval threaded |
| 2: Delegation is the unit | ✅ PROVEN | Duty scheduler spawned on boot, fires on cron, auto-approval gate end-to-end |
| 3: Platform builds workers | ✅ PROVEN | Genesis pipeline tested, vertical packs generate, safety judge hardens sensitive actions |
| 4: Skills shared + learned | ⚠️ **PARTIAL** | See below |
| 5: MCP + computer-use | ✅ PROVEN | 37 MCP connectors, macOS CU + E2B, executor_kind routing |
| 6: Multi-channel approval | ✅ PROVEN (outbound), ⚠️ inbound-loop audit-gap (see P0-2) |

For claim 4: substrate exists (`argo-skill::learn`, `per_worker_learning_cron`, `marketplace`, `publish`, library/workspace layers). `LayeredLookup::with_library` IS wired at `crates/argo-agent-runner/src/turn_helpers.rs:65` (the codex audit's earlier "missing" claim was based on `turn.rs`, but the wiring moved to `turn_helpers.rs` and IS live).

**Gap:** Dreams distillation pipeline (V2.0 Integration B) and CU-trajectory → SkillCandidate dispatch-shim closure are substrate-ready but runtime hookup not end-to-end traced in this audit. F-11 selector landed (#1740 MERGED), so relevance filtering is in. What remains: prove that an action's outcome triggers `SkillCandidate::from_cu_trajectory` → `propose_for_review` → marketplace publish for a worker actually running.

**Needed:** Brief `mission-claim-4-runtime-closure-trace` — an integration test that drives a worker through a real action, asserts the post-action skill-propose hook fires, and verifies the candidate row appears + (if approved) marketplace row appears. ~250 LOC.

### P1-4 — CLAUDE.md contains stale "v1 SHIP-BLOCKER" text

CLAUDE.md line 112: "Skill-library worker load is the v1 SHIP-BLOCKER." This was correct on 2026-05-14 when authored. It shipped via #1682 (claude-skill-library-worker-load-wire) on 2026-05-15. The line in CLAUDE.md still reads as if blocking.

Line 113: "Migration slot collisions are P0 operational risk." Fixed via the duplicate-slot validator (#1735). Still listed as "P0 operational risk."

Line 122: "🔴 SHIP-BLOCKER + P0 migration-slot + AuditRow + parallel-names-round2-high still TODO — all 4 CLAUDE-PREFERRED, held for the fresh session 2026-05-19." This is the 2026-05-15-evening snapshot; the account it references was restored 2026-05-17.

**Impact:** A fresh worker or agent reading CLAUDE.md will see urgency on items that are landed. Wastes time hunting for "blockers" that aren't blocking.

**Needed:** Single commit refreshing CLAUDE.md's "Status" section to current state. Drop the stale SHIP-BLOCKER + migration-slot bullets (move to a "Historical milestones" section if you want the chronicle). Update "Status 2026-05-15 evening" to "Status 2026-05-17" with current numbers.

### P1-5 — Architecture diagrams claim chain shape the code doesn't emit

[`docs/skycot/architecture/21-audit-chain.md`](../architecture/21-audit-chain.md) shows `ApprovalAsked` → `ApprovalReplied` as documented audit variants in the diagram. Per P0-2 above, neither is in `AuditKind`.

**Impact:** The architecture documentation contradicts the runtime. Anyone (operator, auditor, customer) reading the diagrams gets a wrong picture.

**Needed:** Either (a) fix the code (P0-2 — add the variants), then no doc change needed; or (b) update the diagram to show only the variants actually emitted. (a) is better.

## P2 findings

### P2-1 — README.md snapshot date stale (2026-05-07; should be 2026-05-17)

`README.md` line 7 references "snapshot 2026-05-07" with PR count "930+". Today is 2026-05-17 with ~50 more PRs merged in the last 24h. Should be ~980+.

**Needed:** Trivial bump in next commit that touches README.

### P2-2 — CLI reference is incomplete (3 commands listed)

`docs/skycot/cli-reference.md` lists `argo config show`, `argo doctor`, `argo replay export`. Actual CLI surface has 20+ subcommands (skill, vertical, workspace, audit, billing, gateway, completions, etc.). No CI gate verifies reference matches binary output.

**Needed:** A CI script that runs `argo --help` + `argo X --help` for each subcommand and asserts the reference doc lists them. Brief `codex-cli-reference-completeness-gate` — ~120 LOC, fast-mode. (Note: `cli-reference-drift.yml` exists; may already gate this. Verify.)

### P2-3 — Architecture diagrams 1 day stale (acceptable)

Snapshot SHA `ed1ac928` (2026-05-16); 241 commits since. Within tolerance per the diagrams' own update protocol. Worth a routine restamp when next architectural change lands.

### P2-4 — Conductor mutation-pattern caveat

Sub-agent flagged that 11 of last 20 commits authored by `mark-hallam` are feature/refactor code, not ledger-only. Worth a calibration note: those are **squash-merge commits** from `gh pr merge --squash --admin`, where the merger becomes the commit author. The underlying PR work was authored by workers; the squash-merge attribution is mechanical, not a protocol violation.

**However**, the broader concern stands: all 50 last PRs admin-merged by one account. Reviews are 1-of-1 (the admin = the merger). No second pair of eyes.

**Needed:** Either (a) accept this as v1 cost of velocity (operator made this call already), or (b) standup a second admin user for review-then-merge separation. Operator decision; not a code change.

### P2-5 — Velocity risk

50 PRs/day. 4-hour brief→merge cycle. 0 detected regressions in last 50 PRs. The pace itself is the risk class — latent bugs in migration logic, rate-limit edge cases, gateway routing changes may not surface until production load (the kind of thing the operational benches `codex-gateway-load-soak-baseline` etc. would catch).

The deployed-real E2E lane (#1810) is the first guardrail against latent bugs; it just shipped. Until it's been run against a real Fly deploy a few times, the safety net isn't proven.

## Validation: prior audit closure status

Cross-checking the 2026-05-17 codex audit's 13 findings:

| # | Finding | Status now |
|---|---|---|
| 1a | parallel-pub-types | ✅ CLOSED (#1805 + canonicalize wave) |
| 1b | loc-budget | ✅ CLOSED (#1808 split 7 files) |
| 2 | v1 mission proof | ⏸ Pending operator recording tonight + #1796 land |
| 3 | Public signup security | ⏸ Pending #1796 magic-link merge |
| 4 | Pricing-tier enforcement | 🟢 NEWLY CLAIMABLE (post #1801 boot-split) — worker building |
| 5 | send_message wired | ✅ CLOSED (#1807 dispatcher MERGED) |
| 6 | Code-executor fail-close | ⏸ Pending #1799 F-12 recovery |
| 7 | Skill-library selection | ✅ CLOSED (#1740 F-11) — codex audit's own claim was based on stale grep |
| 8 | MCP per-tenant rate-limit | ✅ CLOSED (#1806) |
| 9 | AuditSink lint | ✅ CLOSED (#1809) |
| 10 | env-reads warning-only | 🟡 Intentional ratchet (unchanged) |
| 11 | Mock-fallback E2E | ✅ CLOSED (#1810 deployed-real lane) |
| 12 | Disabled workflows | ✅ CLOSED (`DISABLED_WORKFLOWS_POLICY.md` shipped) |
| 13 | Legacy migration suppressions | 🔵 Low priority (unchanged) |

**8 of 13 codex-audit findings now CLOSED on main.** 3 pending in-flight PRs (#1796 magic-link, #1799 F-12, #1801-cascade pricing-tier). 2 intentional/low-pri. 1 operator-action (demo recording).

## Recommended sequence

**Tonight:**
1. Operator: either run `scripts/demo-dry-run.sh` and accept ~6/8 reds OR wait for #1796 to land before recording. My recommendation: **wait for #1796**. The diff is small; if a fresh-Mac terminal has time to rebase it, ~30 min of work. The script self-diagnoses.

**Tomorrow:**
2. Author + ship `codex-audit-kind-add-asked-replied` (P0-2) — ~80 LOC. Closes the audit-trail honesty gap.
3. Refresh CLAUDE.md (P1-4) — drop stale SHIP-BLOCKER text, bump status snapshot to 2026-05-17.
4. Bump README.md snapshot date (P2-1).

**This week:**
5. Author `codex-split-files-450-500-wave` (P1-1) — keeps the LOC headroom band wide.
6. Author `mission-brand-execution-polish-pass` (P1-2) — closes the 3 brand drifts.
7. Author `mission-claim-4-runtime-closure-trace` (P1-3) — proves the skills-learned loop.

**Pre-launch:**
8. Decide on SQLCipher feature (P0-3): either ship `--features sqlcipher` as default OR document plaintext-at-rest as the v1 limitation.
9. Run the deployed-real E2E lane (#1810) against an actual Fly deploy a few times — that's the velocity-risk guardrail.

## Bottom line

The codebase is in materially better shape than I expected going into this audit. Static gates are GREEN. 5 of 6 mission claims are runtime-PROVEN. Brand foundation is tight. Operator runbooks are real (11 of them) and reference real tools. Swarm throughput is excellent.

The honest gaps are: (1) tonight's demo can't record cleanly until #1796 lands, (2) audit-chain has a documented-vs-actual mismatch that needs an 80-LOC fix, (3) at-rest encryption is plaintext by default, (4) some minor brand drift, (5) a few stale docs.

None of this is architecture dead-end. None of it requires walking back recent decisions. Each item maps to a specific brief or operator action. The launch-readiness picture from the codex audit + this audit combined: **9 of 13 audit findings closed, 3 more pending in-flight PRs landing, 1 operator-recording action**. The asymptote is launch-ready by tomorrow if #1796 lands tonight.
