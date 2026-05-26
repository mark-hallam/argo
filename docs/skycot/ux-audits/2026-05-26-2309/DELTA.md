# UX e2e audit — T2309MEL delta vs prior

**Run**: 2026-05-26T13:09Z (23:09 Melbourne)
**Prior**: 2026-05-26T12:14Z (22:14 Melbourne) — 55 min earlier
**5 audit reports**: onboarding hire-flow, scenes 1+2, scenes 4+5, scene 6+approvals, whole-site MVP coverage
**Synthesis**: 16 net-new root-cause clusters authored (out of ~68 findings); ~52 findings deduped against the 238 fix-* briefs already on origin/main
**Ledger insertion**: clean — header `▼ ▼ ▼ UX-AUDIT-T2309MEL BATCH (16 briefs)` inserted at top of DEMO-RECORDING UNBLOCK section, 3 P0 + 7 P1 + 4 P2 + 2 P3 rows queued

## P0 punch-list — this run

| # | brief | source | demo-camera-sees? |
| - | - | - | - |
| 1 | fix-genesis-orphan-discoverability-and-provider-key-503 | onboarding F1 | YES — Genesis Studio (THE investor moment per operator memory) is unreachable from any nav/CTA; POST /api/v1/genesis 503s |
| 2 | fix-hire-demo-mode-stub-uuid-worker-roster-invisibility | onboarding F3 | YES — Scene 1 hire wizard mints `stub-<uuid>` worker that never appears in /team; detail page title is the UUID, personality card 404s |
| 3 | fix-workflows-dark-mode-card-white-on-white | whole-site F4/F22/F23/F25 | YES if dark-mode is recorded — /workflows + templates + detail-page cards have hardcoded white bg, text unreadable |

## Delta vs prior (T2214MEL)

### Newly opened P0s (this run found, prior did not)

- **F1 onboarding (Genesis orphan + 503)** — prior runs flagged Genesis surface incompleteness via `fix-onboarding-welcome-step-genesis-studio-framing` but did NOT name that the studio surface itself is unlinked AND the POST endpoint returns 503. The investor moment (per memory `feedback_genesis_is_ai_teammate_onboarding_studio`) is currently invisible.
- **F3 onboarding (stub-uuid worker invisibility)** — prior runs flagged auth-bypass theatre via `fix-hire-wizard-no-account-context-bypasses-signup` etc. but did NOT name that the hireWorker() implementation in demo-mode returns `stub-<uuid>`. Scene 1 of the runbook will record a hire that doesn't show up in /team.
- **F4/F22/F23/F25 whole-site (workflows dark-mode unreadable)** — prior run shipped `fix-dark-mode-tokens-pricing-compare-docs-marketplace` covering marketing pages; this run finds the same pattern on the in-app workflow surfaces which were NOT covered by that brief.

### Persistent P0s (still open from prior run, NOT redundantly briefed)

- **fix-zombie-pnpm-dev-port-3000-from-stale-worktree** (T2214) — meta-fix for orphan dev process
- **fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance** (T2214) — audit drill-in shows `placehold00000000`-shaped IDs
- **fix-activity-row-audit-detail-synthesized-fixture-theatre** (T2214) — Scene 1/2 fabricated 8-stage chain
- **fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona** (T2214) — Scene 2 workflows + Scene 4/5 Run-now + audit feed seed bundle
- **fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect** (T2152) — still TODO
- **fix-auth-demo-mint-route-returns-gateway-unreachable** (T2152) — still TODO
- **fix-playwright-hire-flow-regression-cluster** (T2007) — still IN_REVIEW (#2408)

### Closed since prior run

55 minutes elapsed since T2214. Recent commits since: `b9deef872` (worker-supervisor watchdog ping), `2912586d4` (T2152MEL heartbeat refresh), `781f35ec3` + `41f5c3643` (status claims). No P0-closing PR merges in the window — closure delta effectively zero.

## 6-MISSION-claim coverage delta (this run vs T2214)

| claim | T2214 verdict | T2309 verdict | shift |
| - | - | - | - |
| 1 — Teammate, not tool | YES, demonstrable | 🟡 SPA mocks: /team falls back to `web/lib/api/team.ts` seed (gateway /api/v1/workers 404s); demo-mode hire-flow mints `stub-<uuid>` invisible in /team | 🔴 **regression in SPA visibility** |
| 2 — Delegation is the unit | NOW DEMONSTRABLE (DelegationChainCard live via #2345) | 🟡 DelegationChainCard renders fixture handoffs; live gateway returns no chain data | stable-with-caveat |
| 3 — Platform builds workers | YES (Genesis SSE live) | 🔴 **regression**: Genesis surface unreachable from nav + POST /api/v1/genesis 503s | 🔴 **−1** |
| 4 — Skills shared + learned | SURFACE LIVE, SEED STILL EMPTY (PR #2407 pending) | 🟡 marketplace UI live, 0 skills in gateway → SPA renders 6 from `lib/api/skills.ts` seed; skill detail has no Install CTA | stable |
| 5 — Any-PC execution | NOT IN SPA SCOPE | NOT IN SPA SCOPE — workflows surface has substrate chips (some show macOS) but no runtime proof | stable |
| 6 — Multi-channel + multi-tenant + trust | NOW DEMONSTRABLE (ChannelStripCard live via #2391) | 🟡 ChannelStripCard renders; /audit drill-in returns "Action detail unavailable — Unknown worker" for the 9 seeded rows | 🔴 **regression on audit credibility** |

**Net shift**: T2214 reported "5 of 6 claims demonstrable as SPA surfaces"; T2309 finds that **without the right env vars + seed data, the SPA renders fixture content that LOOKS demonstrable but breaks on the first audit drill-in or hire-completion check**. Most of this isn't a code regression — it's the same fixture-vs-live-data gap, more carefully measured by this run's deeper backend curl-verification. Orientation score: ~70 (down from 83 in T2214) because this run weights "drill-in / hire-completion / nav-discoverability" failures more heavily than T2214 did.

## Distance to v1.0 ship-gate

**Subjective assessment: ship-recordable today STILL CONDITIONAL on the same three operator-side gates** as T2214 + a new P0 from this run.

1. **Operator action — kill orphan pnpm dev + relaunch with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`** — unchanged from T2214; still highest-leverage 30-second op.
2. **Operator action — grant macOS Accessibility + Screen Recording perms on argo binary** — closes claim-5 runtime gap; unchanged.
3. **Merge `fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona`** + **`fix-hire-demo-mode-stub-uuid-worker-roster-invisibility`** + **`fix-genesis-orphan-discoverability-and-provider-key-503`** — these three together repair the function-side theatre the camera SEES at Scenes 1, 2, 4, 5 and the Genesis intro.

The 16 P-priority briefs queued this run are a mix of P0 demo-blockers (3) and progressive credibility improvements (13). The P0s map to function-side theatre — none are dressing — so each one needs to ship before the final recording attempt.

## Top 3 highest-leverage fix-briefs (this run)

1. **fix-hire-demo-mode-stub-uuid-worker-roster-invisibility** — closes Scene 1's hire-completion proof. The runbook's first scene narrates "watch a new teammate show up in the roster"; current state hires an invisible `stub-<uuid>` whose personality card 404s. Function-side P0. Effort: S.
2. **fix-genesis-orphan-discoverability-and-provider-key-503** — restores the investor-moment surface that distinguishes Skycot from "ChatGPT with prompts" (per memory `feedback_genesis_is_ai_teammate_onboarding_studio`). Two-part: add a nav/CTA entry-point + fix the gateway POST so the wizard's plan→provision call doesn't 503. Effort: M.
3. **fix-workflows-dark-mode-card-white-on-white** — single-css-pass repair across `/workflows + /workflows/templates + /workflows/{id}`. If the demo MP4 records in dark mode (Tom's preferred screen-recording aesthetic per past sessions), the workflows cards are unreadable. Effort: S.
