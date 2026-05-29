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

1. `cd ~/code/argo && git fetch origin -q && git log --oneline -8`
2. Read this file's **Queue** + **Log** below — pick the first `TODO`/`PARTIAL`
   item by priority that isn't `BLOCKED`.
3. Build it (single Rust integrator: `export CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target CARGO_INCREMENTAL=0`).
4. Verify (cargo test/clippy for Rust; vitest/tsc for web). Commit + push ONLY if
   green. Append a Log line. Never break the running demo (gateway :7400 / SPA :3000).
5. Keep going until the queue is exhausted or every remaining item is BLOCKED.

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
- [ ] **V1 · `argo-voice-io` trait redesign + mock adapter** (STT/TTS traits +
  a deterministic test adapter). Buildable now (no key). Rust.
- [ ] **V2 · Voice-loop over the real truth-bounded `pitch-ask`** (STT → pitch
  answer → TTS) + gateway routes. Buildable with the mock adapter; real-vendor
  adapter is **BLOCKED on an operator-provided speech API key**.
- [ ] **V3 · Browser push-to-talk on `/pitch`** — web; depends on V2 routes.

### P1 — Theatre removal (wire real backends / flip mock gates where real)
- [ ] **T-audit · Produce the theatre-removal sub-queue** — audit every
  `web/lib/api` mock surface: which have a live endpoint (flip the gate) vs need
  a backend built. *(delegated to a background audit agent; results appended below)*
- [ ] (sub-items populated by T-audit)

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
