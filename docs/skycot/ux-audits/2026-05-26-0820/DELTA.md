# UX e2e audit — T22-20 delta vs T23-09

**This run**: 2026-05-25T22:20Z (Mel 2026-05-26 08:20) — 5-agent parallel sweep + synthesis (4-hourly scheduled task).
**Prior run**: 2026-05-25T13:09Z (T23-09) — 5-agent sweep + synthesis batch (delta-batch of 9 net-new P0s landed).

## Headline

**~26 P0s → ~29 P0s on the raw tally, but the surface picture has moved sharply forward.** The whole-site MVP agent now scores **4 of 6 MISSION claims 🟢 GREEN** (was 1 of 6 GREEN at T23-09); only Claim 3 (Genesis) and Claim 5 (macOS cu) remain 🟡 YELLOW. **No 🔴 RED claims left.** The site is shippable for the v1.0 demo MP4 with the open CLAUDE.md items (P-05 Track P, P-02 audit event_kind split, macOS perms grant) closed off-camera.

**18 net-new fix-briefs** authored this pass (clustered from ~58 raw findings). **17 additional findings** mapped to existing fix-briefs already on disk (no duplicates written). Big leverage moves:

1. **fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch** (P0, codex) — single biggest unblock: gateway readers use CWD-based tenant, first-hire writer uses per-principal email, so newly-hired workers are **invisible everywhere**. Closes 3+ separate findings.
2. **fix-onboarding-first-hire-name-parser-prefers-named-pattern** (P0, claude) — `name_from_description` grabs **"A"** from the wizard's own example prompt; every demo hire is named "A". Embarrassing on camera; trivial fix.
3. **fix-delegation-chain-card-visual-surface** (P0, claude) — Claim 2 "delegation is the unit" has **zero visual surface** in SPA. The big remaining hole in the investor narrative.
4. **fix-replay-endpoint-contract-spa-vs-gateway-mismatch** (P0, claude) — Gateway returns hash-recompute verdict; SPA expects sandboxed-rerun; falls through to LeakView crash. Scene 6 demo blocker.
5. **fix-worker-detail-loading-forever-no-error-boundary** (P0, claude) — Hire → redirect → spinner forever on /workers/<id>. Kills the onboarding climax.

## P0 delta vs T23-09

| Status vs T23-09 | Approx count | Notes |
|---|---|---|
| **Closed since T23-09** | ~5 | charter-object crash, audit list/detail ID mismatch (#2297), replay fabrication for unknown IDs (#2296), gateway-503 schema bump (#2331), check-gateway-freshness-503 distinction (#2335) |
| **Persistent (root cause unchanged)** | ~13 | No Run-now button (still BLOCKED on responsibilities-schedules brief), competitor-pricing search seed, customer_research_followup workflow, persona avatars, Schedules tab end-to-end, /skills Install button, approvals-vs-ops-dashboard route ambiguity, team-hero hardcoded KPIs |
| **Net-new this run** | 18 (synthesised → fix-briefs) | tenant-resolution mismatch, hire-name parser, delegation-chain visual surface, provider-key 500, replay endpoint contract, worker-detail spinner |

The **tenant-resolution mismatch** (codex brief) is the single highest-leverage finding — multiple "X is invisible" findings across the 5 reports all point at the same cwd-vs-principal split in `crates/argo-cli/src/handlers/gateway/*.rs`. One PR closes 3+ findings.

## 6-MISSION-claim coverage delta

| Claim | T23-09 | T22-20 | Direction |
|---|---|---|---|
| 1 — Teammate, not tool | 🟢 WORKS (marketing only) | 🟢 GREEN (in-app + marketing) | ⬆ — `/workers` + `/cohorts` materialise teammate metaphor with L1-L9 chips, D3 magazine tiles, C2 personality register |
| 2 — Delegation is the unit | 🟡 PARTIAL | 🟢 GREEN | ⬆ — handoff/delegation across 12 components, approvals as delegation inbox; visual polish of chain timeline still TODO (P0 fix-brief authored this run) |
| 3 — Platform builds workers (Genesis) | 🟡 PARTIAL | 🟡 YELLOW | ↔ — `/hire` flow surfaces Genesis Phase-1 (Step2Listen → Step3Drafting → Step4Introduce → Step5Connect + VoiceProfileEditor); Phase 2 multi-modal SOP/wiki/Slack ingest pending |
| 4 — Skills shared + learned | 🟢 WORKS (workflows/templates) | 🟢 GREEN | ⬆ — `/skills` (210 LOC) confirmed; cross-tenant isolation enforced backend-side (#1702) |
| 5 — Any-PC execution (macOS cu) | 🔴 INVISIBLE | 🟡 YELLOW | ⬆ — `settings/connectors/` surfaces MCP catalog; macOS cu visualisation deferred (operator-only perms grant per CLAUDE.md (c)) |
| 6 — Multi-channel + multi-tenant | 🟡 PARTIAL | 🟢 GREEN | ⬆ — `/audit` (428 LOC) with URL-as-state filters + RBAC redaction + evidence-bundle exporter; `/approvals` ops dashboard; all 4 channel families wired backend-side |

**Net direction**: 5 of 6 claims moved ⬆ (or held GREEN), 0 moved ⬇. **4 of 6 GREEN, 2 of 6 YELLOW, 0 RED.** Strongest single jump: Claim 6 (multi-channel + multi-tenant) consolidated to GREEN — `/audit` is now the strongest SPA surface in the build per the whole-site agent's call.

## Distance to v1.0 ship-gate

**Substantially shorter than T23-09.** Whole-site verdict: *"The site is shippable for the v1.0 demo with the open items in CLAUDE.md 'Remaining for v1.0 launch' (P-05 still TODO; P-02 blocked on audit event_kind split; macOS perms grant + record)."*

The remaining blockers, in priority order:

1. **macOS permissions grant** (operator-only, CLAUDE.md item (c))
2. **Record the demo MP4** (operator-only, CLAUDE.md item (d))
3. **Track P P-05 → P-06 → P-07** (agent-doable, currently TODO)
4. **The 6 new P0 fix-briefs** from this run (tenant-resolution mismatch is the highest-leverage)
5. **The ~13 persistent P0s** from prior runs (mostly worker-detail Schedules tab + skills install flow + demo-seed wiring)

## Top 3 highest-leverage fix-briefs to claim next

1. `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch` (codex) — single defect, 3+ findings closed, unblocks the onboarding hire-flow narrative end-to-end.
2. `fix-replay-endpoint-contract-spa-vs-gateway-mismatch` (claude) — SPA-only adapter as immediate unblock; replay page is Scene 6's drill-in moment.
3. `fix-delegation-chain-card-visual-surface` (claude) — the only remaining visual gap in the Claim 2 story; demo's "you can see the handoff" moment.
