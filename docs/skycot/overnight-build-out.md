# Autonomous platform build-out — master tracker + resume anchor

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
  **Real ElevenLabs/Deepgram adapter still BLOCKED on operator speech API key.**
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
- [ ] Self-improvement Step 1b (eval-case generation) — additive, no key.
- [x] Governance audit-readiness report (gap #1 move A) — read-only assembly. **DONE** (commit `764f725ff`).
- [ ] Orchestration supervisor engine — **BLOCKED: operator to confirm Supervisor vs DAG.**
- [ ] (voice real-vendor — see V2, BLOCKED on key)

### P3 — Mission-completion + hardening
- [ ] (populated as P0–P2 land + from FEATURE_MATRIX 🟡/🟧 rows)

## Blockers needing operator input (parked, not abandoned)
- **Voice real vendor + API key** (V2 real adapter) — recommend ElevenLabs+Deepgram.
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
