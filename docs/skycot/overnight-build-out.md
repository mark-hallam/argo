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
5. Keep going until the queue is exhausted or every remaining item is BLOCKED.

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
- [ ] **T1 · settings/workspace** — BUILD. No store exists → new `workspace_settings`
  table (one generic `(tenant_id, group_key, value_json, updated_at)` row per group)
  via migration v85 + `routes_workspace_settings.rs` (GET/PATCH for workspace /
  providers/policy / notifications/preferences) + flip `settings.ts`. ~150 LOC Rust.
  HIGH (real settings page). *(next build after V1 integrates)*
- [ ] **T2 · worker evals** — routes_evals.rs EXISTS but is MOCK-ONLY; store has
  `skill_reeval_history` (per-SKILL, not per-worker). Build a real query (worker's
  learned skills → their reeval history surfaced as the worker's eval runs) + flip
  `evals.ts`. ~40 LOC change. MEDIUM. **Data-grain noted — if it gets ambiguous, ship
  the skills-join version + mark the per-worker-eval-run table a follow-up.**
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
- [ ] Governance audit-readiness report (gap #1 move A) — read-only assembly.
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
