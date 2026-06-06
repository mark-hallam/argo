# SKYCOT Decision Register

Append-only log of **operator decisions** — the scoped commitments, scope
changes, and process authorizations Mark Hallam makes that aren't
derivable from the code or git history. This is the **source of truth**
for "what did we decide, when, why, and what did it supersede."

## Why this exists

The agent swarm's context resets between sessions. Operator decisions
made in chat (e.g. "Pitch Agent is now MVP scope", "newest-wins on
competing PRs", "Sonnet floor, no Haiku") get scattered across CLAUDE.md,
auto-memory, ledger notes, and chat history — and some get lost (the
Pitch/Genesis MVP scope-creep was never documented in the phase plan,
which misaligned the timeline). This register fixes three things:

1. **Source of truth** — one queryable place for every locked decision.
2. **Conflict-flagging** — the conductor reads this index at session
   start (like CLAUDE.md). When a new request contradicts a locked
   decision, the conductor STOPS and surfaces it:
   *"This conflicts with DEC-014 (you locked X on <date> because Y) —
   confirm you want to override?"*
3. **Timeline integrity** — scope changes are captured at decision-time,
   not painfully reconstructed later.

## How it works (ADR pattern)

- One file per decision: `DEC-NNN-short-slug.md`.
- This README is the **index** — one line per decision, newest first.
- Decisions are **append-only**. To change a decision, write a NEW one
  that `Supersedes:` the old; mark the old `Status: superseded-by DEC-MMM`.
- Never delete a decision file. Reversal is itself a decision.

## Decision lifecycle

`active` → (`superseded-by DEC-NNN` | `reversed-by DEC-NNN`)

## Conductor protocol

- Load this index at session start.
- Before acting on any operator request that touches scope, process,
  routing, or a previously-locked area: grep the register. If the
  request conflicts with an `active` decision, surface the conflict and
  get explicit override confirmation before proceeding.
- When the operator makes a new locked call, write the ADR immediately
  (same turn) + add the index line.

## When this graduates to a database

Stays as files until the file approach hurts (querying, cross-referencing,
or a product surface needs it). At that point it backs onto the existing
`argo-store` SQLite schema (additive migration, tenant-scoped, optionally
hash-chained via the audit primitive) — a `decisions` table that doubles
as a product feature ("your AI workforce's decision log"). Not before.

---

## Index (newest first)

| ID | Date | Decision | Status |
|----|------|----------|--------|
| [DEC-041](DEC-041-execution-posture-risk-tiered-act-by-default.md) | 2026-06-06 | **Execution posture: risk-tiered act-by-default** (operator chose "B") — evolve [DEC-025](DEC-025-genesis-done-right-full-synthesis.md) #6 from uniform draft-only to tiered: **low-risk + non-approval actions auto-execute** within a tight reversible/audited envelope; **sensitive/irreversible/public verbs stay approval-gated** (DEC-028 red line preserved). Operationalizes DEC-036 (risk-tiered gate default) + DEC-037 (auto-graduate the safe tier). **Built default-OFF + demoable; the live flip stays operator-only after the blast-radius review is re-run against the wired path.** | active |
| [DEC-040](DEC-040-next-batch-authorization.md) | 2026-06-06 | **Next-batch authorization** — Phase B (computer-use real execution: B1/B2/B3) **BUILD default-OFF** (live flip stays operator-only, adversarial review per PR); Phase C **focused subset** (doc-upload + comprehension-quiz); demo-tenant fixes **authorized to brief** (operator reviews PRs). Authorizes the build of DEC-036/037; live-enable remains a future operator call. | active |
| [DEC-039](DEC-039-genesis-conversational-mode-additive.md) | 2026-06-05 | **Genesis conversational interview is ADDITIVE** — the "Talk it through" multi-turn mode ships alongside (not replacing) the one-shot "describe and watch" flow; both feed the same materialization. Resolves the A3 open design question (lower-risk; preserves the merged/imminent genesis Phase-2 panels). | active |
| [DEC-038](DEC-038-per-teammate-cost-attribution.md) | 2026-06-05 | **Per-teammate cost attribution** — add nullable `worker_id` to `session` + `spend_ledger` (additive) + thread `SpendCtx`; new tenant-scoped `/costs/by-worker` + SPA cost surface; reconcile the stale `claude-session-worker-id-schema` brief. Verified 2026-06-05 gap: spend is tenant/session-only + invisible in the app. Phase A of the usefulness plan. | active |
| [DEC-037](DEC-037-graduated-real-execution.md) | 2026-06-05 | **Graduated real execution** — a PROVEN teammate graduates draft→real execution (MCP/computer-use), per-action, post-probation, approval retained on sensitive verbs. **Operationalizes [DEC-025](DEC-025-genesis-done-right-full-synthesis.md) #6's "deliberate post-hire grant"; does NOT reverse draft-only-at-birth.** Phase B of the usefulness plan. | active |
| [DEC-036](DEC-036-autonomous-scheduled-execution.md) | 2026-06-05 | **Autonomous scheduled execution** — replace the blanket `ClosedGate` deny-all with the merged `auto_approval::evaluate` (`H-auto-approval`) wired as a `DutyGate`; bridge hire→`role_duty`; default-OFF `ARGO_DUTY_AUTOFIRE`, go-live after Phase A. Verified 2026-06-05 gap: hired teammates never fire on schedule. Phase B of the usefulness plan. | active |
| [DEC-035](DEC-035-pro-price-49-and-single-frontend-app.md) | 2026-06-04 | **Pro price = $49/mo (raise product to match marketing) + ONE front-end app at both domains (delete `marketing/`; `web/` serves skycot.com + app.skycot.com).** Canonical `v1-tiers.toml` pro=49 (#2702 merged); consolidation #2703 (107 files, in review). Operator actions remaining: create the $49 Stripe price + wire `price_id` (display ≠ charge); attach skycot.com to web/ Vercel + DNS. Evidence: `audit/2026-06-04-marketing-vs-web-app-duplication.md`. | active |
| [DEC-034](DEC-034-paying-ready-sprint-and-conductor-driven-run-mode.md) | 2026-06-04 | **Paying-ready 1-week sprint + conductor-driven run-mode (no cron).** POC → fully-public self-serve SaaS + enterprise tier (6 workstreams; wedge = the platform loop). "I drive it (no cron)": `skycot-conductor-20min`/`skycot-evaluator-hourly` STAY DISABLED; swarm builds 24/7, conductor (interactive `/loop`) drives merges + replenish + ledger-truth + twice-daily live signup gate + adversarial review on the 3 risky landings (tenant/billing/perimeter). Disciplines: sprint-only TODO queue, close off-sprint PRs, drain PR backlogs (self or spawned agents), verify briefs vs current main, ledgers in-band. Supersedes DEC-033 (paid prototype dropped). | active |
| [DEC-033](DEC-033-overnight-build-toward-funding-meeting-thursday.md) | 2026-06-01 | **Overnight autonomous build toward Thursday's funding meeting** (per [DEC-031](DEC-031-overnight-autonomous-frontier-b-buildout.md)/[DEC-026](DEC-026-overnight-autonomous-genesis-completion.md) pattern, [DEC-029](DEC-029-truth-bounded-status-reporting.md) truth-bound). Operator (remote, going to bed): *"queue up work from the next phases … a lot done tonight … then lots of testing in the browser."* Split: **build everything non-browser tonight** (conductor-driven, ONE rustc build at a time, NOT the swarm — controlled + verified), **browser-testing is its own phase** for when Mark's back. Queue: `docs/skycot/overnight-build-queue-2026-06-01.md` — Cora's voice backend live, consolidate the voice stack, Cora's frontend, pitch materials, quality hardening (full cargo+vitest+tsc), deploy-kit finalize, push the validated stack (+FEATURE_MATRIX rows). Browser phase: mic fix → streaming-voice smoke → hire Cora on /genesis → talk to Cora → founder-story voice interview → demo walkthrough → AWS deploy. Morning report honest DONE/PARTIAL/BLOCKED. | superseded by DEC-034 (investor path dropped) |
| [DEC-032](DEC-032-deployment-readiness-phase.md) | 2026-06-01 | **Next phase entered: Deployment Readiness** — operator directed the program beyond the MVP demo recording into making SKYCOT a **gated, pointable, AWS-deployable product** ("something to point people at" on the AWS credits). The *launch* track, parallel to the [DEC-030](DEC-030-post-demo-frontier-order.md) feature frontiers; operator does AWS account/DNS, conductor preps the kit. **Architecture:** gateway on LOOPBACK behind nginx (TLS + basic-auth gate) so the seeded demo stays viewable (a non-loopback bind would force prod bearer-token auth per `routes_auth_production_mode.rs`); container bundles the gateway's on-disk runtime deps (corpus/policies/manifests via `ARGO_*_DIR`); prod SPA shows the demo via `NEXT_PUBLIC_SKYCOT_DEMO_MODE/_LIVE` (auth-bypass is off in prod builds). Scale-later: App Runner/ECS/Amplify/RDS + real auth. **Verified so far:** SPA prod-build blocker FIXED (deleted off-brand oklch `/icon` generators; `next build` green + standalone; `72d8df0`); deploy kit (`docs/skycot/operator/DEPLOYMENT.md` + `deploy/`) authored (`80c04f0`). Truth-bounded ([DEC-029](DEC-029-truth-bounded-status-reporting.md)): the deploy itself is "prepped, verification pending the operator's instance." | active |
| [DEC-031](DEC-031-overnight-autonomous-frontier-b-buildout.md) | 2026-06-01 | **Overnight autonomous frontier-B ("everything real") build-out authorized + executed** (per [DEC-030](DEC-030-post-demo-frontier-order.md) order, [DEC-026](DEC-026-overnight-autonomous-genesis-completion.md) pattern, [DEC-029](DEC-029-truth-bounded-status-reporting.md) truth-bound). Operator put the conductor in charge of building + testing frontier B overnight ("keep pushing… built + tested when I wake up"; the "2,500×" framing = be ambitious, NOT inflate the report). Held: truth-bounded re-verify of every load-bearing claim; build-in-worktree → conductor-deploy → live-verify per piece; ONE Rust build at a time (near-OOM RAM); NO THEATER (omit / empty-state where no real data, never fabricate). OUTCOME (live on `043f5ae` / schema 87, conductor-verified): Tier-1 data, 4 backend-route gaps, api_key store (v86), C1 real telemetry (killed the "184 runs" mock), worker-memory (v87), seed-prune; bell-crash + enum-map crash class + false-banner + approvals-layout all fixed. Demo 🟢 green-to-record, clean-8. Caveats: seed-prune doesn't fire on the stale-cache binary (workaround documented); ~60 stale worktrees. Frontier C scoped + ready (C1 done; C2/C3 queued). | active |
| [DEC-030](DEC-030-post-demo-frontier-order.md) | 2026-05-31 | **Post-demo build order is locked: A → B → C → D** (work it top-down, [DEC-023](DEC-023-continuous-autonomous-priority-rank-order.md) discipline). The v1.0 demo is essentially recordable and the three-lane push is complete (Genesis P0–P7 [DEC-025](DEC-025-genesis-done-right-full-synthesis.md), the 3 hero teammates, the eval + self-improvement loop [DEC-017](DEC-017-teammate-eval-harness-synthetic-golden-companies-first.md), frontend de-theatre + locked-design parity [DEC-007](DEC-007-no-theater-in-demo.md)/[DEC-027](DEC-027-brand-and-design-system-locked.md)). **A. Voice (ACTIVE FRONTIER)** — live voice for teammates on the [DEC-021](DEC-021-voice-vendor-openai-for-launch-selectable.md) OpenAI discrete STT+TTS path; thin first slice = "talk to Portia by voice"; the biggest differentiator/wow. **B. Everything real** — build the backend endpoints the UI marks honestly-unavailable (`/api/v1/api-keys`, `/api/v1/providers/keys`, `/api/doctor/channels`) + close remaining honest-gaps so the whole product is real, not just the demo path. **C. Marketplace depth** — more real hero teammates + richer hire-from-catalog. **D. Enterprise governance** — SSO/SCIM/BYOK/data-residency (SSO crypto chain can't build on Windows). Relates to DEC-021, DEC-010/025, DEC-007, DEC-029. | active |
| [DEC-029](DEC-029-truth-bounded-status-reporting.md) | 2026-05-31 | The conductor's **own status reporting is truth-bounded** — NO THEATER ([DEC-007](DEC-007-no-theater-in-demo.md)) applies to status reports, not only product agents. NEVER report a background process / autonomous mechanism (buildout tick, worker swarm, scheduled tasks) / build / deploy / agent as "running"/"doing X"/"carrying"/"in progress" **without current-turn evidence** (log line, `launchctl`/`ps`, fresh commit SHA, health-endpoint response). **Distinguish VERIFIED from SET-UP/EXPECTED/UNCONFIRMED** — arming a job is NOT evidence it's producing work. "Done"/"shipped"/"live" needs verification (SHA / passing-test tail / live endpoint), not an agent's say-so. Say "set up but unverified" plainly when you can't confirm. **Highest-priority fix from an operator trust breach** (conductor reported the idle buildout tick as "carrying the work"). Build-agent prompts inherit this; conductor re-verifies load-bearing claims rather than relaying them. | active |
| [DEC-028](DEC-028-marketing-teammate-third-hero.md) | 2026-05-31 | Marketing teammate is the **3rd hero teammate** (alongside Portia=pitch, Grant=GTM/inbound) — owns top-of-funnel marketing + social (articles, social listening, drafting public responses, thought-leadership), **built THROUGH Genesis** (doubles as live proof Genesis works) in Mark's documented voice. **MVP discipline: draft-only + approval-gated + listening-first — NO autonomous posting** (public action = brand/legal risk; truth-boundary hard-bound; ties to DEC-025). Boundary: Grant qualifies inbound, Marketing fills the funnel. **Fast-follow after the Genesis safety stack P0–P6.** | active |
| [DEC-027](DEC-027-brand-and-design-system-locked.md) | 2026-05-31 | Brand + per-page design directions are LOCKED (gradient purple→PINK `#8B5CF6→#EC4899`; purple→cyan DEPRECATED; Audiowide/Inter/JetBrains Mono). The optometrist / from-scratch design method does NOT apply to already-locked pages — consume the tokens + locked direction; explore only genuinely new surfaces. Entry point: `docs/skycot/brand/DESIGN_SYSTEM.md` | active |
| [DEC-026](DEC-026-overnight-autonomous-genesis-completion.md) | 2026-05-31 | Overnight autonomous Genesis-completion run authorized — P1 (materializer) → P3–P7 via the DEC-020 auto-resume tick, "never stop till launch-ready"; each phase implement → adversarial bypass-hunt → deploy → live-verify; NO THEATER | active |
| [DEC-025](DEC-025-genesis-done-right-full-synthesis.md) | 2026-05-30 | Genesis done right — full from-scratch (archetype-free) teammate synthesis is the committed core build, to a production-quality bar (teammates do real work + can't ruin a business); design-first via workflow → operator alignment → build; ultracode effort. LLM-archetype-selection (3f8a6f3) is a stepping stone, not the destination | active |
| [DEC-024](DEC-024-build-agents-opus-no-downtiering.md) | 2026-05-30 | Conductor build-agents run on Opus 4.8 — no task-based down-tiering to Sonnet (quality/drift-avoidance over speed/cost); distinct from DEC-013/014 product routing | active |
| [DEC-023](DEC-023-continuous-autonomous-priority-rank-order.md) | 2026-05-30 | Continuous autonomous execution through a fixed P1–P5 priority order (de-theatre → voice+Genesis wow → real delegation → self-improvement → enterprise governance); operating model locked = conductor + on-demand build-agents + auto-resume, swarm-daemon retired for now (auditor/evaluator on-demand) | active |
| [DEC-022](DEC-022-sprint-2-scope-full-program.md) | 2026-05-30 | Sprint-2 scope = full program: all four tracks (go-live+verify / kill-all-theatre / multi-worker orchestration [Supervisor] / broad backlog) + real voice | active |
| [DEC-021](DEC-021-voice-vendor-openai-for-launch-selectable.md) | 2026-05-30 | Voice vendor = OpenAI for launch (discrete STT `/v1/audio/transcriptions` + TTS `/v1/audio/speech`); vendor-selectable (`ARGO_VOICE_PROVIDER`) + user-selectable (`voice` workspace_settings); Realtime socket = later upgrade | active |
| [DEC-020](DEC-020-autonomous-overnight-build-out-auto-resume.md) | 2026-05-29 | Autonomous overnight build-out + auto-resume authorized: conductor + self-committing build-agents + launchd `com.skycot.buildout-resume` tick; off-switch `~/.argo/buildout-stop` | active |
| [DEC-019](DEC-019-beyond-the-demo-build-out-whole-platform.md) | 2026-05-29 | Beyond the demo — build out the whole platform, remove ALL theatre, drive to the mission; supersedes demo-prep as the active program | active |
| [DEC-018](DEC-018-gtm-agent-grant-second-hero-teammate.md) | 2026-05-29 | GTM teammate "Grant" is the 2nd real working teammate; owns inbound growth as a responsibility; `/gtm` console + `POST /api/v1/gtm/draft-followup`, truth-bounded, approval-gated | active |
| [DEC-017](DEC-017-teammate-eval-harness-synthetic-golden-companies-first.md) | 2026-05-29 | Teammate-eval harness: synthetic golden-companies first, historical replay a later seam; Portia=reference, Mariah=first synthetic | active |
| [DEC-016](DEC-016-pitch-agent-persona-portia-female.md) | 2026-05-29 | Pitch Agent persona is "Portia" (female); P-name convention | active |
| [DEC-015](DEC-015-decision-register-adr-files-first.md) | 2026-05-29 | Decision register ships as ADR markdown files first; DB only if it earns it | active |
| [DEC-014](DEC-014-model-catalog-opus-4-8-sonnet-4-6.md) | 2026-05-29 | Model catalog = Opus 4.8 (frontier, 1M) / Sonnet 4.6 (1M) / Haiku 4.5; Opus 4.8 is the fixed-policy default | active |
| [DEC-013](DEC-013-sonnet-floor-no-haiku.md) | 2026-05-29 | Tier step-down floor is Sonnet 4.6 — never Haiku; Opus 4.8 for complex | active |
| [DEC-012](DEC-012-claude-code-cli-dev-demo-routing.md) | 2026-05-29 | Dev/demo LLM routes through Claude Code subscription (claude-code-cli provider) for cost-control; production flips to anthropic API | active |
| [DEC-011](DEC-011-newest-wins-competing-prs.md) | 2026-05-28 | Competing PRs on the same surface: newest-wins, close older | active |
| [DEC-010](DEC-010-genesis-mvp-must-have.md) | 2026-05-28 | Genesis (AI-teammate onboarding studio) is MVP-must-have, not v1.x-deferred | active |
| [DEC-009](DEC-009-computer-use-mvp-must-have.md) | 2026-05-28 | Computer-use is MVP-must-have (was already macOS-first per DEC-002) | active |
| [DEC-008](DEC-008-pitch-agent-in-mvp-scope.md) | 2026-05-24 | Pitch Agent is IN v1.0 MVP scope (originally beyond-v1.0) | active |
| [DEC-007](DEC-007-no-theater-in-demo.md) | 2026-05-24 | NO THEATER in the v1.0 demo MP4 — real backend, no mock data at runtime | active |
| [DEC-006](DEC-006-web-test-doc-merge-lane.md) | 2026-05-26 | Web-only ≤200 LOC ≤5 files / test-only / doc-only PRs may admin-merge without evaluator pre-verification | active |
| [DEC-005](DEC-005-skip-clippy-evaluator-preverified.md) | 2026-05-24 | --skip-clippy authorized on conductor-pre-merge-gate for evaluator-pre-verified PRs | active |
| [DEC-004](DEC-004-vocabulary-schedule-not-duty.md) | 2026-05-23 | User-facing vocab: "Schedule" not "Duty"; code/SQL keep "duty" | active |
| [DEC-003](DEC-003-melbourne-not-sydney.md) | 2026-05-18 | SKYCOT is built in Melbourne, not Sydney — brand/marketing copy | active |
| [DEC-002](DEC-002-macos-first-computer-use.md) | 2026-05-14 | macOS is the v1 computer-use path; Linux/Windows/E2B deferred-v1.x | active |
| [DEC-001](DEC-001-enterprise-not-consumer.md) | 2026-05-14 | Enterprise focus, not consumer — connector priority + scope decisions | active |

_DEC-001..DEC-018 were backfilled 2026-05-29 from CLAUDE.md, auto-memory, and
session history (a reconstruction of decisions already made). DEC-019 onward are
captured at decision-time, as intended._
