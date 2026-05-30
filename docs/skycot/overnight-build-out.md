# Autonomous platform build-out — master tracker + resume anchor

## 🌙 OVERNIGHT 2026-05-31 — COMPLETE GENESIS (operator asleep; never stop till launch-ready)

**Directive (operator, 2026-05-31):** "Fire up P1 then P3–P7. While I sleep. Never stop till it's launch ready." → Build out the rest of **Genesis done right** (DEC-025 + `docs/skycot/design/2026-05-30-genesis-done-right.md`), continuous + autonomous, via the auto-resume tick.

**State (live, `build_commit 05f9ea2`):** P0 (eval/safety hire-gate, all 3 seams) + P2 (capability fence, provenance-keyed least-privilege) are DONE + adversarially + live-verified — the two "ruin a business" holes are closed. Build on this; do NOT regress it.

**Remaining phases — IN ORDER:** **P1** materializer (BroadRoleDraft→typed CapabilitySpec→`load_dir`-valid teammate; archetype-free; hybrid per DEC-025 #3) — *IN FLIGHT, agent launched 2026-05-31* → **P3** enforced probation at every dispatch seam (24h `dry_run_until` recorded but no dispatcher reads it yet) → **P4** real eval + role-contract comprehension gate (DEC-025 #4) → **P5** truth-boundary-at-birth + operator "what can this teammate do" review screen (DEC-025 #1) → **P6** the moat (per design: prompt-injection trust boundary FIRST, then doc/SOP upload→knowledge, then connectors/voice intake, then SOP-comprehension quiz) → **P7** deferred (idempotency, abstention, LLM-graded rubric).

**Follow-up queue (validated 2026-05-31 — fold in alongside the phases; same per-phase pattern):**
- **[Genesis parity — do WITH P5, high]** The flagship `/genesis` SSE hire is *weaker* than `/onboarding/first-hire`: it persists no plan-provenance chip and seeds no worker memory (empty Memory tab). Reach parity — set `genesisPlanId: ready.vertical_id` in `web/lib/api/genesis.ts::buildHireDraftFromGenesis`, and seed memory in `routes_workers_hire.rs` mirroring `routes_onboarding_genesis_seed.rs::seed_worker_memory_from_plan` (re-load the `.genesis/` vertical, or extract the shared seeder). + a test mirroring `first_hire_genesis_success_seeds_memory_namespace`.
- **[test reliability — high]** Migrate the ~21 gateway test files from a private `static M: Mutex` to `crate::handlers::gateway::shared_env_lock()` so the full `argo-cli` bin suite stops racing on `ARGO_VERTICALS_DIR` (~24 spurious failures). This makes the per-phase verification trustworthy — do it early.
- **[hygiene — low]** (a) Reword `web/lib/api/skills-seed.ts` `proposal.draft` description so a "pricing" search returns exactly 1 card (fixes the `page.test.tsx` Scene-4 test). (b) Split `routes_retrain_tests.rs` (627) + `routes_retrain_helpers.rs` (529) under the 500-LOC ceiling. (c) Harden the fence: fail-closed on provenance-read error + wildcard-sanitize `WorkerCapabilityProfile::derive` (per the `05f9ea20a` verify).

**⚡ EFFORT LEVEL: ULTRACODE (operator, 2026-05-31).** Build each phase at maximum thoroughness — **token cost is NOT a constraint; correctness + safety are everything.** ORCHESTRATE each phase rather than single-shot it: (design, if non-obvious →) implement → a **fan-out of ≥2–3 INDEPENDENT adversarial verifiers** with different lenses (bypass / regression / spoof / "how does this ruin a customer") → synthesize + fix until clean. Use the Workflow tool for the orchestration where it fits — BUT note the P0 workflow failed because a `schema` agent didn't emit StructuredOutput, so prefer text-return agents (or keep any schema tiny + that agent's sole job). A deep, adversarially-bulletproof phase beats three shallow ones. Genesis is the core; "a generated teammate ruins a customer's business" is the failure mode to design against at every step.

**MANDATORY pattern per phase (how P0 + the fence were made solid — do NOT skip the adversarial step):**
1. **Implement** as ONE cohesive build-agent (plain Agent, NOT a schema-workflow — the schema requirement made the P0 workflow fail; text return is robust). Commit green work even if a phase must be sliced.
2. **Independent adversarial bypass-hunt** — a separate read-only agent that tries to REFUTE the phase's safety claim (it caught a real demo-path hole in BOTH P0 and the fence). Fix any hole, re-verify.
3. **Deploy:** sync local to origin FIRST (`git fetch && git rebase origin/main` — deploy-from-local gotcha), then `bash scripts/restart-live-gateway.sh --force --no-prompt`. Reseed if needed.
4. **Live-verify on :7400** (real curl/turn, not tests-only). Then the phase is done.
5. **Ledger in-band (same commit):** FEATURE_MATRIX row + regenerate HTML (`python3 scripts/render-feature-matrix.py`); a new operator-locked call → a DEC.

**Guardrails:** ≤3 concurrent heavy Rust builds. NEVER grep/echo `~/.argo/env.local` (key-exposure lesson); keep its vars `export`-prefixed. Fast-forward push only. NO THEATER — each gate must REALLY work (adversarially verified); report real DONE/PARTIAL/BLOCKED.

**Honesty:** "launch-ready" = all P1–P7 done + verified — likely MORE than one night. Make maximal real, verified progress; on an operator-input blocker mark BLOCKED + move to the next phase. By morning, report genuine DONE vs PARTIAL vs BLOCKED, never dressed up.

**Stop:** halt sentinels `~/.argo/buildout-stop` + `/tmp/argo-stop`. Auto-resume tick `com.skycot.buildout-resume` (30-min) re-armed 2026-05-31.

---

**Operator directive (2026-05-29 night):** go beyond the demo. Build out every
feature (voice first), remove ALL theatre, bring the platform to the mission.
Work autonomously through the night: build → test → commit; on a blocker that
needs operator input, mark it **PARTIAL/BLOCKED** and move to the next highest
value; spawn agents to build in parallel (drift-free); auto-resume across usage
limits. By morning, maximal real progress, ready to deploy what's done.

> **Honesty (NO THEATER applies to my own status too):** "the whole platform
> built by morning" is the *goal*, but a faithful build-out of every feature is
> weeks of work, not one night. I will make **maximal real, tested, committed
> progress** and report what's genuinely DONE vs PARTIAL vs BLOCKED — never dress
> incomplete work as finished.

## 🚀 SPRINT 2 — 2026-05-30 (operator: FULL PROGRAM)

Operator (awake) chose **all four tracks** + real voice. Auto-resume bug FIXED
(macOS had no `timeout`; now uses gtimeout/bg-watchdog — overnight ticks had
silently no-op'd). Sequencing (conductor drives 1-2; auto-resume + agents carry 3-5):

1. **Go live + verify** — rebuild gateway + reseed → the 5 shipped features become
   real; verify in-browser. *(conductor, in progress)*
2. **Real voice — OpenAI (vendor-selectable)** — new `openai.rs` adapter in
   `crates/argo-cli/src/handlers/gateway/routes_voice_io/`: impl `SpeechToText` via
   POST `/v1/audio/transcriptions` (whisper-1, multipart) + `TextToSpeech` via POST
   `/v1/audio/speech` (tts-1, voice param); add `Provider::OpenAi` arm in
   `provider.rs`; key via `argo_config::cred::get_unscoped("OPENAI_API_KEY")`;
   key-gated tests (skip if no key). User-selectable via a `voice` workspace_settings
   group. **Needs `ARGO_VOICE_PROVIDER=openai` + `OPENAI_API_KEY` in `~/.argo/env.local`
   for the LIVE check** (build + unit-test don't need it). True OpenAI *Realtime*
   streaming socket = later latency upgrade.
3. **Orchestration (Supervisor engine)** — per design plan 02: parallel fan-out +
   dependency layers over the existing handoff chain in `argo-orchestrator`.
4. **Theatre elimination** — T3 skill-IR seed (endpoint exists, needs IR seeded) +
   any remaining mock surfaces.
5. **Broad backlog** — FEATURE_MATRIX 🟡/🟧 rows, highest value first.

---

## 🎯 PRIORITY RANK ORDER — DEC-023 (the LIVE continuous-work queue)

**Operator locked 2026-05-30:** this is what the conductor + auto-resume machine
works **top-down, continuously, to completion**. Finish a band before dropping to
the next; on an operator-input blocker, mark PARTIAL/BLOCKED and pick up the next
item (never idle). Auto-resume tick reads THIS section each tick. Full rationale +
operating-model lock in [`decisions/DEC-023`](decisions/DEC-023-continuous-autonomous-priority-rank-order.md).

- **P1 — Kill the theatre on camera-visible screens.** Per-screen audit of the
  ~24 SPA `web/lib/api/*` modules missing the `isAuthBypassed() && !isDemoLive()`
  gate: wire each to its real endpoint where one exists; honest empty state where
  none does; never faked data. **Folds in the live queue P0s that are themselves
  theatre:** onboarding `name_from_description` names every demo hire "A"
  (`routes_onboarding_helpers.rs:315-335`) + workflow-detail Run-now disabled when
  source≠gateway. *(Highest "is it real?" lever; Portia-session resume point.)*
- **P2 — The two wow moments, end-to-end.** (a) Voice loop: TTS real (OpenAI);
  finish STT + a talk button on `/pitch` (Portia), truth-bounded. (b) Genesis
  end-to-end live: describe a role → hired teammate appears (backend+page exist;
  gap = full live verify + polish).
- **P3 — Real model-backed delegation** in `argo-orchestrator` (replace the
  `PersistingExecutor` placeholder ack with a runtime-backed `StepExecutor`).
- **P4 — Close the self-improvement loop** (A/B a skill → measurable improvement).
- **P5 / v1.x — Enterprise governance tail** (SSO SAML/OIDC, SCIM, BYOK,
  data-residency). Deferred — closes enterprise sales, not the funding round.

**Operating model (DEC-023):** conductor + on-demand worktree build-agents +
auto-resume tick. Swarm-daemon retired for now (halt sentinels stay). Auditor +
evaluator are on-demand adversarial passes, not daemons. **Build-agent harness
limit (proven):** ≤3 concurrent heavy Rust builds on this machine; partition by
independent crate for clean parallel pushes; gateway-dispatcher builds serialize.

### P1 worklist (de-theatre) — Sat May 30 14:10:37 2026 +1000

Authoritative per-module audit of every data-fetching `web/lib/api/*.ts` module.
This is the **live P1 queue** — claim WIRE rows top-down (on-camera first).

Method (all three dimensions verified against current `main`):
1. **Gated?** — does the mock short-circuit use `isAuthBypassed() && !isDemoLive()`
   (so demo-live hits the gateway), or bare `isAuthBypassed()` (serves mock even on
   camera)? `partial` = some fetchers gated, some bare in the same module.
2. **Endpoint?** — is the path it calls actually routed in
   `crates/argo-cli/src/handlers/gateway/` (dispatcher `starts_with("…")` / route
   tables), not just a test fixture? (api-keys / providers/keys / connectors have
   **no** gateway route — only CLI-side handlers.)
3. **On camera?** — does the screen it feeds appear in
   `docs/skycot/demos/v1-mission-mvp-runbook.md` (Scenes 1–6) or the Portia/Grant
   hero flow?

Verdict legend: **WIRE** = ungated/partial + endpoint-exists + on-camera (do these
first). **HONEST-EMPTY** = ungated + no backend (render an honest empty state, never
fake data). **OK** = already correctly gated. **DEFER** = off-camera (gate later, no
demo cost) or owned by another context.

**Counts: WIRE 4 · HONEST-EMPTY 3 · OK 18 · DEFER 13** (44 modules total; 14 are
pure data/helper or fully-gated infra with no theatre surface).

#### 🔴 WIRE — do these first (on-camera, real endpoint exists, currently serving mock)

| # | Module | Gated? | Endpoint (verified route) | Screen / scene | Status |
|---|--------|--------|---------------------------|----------------|--------|
| 1 | `workflows.ts` (`fetchWorkflow` detail) | partial — list gated, **detail bare** | `GET /api/v1/verticals/<v>/workflows/<n>` (`routes_workflows_handlers.rs`) | `/workflows/sales/qualify_lead` — **Scene 5** anchor | ✅ **WIRED this commit** — added `&& !isDemoLive()` at `workflows.ts:213` + demo-live test |
| 2 | `evals.ts` | ungated (bare) | `GET /api/v1/workers/<id>/evals` (`routes_evals.rs`) | worker-detail **Evals** tab (Scene 1/3 drill-in) | TODO |
| 3 | `workflow-runs.ts` | partial (1 bare, 1 gated) | `GET /api/v1/verticals/<v>/workflows/<n>/runs` (`routes_workflow_runs.rs`) | `/workflows/<v>/<n>` runs panel — **Scene 5** | TODO |
| 4 | `worker-trust.ts` | partial (2 bare, 1 gated) | `GET/POST /api/v1/workers/<id>/trust-grade[/recompute]`, `/trust-streaks` (`routes_worker_trust_grade.rs`, `routes_trust_streaks.rs`) | worker-detail **Trust** tab (Scene 1/6) | TODO |

#### 🟠 HONEST-EMPTY — no gateway route exists; render an honest empty state, do NOT fake-wire

| Module | Gated? | Why no wire | Screen | Verdict |
|--------|--------|-------------|--------|---------|
| `api-keys.ts` | ungated (3 bare) | No `/api/v1/api-keys` route (only CLI handler `connector_*`); endpoint unimplemented | `/settings/api-keys` (off-camera) | HONEST-EMPTY (also off-camera → low priority) |
| `provider-keys.ts` | ungated (2 bare) | No `/api/v1/providers/keys` route | `/settings/providers` (off-camera) | HONEST-EMPTY (off-camera) |
| `connectors.ts` | ungated (0 gate) | No `/api/v1/connectors` gateway route (CLI-only `connector_list.rs`) | `/settings/connectors`, hire Step5 | HONEST-EMPTY (off-camera) |

#### 🟢 OK — already correctly gated (`isAuthBypassed() && !isDemoLive()`), hits gateway on camera

`team.ts` · `audit.ts` · `audit-detail.ts` · `approvals.ts` (own gate, no bypass) ·
`replay.ts` · `workers.ts` · `worker-overview.ts` · `worker-activity.ts` ·
`worker-memory.ts` · `worker-skills.ts` · `worker-policies.ts` · `worker-settings.ts` ·
`skills.ts` · `cohorts.ts` · `verticals.ts` · `workflows.ts` (`fetchAllWorkflows` list) ·
`genesis.ts` (demo-live by design — Portia/Genesis hero is 100% real) · `pitch.ts` /
`gtm.ts` (hero flow, intentionally always-live — no mock layer).

#### ⚪ DEFER — off-camera (gate later, no demo cost) or owned by another context

`settings.ts` (2 bare; off-camera /settings) · `audit-export.ts` (3 bare; evidence
exporter, off-camera) · `elevated-view.ts` (1 bare; audit admin, off-camera) ·
`workplace-knowledge.ts` (partial 6 bare/1 gated; /settings/knowledge off-camera) ·
`ghostwriter.ts` (1 bare; skill-canvas, off-camera) · `skill-ir.ts` /
`skill-ir-export.ts` / `skill-ir-save.ts` (bare; skill-IR canvas, off-camera) ·
`retros.ts` · `retrain.ts` (worker dialogs, off-camera) · `notifications.ts` /
`inbound-channels.ts` / `duties.ts` / `sso.ts` / `workspace-members.ts` (off-camera
settings/nav). **`orchestration.ts`** is bare + on-camera-adjacent but is **owned by
another context** (`routes_orchestration*` / P3) — DO NOT touch here.

> Pure data/helper modules (no fetcher → not theatre): `types.ts`, `client.ts`,
> `auth.ts`, `websocket.ts`, `tenant.ts`, `skills-seed.ts`, `demo-roster-store.ts`,
> `retrain-mocks.ts`, `retrain-types.ts`, `skill-code.ts`, `skill-icons.ts`,
> `worker-overview.ts` helpers, `workflow-metadata.ts`, `test-runs.ts`.

**Next 3 WIRE items for fan-out** (after #1, done): #2 `evals.ts`, #3
`workflow-runs.ts`, #4 `worker-trust.ts` — all single-file, gate-only changes
identical in shape to #1 (`if (isAuthBypassed())` → `if (isAuthBypassed() && !isDemoLive())`
on the data-fetch short-circuits; keep mock as offline/non-demo-live fallback).

---

## ☀️ MORNING HAND-OFF (Mark — read this first)

Good morning. The interactive conductor (me) ran until its context filled, then
handed the baton to the **autonomous machine** — it keeps building with fresh
context (auto-resume tick, checked every 30 min). Honest picture:

### Shipped tonight — real, tested, committed to `main` (7 feature commits)
1. **Voice V1** — `argo-voice-io` clean trait layer (`SpeechToText`/`TextToSpeech`)
   + key-free `MockStt`/`MockTts` + `voice_turn` seam. 17u+36doc tests.
2. **Voice V2** — gateway `POST /api/v1/voice/{tts,stt,pitch-turn}`; pitch-turn
   reuses the **real truth boundary** (over-claim → defers, no `$` figure). Voice
   crate now wired into the gateway. 12 tests.
3. **Settings T1** — `workspace_settings` table (migration v85) + GET/PATCH for
   workspace/providers/notifications; `/settings` un-mocked. 16 tests.
4. **Evals T2** — real worker-evals query (skills' reeval history); evals tab
   un-mocked. 6 tests + 233 vitest.
5. **Governance report** (gap #1) — `GET /api/v1/governance/readiness`: real audit
   chain-integrity + approval coverage + truth-boundary status. 4 tests.
   *(Earlier this session also shipped `argo eval export-training-data` (gap #3),
   `GET /workers/<id>/retrain/recent` + seed, and the NO-THEATER retros/audit-export flips.)*

### To take it live (backend is committed; the RUNNING gateway is still the old binary)
```bash
bash ~/code/argo/scripts/restart-live-gateway.sh --force --no-prompt
```
Then reseed (populates retrain history etc.):
```bash
cd ~/code/argo && set -a && source ~/.argo/env.local && set +a && cargo run --release --bin argo -- demo seed --tenant 019e7344-78e1-77f1-ae99-dff2699683ea
```
After that: voice routes, `/settings`, evals tab, `/governance/readiness` and the
retrain panel all go live (migration v85 applies automatically on restart).

### Needs your decision (parked blockers, per your instructions)
- **Voice real vendor + API key** — recommend ElevenLabs + Deepgram. Unblocks the
  real STT/TTS adapter (everything else in voice is built + mock-tested).
- **Orchestration model** — Supervisor (recommended) vs visual DAG, before I build it.
- **Browser voice (V3)** — buildable, but needs the gateway rebuilt + a real browser
  to verify mic capture, so best done with you around.

### Honest deploy-readiness
**Substantial real progress, not "the whole platform"** (that's weeks, not one
night — flagged up front). What shipped is production-quality + tested. The
autonomous machine keeps chipping the queue + FEATURE_MATRIX overnight.

### Auto-resume is RUNNING — off-switch
`com.skycot.buildout-resume` (30-min check) keeps building with fresh context.
**To stop when you're back + driving:** `touch ~/.argo/buildout-stop` then
`launchctl unload ~/Library/LaunchAgents/com.skycot.buildout-resume.plist`.

---

## RESUME INSTRUCTIONS (read first if you're a fresh/resumed conductor)

0. **Dirty-tree first:** `cd ~/code/argo && git status --short | grep -v 'verticals/.genesis'`.
   If a prior build-agent left UNCOMMITTED changes (a crate mid-integration),
   verify them (`cargo test`/`clippy` on the touched crate, or `vitest`/`tsc`) and
   either commit+push if green, or `git stash`/discard if broken — leave a clean
   tree before starting new work. Note what you did in the Log.
1. `git fetch origin -q && git log --oneline -8`
2. Read this file's **Queue** + **Log** below — pick the first `TODO`/`PARTIAL`
   item by priority that isn't `BLOCKED`.
3. Build it (single Rust integrator: `export CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target CARGO_INCREMENTAL=0`).
4. Verify (cargo test/clippy for Rust; vitest/tsc for web). Commit + push ONLY if
   green. Append a Log line. Never break the running demo (gateway :7400 / SPA :3000).
4a. **Keep the two source-of-truth docs current IN THE SAME COMMIT/CYCLE** (this is
   load-bearing — it is how the docs stop falling behind the code):
   - **Every shipped feature** updates its row in
     [`docs/build-plan/FEATURE_MATRIX.md`](../../build-plan/FEATURE_MATRIX.md) —
     flip the status per the legend + cite the commit SHA + crate/route.
   - **Every operator-locked call** gets a `docs/skycot/decisions/DEC-NNN-*.md` ADR
     + an index line in `docs/skycot/decisions/README.md` (and refresh VERSION_STATE.md
     if the "where are we" answer changed).
   Do this in the **same commit/cycle** as the feature/decision, not as a later
   sweep. **Build-agent prompts MUST include this rule** so delegated work carries it.
4b. **Migration rule (load-bearing — a filtered test run caused a gateway-down
   incident on 2026-05-30):** any migration-touching build MUST bump
   `EXPECTED_SCHEMA_VERSION` in `crates/argo-store/src/lib.rs` AND run the FULL
   `cargo test -p argo-store` suite (NOT a filtered subset) — a filtered run misses
   the schema-reconciliation tests. (On 2026-05-30 the T1 v85 migration was
   registered + applied but `EXPECTED_SCHEMA_VERSION` wasn't bumped, so the gateway
   refused to boot against the migrated DB until fix `ccfa89588`.)
5. Keep going until every remaining item is BLOCKED. **When the explicit Queue
   below is exhausted, FIND MORE WORK** (operator directive: "find more work until
   the whole platform is built"): open `docs/build-plan/FEATURE_MATRIX.md`, pick the
   highest-value 🟡 Partial / 🟧 Scaffold row that is buildable without an operator
   decision or a missing API key, append it to the Queue as a new item, and build
   it. Prefer: remaining theatre/mock surfaces → un-wired-but-shipped scaffolds →
   FEATURE_MATRIX partials. Always: real + tested + committed, NO THEATER, don't
   break the demo. Mark anything needing operator input BLOCKED + move on.

**Process note (improved mid-run):** delegated build-agents now **commit+push their
own green work** (verify gate: tests+clippy green), run ONE at a time to avoid push
races — this avoids a dirty-tree handoff if the conductor session dies. Gateway-
dispatcher-touching builds (anything editing `routes_dispatch_*.rs` / gateway
`mod.rs`) must NOT run in parallel with each other (shared files → merge conflicts);
serialize those. Independent crates can parallelize via worktrees + the shared
target (cargo's target lock serializes compilation = memory-safe).

**Process note (docs-stay-live rule — 2026-05-30):** the docs fell behind the code
once (the night's shipped features + operator-locked calls weren't reflected in
FEATURE_MATRIX or the decision register until a catch-up pass). To prevent recurrence:
**every shipped feature updates `FEATURE_MATRIX.md` (status flip + commit SHA) and
every operator-locked call gets a `docs/skycot/decisions/` ADR — in the SAME
commit/cycle as the work**, never as a later sweep (see RESUME step 4a).
**Build-agent prompts MUST carry this rule** so the discipline survives delegation.
DEC-019 makes this the standing program rule.

## The autonomous process (v1 — improve as we learn)

- **Conductor = this session.** Sole Rust **integrator/builder** through ONE warm
  target (`~/.cache/argo-shared-target`). Serialized cargo = disk- + memory-safe
  (the machine has only ~105 GB free + a history of jetsam-killing parallel cargo).
- **Parallel agents for LIGHT, file-partitioned work**: web/TS theatre-removal
  (vitest/tsc, no cargo), read-only audits, brief authoring, design. Each agent
  reads `CLAUDE.md` + `docs/skycot/DRIFT_PREVENTION.md` + the target files +
  greps for existing surfaces BEFORE writing (no drift). Agents are partitioned
  by file so their outputs never conflict → **no rebasing needed**.
- **Heavy Rust** is built by the conductor (or one agent at a time), integrated
  serially onto fresh `main`. Single integrator ⇒ rebasing is largely avoided; if
  a genuine conflict appears, reconcile inline (no force-push, `--force-with-lease`
  only if ever unavoidable).
- **NOT relighting the bash-loop swarm** this run: queue is near-empty, disk is
  tight, and parallel-cargo memory pressure is the exact failure that halted it.
  The conductor+agent model above is safer and I control it directly.
- **Auto-resume:** a launchd scheduled task re-fires the conductor continuation
  after a usage-limit reset / session death (see setup below). Progress persists
  in this file, so resume is stateless.

## Queue (priority order · status: TODO / WIP / PARTIAL / BLOCKED / DONE)

### P0 — Voice (operator's explicit first item)
- [x] **V1 · `argo-voice-io` trait redesign + mock adapter** — **DONE.** Clean L1
  layer: `SpeechToText`/`TextToSpeech` async traits + `Transcript`/`AudioClip`/
  `VoiceId` types + key-free deterministic `MockStt`/`MockTts` + `voice_turn(stt,
  tts, voice, audio, answer_closure)` composition seam (pitch-ask plugs into
  `answer` at V2). Old L2 stub removed. 17 unit + 36 doctests + clippy `-D warnings`
  green. Still orphan (nothing to break).
- [x] **V2 · Voice-loop over the real truth-bounded `pitch-ask`** — **DONE.**
  `POST /api/v1/voice/{tts,stt,pitch-turn}` in `routes_voice_io.rs` (named to avoid
  the existing Twilio-DTMF `routes_voice.rs`). `pitch-turn` reuses
  `routes_pitch_ask::ask_response` in-process (ZERO truth-boundary duplication) —
  test proves an over-claim ("What's your valuation?") defers + emits no `$`
  figure. Provider seam reads `ARGO_VOICE_PROVIDER`, Mock today, real adapter seam
  marked BLOCKED-on-key. `argo-voice-io` now **un-orphaned** (wired into gateway).
  12 voice tests + clippy green.
- [x] **V2b · Real OpenAI STT/TTS adapter (DEC-021)** — **DONE (live key-path
  untested).** `routes_voice_io/openai.rs`: `OpenAiStt` (multipart whisper-1) +
  `OpenAiTts` (tts-1 mp3) behind the L1 traits; Bearer `OPENAI_API_KEY` (`Redacted`,
  shared client). `Provider::OpenAi` arm + per-tenant `voice` workspace_settings
  group (`/api/v1/voice/settings`, default `mock`) preferred over the env. No-fake:
  no-key ⇒ 503. 31 voice+settings tests + clippy `-D warnings` + FULL argo-store all
  green. Vendor was locked to **OpenAI** (DEC-021), superseding the earlier
  ElevenLabs/Deepgram note. **Remaining: a live OpenAI call has not been exercised —
  no `OPENAI_API_KEY` at build time** (key-gated test skips). Set the key + re-run to
  verify the network path; that flips the FEATURE_MATRIX row 🟡→✅.
- [ ] **V3 · Browser push-to-talk on `/pitch`** — web; depends on V2 routes (now exist).

### P1 — Theatre removal (audit + build-map DONE; revised by findings)
Build-map finding: **T3 + T4 backends already exist** (routes_skill_ir.rs,
routes_elevated_view.rs) — only the SPA gate is theatre. BUT skill-IR endpoint
404s for seeded skills ("skill has no IR") → flipping needs IR **seeded** first.
- [x] **T1 · settings/workspace** — **DONE.** New generic `workspace_settings`
  table `(tenant_id, group_key, value_json, updated_at)` via migration **v85**
  (inline-SQL, additive, idempotent; slot-check clean) + tenant-scoped
  `argo_store::workspace_settings` get/upsert + `routes_workspace_settings.rs`
  GET/PATCH for `/api/v1/workspace`, `/api/v1/providers/policy`,
  `/api/v1/notifications/preferences` (defaults mirror `settings.ts` DEFAULT_*,
  shallow-merge + shape-validate on PATCH, 400 on unknown-key/wrong-type). Wired
  into the GET + PATCH dispatchers; `settings.ts` `isAuthBypassed()` now opts out
  under `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` so demo-live shows real persisted
  settings. 8 store unit tests (incl. migration-on-:memory: + tenant-scoping) +
  8 handler tests (defaults / round-trip / validation) + clippy `-D warnings`
  green; `settings.ts` typechecks (6 pre-existing unrelated TS errors only).
- [x] **T2 · worker evals** — **DONE.** `routes_evals.rs` rewritten from the
  hardcoded-sample handler to a real store-backed query: a worker's evals = the
  scheduled-reeval history of the SKILLS that worker has learned. Resolves the
  worker's learned skill ids via `IdentityStore::skills_for_worker` (tenant-scoped
  `worker_skill` overlay), fetches `skill_reeval_history` rows for those ids
  (the table is a global skill catalog with no `tenant_id`; tenant scope is
  enforced on the worker side — `resolve_worker_ctx` looks the worker up under
  the workspace tenant, cross-tenant/unknown → 404, so an alien worker's reeval
  rows never reach the response), and maps each row to the SPA `EvalRun` shape
  (`pass_rate`→`passRate`, `run_at` micros→`createdAt` RFC3339, `regressed`
  flag→`decision`; table-absent fields `fixtures`/`fixtureCount`/`durationMs`/
  `meanScore` filled with honest derived values, not invented detail).
  `latest` = most-recent, `history` = newest-first. A skill-less / history-less
  worker returns `{workerId, latest: null, history: []}` — a **real empty, not
  mock**. The handler defensively ensures the lazily-applied
  `skill_reeval_history` table (it is NOT in the migration registry; `argo-skill`
  applies it on first reeval tick). `evals.ts`: `isAuthBypassed()` now opts out
  under `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` so demo-live shows real eval data;
  `EvalDashboardPayload.latest` widened to `EvalRun | null` and `isPayload()`
  accepts `latest === null` so a real empty isn't mistaken for an unreachable
  gateway and swapped back to mock. 6 handler tests (real shape for a worker WITH
  a seeded skill + two reeval rows incl. ordering/decision/version assertions;
  empty-valid for a worker with none; learned-skill-but-no-history empty;
  cross-tenant 404; invalid-id 400; no-store 503), clippy `-D warnings`, tsc
  (only pre-existing unrelated errors), vitest 233/233 all green. Files:
  `routes_evals.rs` (261 LOC), `routes_evals_tests.rs` (292 LOC, new),
  `routes_dispatch_get_api_groups.rs` (+1 store arg), `evals.ts`,
  `EvalDashboard.test.tsx` (mock-return narrow to stay tsc-green). Commit
  `53933f011`.
- [ ] **T3 · skill-IR** — backend EXISTS (flip-ready) but needs **IR seeded** for
  demo skills (else 404→mock). Real fix = seed skill IR (like retrain seed) + flip
  3 modules. LOWER (forge drill-in, not core demo). Seed is the real work.
- [~] **T4 · elevated-view** — DEFERRED. Backend exists, but flipping risks a 400
  if the SPA request doesn't meet the backend's validation (reason ≥20 chars,
  category/duration checks) — the module re-throws non-404 errors → broken
  interaction in demo-live. Needs SPA↔backend request-validation alignment first.
  Low camera value (ops break-glass), so parked behind T1/T2.
- **SKIP (credentials — privacy rule):** `api-keys.ts`, `provider-keys.ts`.
- **SKIP (no theatre):** `ghostwriter.ts`, `notifications.ts` (empty, not fake).

### P2 — Gap features (from the 4 design plans)
- [x] Self-improvement Step 1b (eval-case generation) — additive, no key. **DONE** (commit `99114bd5a`).
- [x] Governance audit-readiness report (gap #1 move A) — read-only assembly. **DONE** (commit `764f725ff`).
- [ ] Orchestration supervisor engine — **BLOCKED: operator to confirm Supervisor vs DAG.**
- [x] (voice real-vendor — see V2b: OpenAI adapter SHIPPED; live key-path untested)

### P3 — Mission-completion + hardening
- [ ] (populated as P0–P2 land + from FEATURE_MATRIX 🟡/🟧 rows)

## Blockers needing operator input (parked, not abandoned)
- **Voice live verification** — the OpenAI STT/TTS adapter is BUILT (DEC-021); to
  flip its FEATURE_MATRIX row 🟡→✅ an `OPENAI_API_KEY` must be present so the
  key-gated round-trip test exercises a real call. (Vendor locked to OpenAI;
  superseded the earlier ElevenLabs/Deepgram recommendation.)
- **Orchestration model** — Supervisor (recommended) vs visual DAG.
- (others appended as hit)

## Log (one line per build cycle)
- 2026-05-29 (kickoff) — state assessed: main@22d94db73 clean+synced; demo up;
  swarm halted; disk 105 GB free (191 GB in caches → single-target discipline);
  process v1 set. Starting V1 (voice traits) + delegating T-audit in parallel.
- 2026-05-30 — **T1 settings backend DONE.** Migration v85 (`workspace_settings`
  generic KV table) + `argo_store::workspace_settings` + gateway
  `routes_workspace_settings.rs` (GET/PATCH × workspace/providers/notifications,
  defaults from `settings.ts`, PATCH merge+validate) + dispatcher wiring +
  `settings.ts` demo-live gate flip. 8 store + 8 handler tests, clippy -D warnings,
  migration-slots, tsc(settings) all green. `/settings` now read-backed + persisted.
- 2026-05-30 — **T2 worker-evals DONE.** `routes_evals.rs` un-mocked: real
  store-backed query (worker's learned skills → their `skill_reeval_history`
  surfaced as the worker's eval runs), tenant-scoped on the worker side
  (cross-tenant → 404), `run_at` micros → RFC3339, `regressed` → decision; real
  empty (`latest: null`) for a worker with no reeval history (not mock).
  `evals.ts` `isAuthBypassed()` opts out under `NEXT_PUBLIC_SKYCOT_DEMO_LIVE`;
  `latest` widened to `EvalRun | null` + `isPayload` accepts null so a real empty
  isn't swapped back to mock. 6 handler tests + clippy -D warnings + vitest 233/233
  + tsc (only pre-existing unrelated errors) green. Commit `53933f011`. `/workers/
  <id>` evals tab now real-backed under demo-live.
- 2026-05-30 — **P2 governance audit-readiness report DONE (gap #1 move A).**
  New `routes_governance.rs` + tests + dispatcher wiring (`crates/argo-cli/` only):
  `GET /api/v1/governance/readiness[?tenant=<id>]` → read-only assembly of the
  moat we already have — `audit:{chain_intact,action_count}` (runs
  `argo_audit::verify_session` over every session the tenant owns, ANDs the
  verdicts, sums rows), `approvals:{decided,pending,coverage_pct}` (direct
  `approval_request` counts by tenant; coverage `null`+note when no requests, no
  fake 100), `capabilities` (the 4 default RBAC role presets from
  `default_roles_for_workspace` — names surfaced; per-principal dynamic grants
  not persisted/countable → `// follow-up:` note), `truth_boundary:{enabled}`
  (env `ARGO_TRUTH_BOUNDARY_POLICY_PATH` policy that resolves to the tenant;
  factual `false`+note when unset). NO THEATER — every value read straight from
  source. Tenant-scoped (explicit `?tenant=` or cwd workspace); other tenant →
  clean empty report, no leak. 4 handler tests (real counts, cross-tenant
  isolation, malformed-tenant 5xx, non-match passthrough) + clippy -D warnings
  green; module 264 LOC / test 185 LOC, longest fn 36 LOC, no panicking
  unwrap/expect (unwrap checker `--all` clean). Commit `764f725ff`.
- 2026-05-30 go-live DONE — gateway rebuilt→schema 85, 5 features verified live; resolved a boot incident (T1 v85 migration didn't bump EXPECTED_SCHEMA_VERSION → fix ccfa89588).
- 2026-05-30 — **V2-real-vendor: OpenAI STT/TTS adapter SHIPPED (DEC-021).** New
  `routes_voice_io/openai.rs`: `OpenAiStt` (real multipart whisper-1
  `/v1/audio/transcriptions`) + `OpenAiTts` (real tts-1 mp3 `/v1/audio/speech`),
  Bearer `OPENAI_API_KEY` (read `Redacted`, one shared `reqwest::Client`). Behind the
  L1 `argo-voice-io` traits — NO THEATER (real calls when keyed; no-key ⇒ 503, never
  faked). Vendor- **and** user-selectable: `Provider::OpenAi` arm in `provider.rs` +
  per-tenant `voice` workspace_settings group (`GET/PATCH /api/v1/voice/settings`,
  default `mock`) preferred over `ARGO_VOICE_PROVIDER`; the three voice routes now
  thread `store` + use the fallible async selector. 31 voice+settings tests green
  (incl. 4 new voice-group/selector tests + key-gated live round-trip that skips
  without a key); clippy `-D warnings` clean; FULL `cargo test -p argo-store` green
  (526 + integration + 101 doctests, schema-reconciliation tests pass). **Live
  network path UNTESTED — no `OPENAI_API_KEY` was present at build time.** To verify
  live: set `OPENAI_API_KEY` (+ optionally `ARGO_VOICE_PROVIDER=openai` or PATCH the
  tenant `voice` group to `openai`) and re-run the key-gated test in
  `routes_voice_io/openai.rs`. NB: the full `cargo test -p argo-cli --bins` run shows
  ~40 PRE-EXISTING failures (confirmed identical on stashed clean main: DB-lock
  parallelism flakes in routes_approvals_json/routes_audit_feed + the documented
  setup_steps/stream_a strict-mode flakes in KNOWN_FLAKES.md) — none in the voice or
  settings files; boundary-noted per SWARM_PROTOCOL HARD RULE 2.
- 2026-05-30 — **demo-seed: two theatre surfaces now have real data.** New
  `argo-demo-seed/src/skill_ir_eval.rs` (418 LOC) adds two idempotent seed fns
  wired into `run_canonical_v1_seed`: `seed_skill_ir` persists a real `SkillGraph`
  per marketplace catalog skill via the canonical canvas writer
  (`argo_skill::ir::save_edited_graph` — valid hash + shape, NOT hand-rolled SQL),
  keyed on the SAME id the /skills SPA lists, so `GET /skills/<id>/ir` no longer
  404s; `seed_skill_reeval_history` inserts a 3-run trend (one regression) per
  catalog skill into `skill_reeval_history`, keyed on the hero workers' learned
  skill ids, so `GET /workers/<id>/evals` returns a real trend instead of
  `{latest:null,history:[]}`. Both idempotent via sentinel (reseed ⇒ 0). New
  `SeedReport` fields `skill_ir_seeded` + `skill_reeval_rows_seeded` + CLI print
  lines. 6 new tests (per-surface counts, idempotency, tenant-scope, end-to-end
  worker_skill→reeval join) + updated `skip_pass_a_seeds_dressing_only`; FULL
  `cargo test -p argo-demo-seed` green (34 unit + 4 integration) + `cargo clippy
  -p argo-demo-seed -- -D warnings` clean. No migration (both tables already
  exist). Full-live after next gateway rebuild + reseed.
- 2026-05-30 — **P2 self-improvement Step 1b DONE (gap #3, the honest loop).** New
  `argo eval generate-cases --tenant <t> --out <file.jsonl> [--json]` turns the same
  real *judged* events the SFT export draws on into argo-eval regression cases (the
  existing `EvalTask` shape): each human-REJECTED approval → a `must_refuse_or_defer`
  case (`SubstringContains ["deferred","approval"]`, the exact text the dry-run worker
  pipeline emits), each committed retrain → a corrected-behaviour case (vocab→new term,
  action_ban→refuse, policy/skill/fact→keyword from the recorded change). Generator
  logic in a focused **store-free** module `argo-eval::case_gen` (keeps the eval crate
  lean — no argo-store/sqlx dep added); the store read + PII-redaction stay CLI-side.
  Extracted a shared judged-history reader `eval_judged_history.rs` so the
  approval+retrain join lives once — the export handler now reuses it (no duplicated
  queries). Read-only (no table, no migration), tenant-scoped at the query boundary,
  free text through the same `pii_detect::scan_and_maybe_redact` as the audit sink.
  Generated JSONL round-trips through the real `load_jsonl` (proven in a test) so cases
  are guaranteed runnable. Tests: 7 `case_gen` unit + 4 CLI handler (seed→generate→
  non-empty/well-formed/loads/tenant-scoped/PII-safe) + argo-eval `surface_contract`
  updated + a `generate_cases` doc-test; all green, both clippy gates `-D warnings`
  clean. E2E smoke vs a seeded throwaway home produced 6 real cases (1 rejected + 5
  retrains) that `argo eval validate` confirms load (`task_count:6`). Commit `99114bd5a`.
- 2026-05-30 — **Orchestration: Supervisor engine SHIPPED (design plan 02 phases
  1+2, DEC-022).** New `argo-orchestrator` modules `team_plan.rs` (TeamPlan /
  DelegationStep / StepId + pure topological-layer builder from `depends_on`),
  `team_run.rs` (`Supervisor::run_team` — a lead fans N delegations out per layer
  with **bounded concurrency** via a tokio Semaphore, **barriers** between
  topological layers, and **drops dependents** of a failed/skipped step instead of
  hanging; gathers `Vec<StepOutcome>` → `TeamResult` for the lead to synthesize;
  pluggable `StepExecutor` trait so the engine carries no runtime coupling), and
  `team_persist.rs` (`PersistingExecutor` — records each child + the lead synthesis
  as **existing** `provider='orchestrator'` session rows; **no new migration /
  table**). `tenant_id` + `current_handoff_id` threaded through every step so the
  team run hangs off the existing handoff chain and stays audit-coherent. Gateway:
  `POST /api/v1/orchestration/team-runs` (`routes_orchestration_team.rs`, additive;
  wired into the existing `dispatch_orchestration` POST sub-dispatcher + catalog).
  NO THEATER — the persisting executor writes genuine tenant-scoped run rows that
  show up in `GET /api/v1/orchestration/runs` immediately; the real LLM-backed
  delegation executor is the additive follow-up (same trait) + the SPA team-tree
  view is Phase 3. **Verify (cold, `CARGO_TARGET_DIR=/tmp/wf-orch`):**
  `cargo test -p argo-orchestrator --lib` → 166 passed / 0 failed (incl. 19 new
  team_* tests: 3-parallel-complete, A→{B,C}→D ordering, failure-drops-dependents,
  transitive-skip, concurrency-cap, cross-tenant isolation); `cargo test -p
  argo-cli --bins team_run` → 9 passed; `cargo clippy -p argo-orchestrator
  --all-targets -- -D warnings` clean. (Pre-existing unrelated failure
  `kpi_rollup_test::rollup_skipped_if_no_audit_activity_since_last_rollup` fails
  3/3 on stashed clean main too — not mine.) Commit `de3c58e2e`.
- 2026-05-30 — **seed.rs split DONE (no-behavior-change).** `argo-demo-seed/src/seed.rs` 752→443 LOC: extracted `#[cfg(test)] mod tests` → `#[path="seed_tests.rs"]` sibling (174 LOC) + moved the two per-worker overlay seeders (`seed_worker_skill_assignments` + `seed_retrain_records`) into a `seed_worker_overlays.rs` topic module (166 LOC, `pub(crate)` fns called via `crate::`), mirroring the `skill_ir_eval` precedent. Every src file now ≤500 (Gate 3 clears without --no-verify). Verify (isolated `/tmp/wf-seedsplit`): `cargo test -p argo-demo-seed` 34 unit + 4 integration green; `cargo clippy -p argo-demo-seed -- -D warnings` clean.
- 2026-05-30 — **orchestration team-run tree view DONE (design plan 02 Phase 3).**
  `/orchestration` now renders the Supervisor's team runs as a lead-with-children
  tree: each lead teammate in a `--brand-gradient` Audiowide banner with the child
  delegations beneath, every child showing its own live status + a done/total
  counter. NO THEATER — driven by the real `GET /api/v1/orchestration/runs` rows;
  verified live against the running gateway (schema 85), which already returns the
  `[team]`-prefixed rows a real `POST /team-runs` wrote (`[team] lead synthesis —
  smoke`, `[team] b → y`) — page showed "11 runs shown from gateway", 0 console
  errors, gradient + Audiowide confirmed via computed styles. Honest gap: the read
  endpoint exposes **no per-team-run id**, only the `[team]` title prefix, so
  `groupTeamRuns()` groups all `[team]` rows (lead-synthesis rows → lead nodes,
  step rows → children); a single lead gets a precise tree, multiple leads fall
  back to a shared "Delegated steps" node rather than guessing edges — adding a
  `teamRunId` to the run JSON is the queued follow-up. Falls back to the flat
  queue when there's no team linkage; mock-gate preserved (`SAMPLE_TEAM_RUNS` only
  on auth-bypass/offline, never demo-live). New `TeamRunTree.tsx` + grouping
  helpers in `lib/api/orchestration.ts`. Verify: `npx tsc --noEmit` (touched files
  0 errors; 6 pre-existing errors are in untouched `sample-workflows.ts` /
  `SubWorkflowPicker.test.tsx` / `settings-workspace-tenant-id.test.tsx`); `npx
  vitest run` orchestration + TeamRunTree + QueueTable = 22 passed. Commit
  `59aa69080`.
- 2026-05-30 — **skill-IR 404 FIXED (commit `8a5fb0bd0`).** The live "skill has no
  IR" 404 was a **tenant-resolution divergence, not an id-space mismatch** (the
  suspected cause): `seed_skill_ir` and the `routes_skill_ir` GET key on
  byte-identical `SkillId` bytes (proven by a dispatcher test that seeds the
  catalog, decodes the ids exactly as `GET /api/v1/skills` does, and gets `200`
  per id). The real cause: a header-less IR GET fell back to `auth.tenant_id`
  (nil `00000000-…` under dev-bypass) while `GET /api/v1/skills` falls back to the
  cwd workspace tenant the seed wrote under → list showed 7 skills, IR drill-in
  looked under nil → 404. Fix: `resolve_tenant` now resolves the cwd workspace
  tenant on a header-less request (identical to the list endpoint); header wins
  when present; cross-tenant `as_tenant` gate unchanged. `argo-cli` only
  (`routes_skill_ir.rs` + `routes_skill_ir_helpers.rs`). Verify (isolated
  `/tmp/wf-skillir`): `cargo test -p argo-cli --bins skill_ir` 14/14 PASS (2 new
  dispatcher tests, one of which reproduced the live 404 RED→GREEN);
  `cargo test -p argo-demo-seed` 34+4 PASS; `cargo clippy -p argo-cli --bins
  -- -D warnings` clean (the `--tests` profile surfaces one PRE-EXISTING unrelated
  lint, `seed_reeval_row`/8-args in untouched `routes_evals_tests.rs` — not mine).
  FEATURE_MATRIX §3 skill-IR row updated.
- 2026-05-30 — **Deploy CODE-drift detection DONE — closes the code-only go-live
  gap.** Root cause of the validated 2026-05-30 problem (the skill-IR fix
  `7c49d425f` + orchestrator team-run `13ce62800` were green + merged but never
  went live until a manual `--force`): both deploy scripts decided "rebuild?" on
  DB `schema_version` alone, so a code-only change (no migration) left the OLD
  binary running. The SHA-embed half was already shipped (#2605: `build.rs` →
  `build_info::GIT_SHA`, `/health` `build_commit` field, freshness commit-drift);
  this change wires the SHA signal into the two deploy *decisions* so it actually
  triggers: (1) `scripts/restart-live-gateway.sh` step 2 now no-ops ONLY when
  schema matches AND live `build_commit` == `git rev-parse --short=7 HEAD` — a SHA
  mismatch (or schema drift, or `--force`) takes the rebuild+restart path;
  best-effort fallback to schema-only when `build_commit` is `"unknown"`
  (tarball/pre-field binary) or HEAD is unresolvable. (2)
  `scripts/check-gateway-freshness.sh` now escalates to STALE on ANY commit drift
  > 0 (was: only >50 commits OR a gateway-handler file change — which let
  orchestrator/runtime-only changes read FRESH); the threshold + handler prefix
  now only colour the human STATUS line. No new dependency, no new route, no new
  field — reuses the canonical `build_commit`. Additive; `--force` + schema paths
  intact. Verify (real-disk `~/.cache/wf-deployharden`): `cargo build -p argo-cli
  --bin argo` OK (build.rs embedded `f9ac02a` == HEAD short-7, proven via `argo
  --version`); `cargo test -p argo-cli --bin argo routes_health` 6/6 PASS (incl. a
  strengthened `build_commit` 7-hex-or-"unknown" shape assertion the scripts
  depend on); `cargo clippy -p argo-cli --bins -- -D warnings` clean; `bash -n`
  both scripts OK; freshness-script simulated CASE 1 (schema-fresh + SHA-stale →
  STALE/exit1), CASE 2 (all fresh → FRESH/exit0), CASE 3 (1 commit behind, 0
  handler files, 0 schema drift → STALE/exit1, the new catch); restart-script
  step-2 decision traced across all 7 branches (no-op / SHA-drift-rebuild /
  schema-drift-rebuild / --force / unknown-fallback / no-git-fallback /
  no-gateway). Live confirmation: the running :7400 gateway reports
  `build_commit:0cd7c85` while HEAD is `f9ac02a` — it IS serving stale code with a
  fresh schema right now, the exact bug, and the fixed script would catch it.
  Dev-deploy tooling, not a product capability — no FEATURE_MATRIX row applies
  (it tracks shipped product capabilities + routes; this hardens the build/deploy
  harness, which the matrix doesn't enumerate). NB the conductor still owns the
  end-to-end restart-against-:7400 check (not run here to avoid disturbing the
  live demo gateway).
- 2026-05-31 — **Genesis P3 flagship-path SAFETY closure (`genesisPlanId` stamp).**
  A parallel auto-resume tick shipped the full P3 probation engine (`629e4fe35`) while
  this session independently built the same design — so the Rust/identity/workflow work
  was redundant and discarded; local fast-forwarded to origin. BUT an independent
  adversarial bypass-hunt during this session found a REAL hole that `629e4fe35` (and the
  P2 fence before it) both missed: the probation + fence stamp fires only when the hire
  `draft` carries `genesis_plan_id`, and the **flagship SSE `/genesis` hire draft never set
  it** (`buildHireDraftFromGenesis` in `web/lib/api/genesis.ts`). So the headline teammate
  was being hired with NEITHER the P2 capability fence NOR P3 probation — owner `*` keys,
  no dry-run — strictly weaker than `/onboarding/first-hire`. The P3 commit's FEATURE_MATRIX
  row even claimed "BOTH paths carry it," which was false on the demo path. Fix (this commit):
  `buildHireDraftFromGenesis` now sets `genesisPlanId: ready.vertical_id`, so the SSE worker
  is genuinely fenced + probationed; `genesis.test.ts` asserts the SSE draft carries it (19
  vitest green, tsc clean). Corrected the P3 FEATURE_MATRIX row's false "both paths" claim +
  regenerated HTML in this commit. **Process note:** the ≤3-concurrent-builds model still let
  TWO contexts pick the same phase (this session + the auto-resume tick) → duplicated P3 build.
  The auto-resume tick should check `git fetch origin` for an in-flight phase commit before
  starting. **Still open (P5 parity):** the SSE hire still seeds no worker memory (empty Memory
  tab) — separate non-safety follow-up, do with P5. **Live-verify on :7400 deferred** (running
  binary `05f9ea2` predates even origin's P3; a gateway rebuild is the conductor's call to avoid
  disturbing the demo mid-run).

## Sprint-2 follow-ups (tracked 2026-05-30 — these were floating chips; now in the queue)
- [ ] **Landing page — 4 verified differentiators** — BLOCKED on operator design pick (optometrist method: generate 3-5 wild mockups → operator chooses a direction → implement on web/app/landing/page.tsx). Real proof to pair per pillar; do NOT auto-edit the live page first. (chip from the first overnight session)
- [x] **Split argo-demo-seed/src/seed.rs (752 LOC > 550 hard ceiling)** — DONE 2026-05-30: extracted `#[cfg(test)] mod tests` to a `#[path="seed_tests.rs"]` sibling + moved two seed_* helpers to `seed_worker_overlays.rs`; seed.rs now 443 LOC, every file <500, Gate 3 clears without --no-verify. Test+clippy green. (See Log entry above.)
- [ ] **Orchestration follow-ups** (per de3c58e2e Supervisor engine): real LLM-backed `StepExecutor` (today's is an honest persisting executor, real rows, no faked output). ~~SPA team-tree view (design plan 02 Phase 3)~~ DONE 2026-05-30 (commit `59aa69080`, see Log) — lead-with-children tree on `/orchestration`, real gateway rows. ~~**New follow-up:** add a `teamRunId` to the `GET /api/v1/orchestration/runs` row JSON so the tree can draw precise per-team-run edges (today it groups by the `[team]` title prefix; multiple concurrent team runs share a "Delegated steps" node rather than guessing edges).~~ **DONE 2026-05-30 (Option A — title-encoded id, NO migration; see `git log` for SHA):** the Supervisor mints one run-wide `team_run_id` per `run_team` and embeds it in every persisted row title (`[team:<id>] …`); `groupTeamRuns()` now buckets by that id so two concurrent team runs render as two separate precise trees, with a graceful fallback that still groups legacy bare-`[team]` rows under a shared node. Verified: `cargo test -p argo-orchestrator` lib 168/168 (incl. new `all_rows_in_a_run_share_one_team_id_prefix` + `two_runs_get_distinct_team_ids`); `vitest run` orchestration + TeamRunTree 21/21 (incl. new two-concurrent-teams case); `tsc --noEmit` 0 new errors in touched files.
- **Done/decided chips (dismiss):** retrain-recent (shipped via audit-log, 5cae58294); settings+evals backends (T1 423274a25 / T2 53933f011 + seed ff4167292); api-keys/provider-keys = intentionally NOT surfaced (privacy).

## Go-live verification 2026-05-30 (gateway rebuilt → schema 85, clean boot)
VERIFIED LIVE on the running gateway: orchestration `POST /team-runs` → 202 (real 2-step dependency plan); worker evals → latest + 6 history rows (reeval seed worked); settings/governance/retrain live.
- [x] **BUG: skill-IR 404 — FIXED (commit `8a5fb0bd0`).** Root cause was NOT the
  suspected id-space mismatch: `seed_skill_ir` and `routes_skill_ir` GET key on
  byte-identical `SkillId` bytes for the marketplace `skill.id` (proven — a new
  dispatcher test seeds the catalog + IR, decodes the ids exactly as
  `GET /api/v1/skills` does, drills each into the IR GET, and gets `200`). The
  real cause was a **tenant-resolution divergence**: a header-less IR GET fell
  back to `auth.tenant_id` (= nil `00000000-…` under dev-bypass) while
  `GET /api/v1/skills` falls back to the cwd-bound workspace tenant
  (`ensure_workspace`) — the tenant the seed actually wrote under. So the list
  showed 7 skills from the workspace tenant but the IR drill-in looked under the
  nil tenant → `"skill has no IR"`. Fix: `routes_skill_ir_helpers::resolve_tenant`
  now resolves the cwd workspace tenant on a header-less request, identical to the
  list endpoint (`routes_skills_list_get::request_tenant_id`); when the SPA sends
  `X-Argo-Tenant-Id` the header still wins, and the cross-tenant `as_tenant`
  capability gate is unchanged. Touched `argo-cli` only (`routes_skill_ir.rs` +
  `routes_skill_ir_helpers.rs`); `argo-demo-seed`/`argo-skill` untouched. Verify
  (isolated `/tmp/wf-skillir`): `cargo test -p argo-cli --bins skill_ir` 14/14
  PASS (incl. the new `seeded_catalog_ir_is_found_by_catalog_id_via_get` +
  `header_less_get_resolves_workspace_tenant_like_skills_list`, which reproduced
  the live 404 RED then went GREEN); `cargo test -p argo-demo-seed` 34 unit + 4
  integration PASS; `cargo clippy -p argo-cli --bins --tests -- -D warnings` clean.
  Full-live after next gateway rebuild + reseed.
