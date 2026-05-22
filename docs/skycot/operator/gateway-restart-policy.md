# Gateway restart policy — decision doc

**Authored against**: `origin/main` `209a15e11` on `2026-05-22`.
**Status**: SPEC — pending operator pick. Implementation is a follow-up brief.

## The gap, in one paragraph

The running `argo gateway` process on the operator's demo machine is
PID 28198, reporting `schema_version: 70` via `GET /readyz`. The code
on `origin/main` is at `EXPECTED_SCHEMA_VERSION = 76` and ~30 PRs of
post-merge functionality are baked into that binary slug: workflow
rollback, sticky-notes, sub-workflows, the http + code workflow
nodes, the hire-banner. None of it is live. The demo MP4 path (per
`project_v1_demo_mp4_ship_gate`) routes through several of these new
endpoints; every one of them 404s against a v70 gateway. P0 fixes
landing on `origin/main` do not reach the live service until
something restarts it, and right now nothing does.

This doc picks the policy that closes that gap. It is intentionally
short — three options, one recommendation, one rollback path.

## Option A — auto-restart on merge

The conductor's per-PR merge step
(`gh pr merge <num> --squash --delete-branch --admin`, in step 5 of
`docs/build-plan/PROTOCOL_LOOP_PROMPT.md`) is followed by a
restart hook. Two flavours, both within the same option:

- A1 — local pkill + respawn: `pkill -f 'argo gateway'; argo gateway start &` (or equivalent supervisord/launchd call). ~1s downtime per merge.
- A2 — Fly redeploy: `flyctl deploy --remote-only`. ~60-90s downtime per merge, but pulls a fresh image rather than reusing the local debug binary.

**Pros**
- Zero operator action; demo path is always current.
- The merge commit and the live binary diverge by milliseconds, not hours.
- No drift-detection logic to maintain — restart is the source of truth.

**Cons (concrete, not "more reliable")**
- **Mid-demo merge = mid-demo gateway bounce.** The v1 demo MP4 is the ship gate (`project_v1_demo_mp4_ship_gate`). If a worker tab merges its brief mid-recording, the gateway dies under the operator's screen capture.
- **Multi-PR tick = restart storm.** Conductor ticks routinely merge 3-5 PRs sequentially. Per-merge restart means 3-5 bounces in ~30s; per-tick batch restart means the operator hits a "next tick's worth of merges aren't live yet" window.
- **Bad-merge blast radius is immediate.** A clippy-clean PR that nevertheless ships a broken startup path goes live in one second. There's no "fail fast in CI" buffer because GHA is operator-paused (see `DISABLED_WORKFLOWS_POLICY.md`); the local clippy + tests are the only gate.
- **Restart races merge commit replication.** A1's pkill fires before `git push origin main` has finished syncing on the operator's git host; the new gateway boots against the still-old local checkout if launched from the wrong cwd. Solvable, but adds a sequencing constraint to the conductor that wasn't there before.

## Option B — surface drift in the conductor tick prompt

The conductor tick prompt (`PROTOCOL_LOOP_PROMPT.md`) gains a new
step that runs once per tick:

1. `curl -s http://localhost:<gateway-port>/readyz | jq .schema_version` — reads the live gateway's schema version (the endpoint already returns it; see `crates/argo-cli/src/handlers/gateway/routes_health.rs:84`).
2. Compare against `EXPECTED_SCHEMA_VERSION` declared in `crates/argo-store` (on the current checkout).
3. If `delta >= 1`, surface a P0 line at the top of the §9 tick summary: `"🔴 GATEWAY STALE — live vN, code vM (Δ=K migrations). Operator should restart at a safe window."`
4. Operator restarts via existing `argo gateway start` / `flyctl deploy` flow when convenient — not when the conductor decides.

**Pros**
- **Operator owns the restart timing.** Mid-demo bounce risk is zero.
- **Loud, repeating signal.** The drift line appears at every tick (currently ~1h cadence) until cleared. Easy to ignore once is hard to ignore four times.
- **Trivial implementation.** Tick-prompt edit + one curl. Self-rolling-back: delete the prompt section and the signal vanishes.
- **Uses surface already shipped.** `/readyz` already exposes `schema_version`; no new gateway code required.

**Cons (concrete)**
- **Silent gap between ticks.** A merge at T+0 stays invisible to the operator until the next tick fires (~1h). For most of the demo-prep day this is fine; the day of recording, it's a manual `curl /readyz | jq .schema_version` discipline.
- **Requires operator action.** If the operator misses a tick summary, drift can compound over a day. Mitigation: the drift line repeats every tick.
- **Doesn't help fully-autonomous deploys.** Once the project moves to GHA-driven CI + fly auto-deploy, Option B becomes redundant; it's a v1-local fix, not a v2-fleet fix.

## Option C — scheduled bounce

A cron / launchd timer restarts the gateway unconditionally every N
hours. Sub-flavours: hourly, every-6h, nightly.

**Pros**
- Predictable. Simple. Configurable. No tick-prompt edit. No merge-hook edit.
- Catches drift even if the conductor tab is dead.

**Cons (concrete)**
- **Random-restart-during-demo.** Worst-case timing is the moment the operator hits "record" on the demo MP4. Mitigation requires "don't restart between 9am-5pm local" carve-outs, which is just Option B re-implemented in cron config.
- **Doesn't react to merge bursts.** A 5-PR wave at 02:30 with a 03:00 bounce leaves a 30-minute stale window. A 5-PR wave at 03:30 with a 09:00 bounce leaves a 5.5-hour stale window. The cadence is decoupled from when the gap actually appears.
- **Restart costs are paid even when there's no drift.** Idle days bounce the gateway for no reason; uptime metrics get noisier.
- **No drift signal.** The operator can't tell whether the bounce closed a real gap or restarted a healthy process.

## Recommendation

**Option B — surface drift in the conductor tick prompt.**

The deciding factor is the demo MP4 ship gate. The operator is the
demo recorder, the gateway runs on the operator's local machine, and
a mid-recording bounce is the single worst failure mode in v1 — worse
than carrying drift for an extra hour. Option B keeps the operator in
control of "when does the bounce happen" while making "you have
drift" loud and unmissable. The `/readyz` endpoint already exposes
`schema_version`, so the implementation is ~30 lines of tick-prompt
edit + ~5 lines of script: lowest LOC of the three options, lowest
new-failure-mode count, easiest to roll back.

Option A is the right answer once the project moves to a non-local
deployment with GHA-gated CI in front of merges and the operator is
no longer the demo recorder. That's a v1.x → v2.0 conversation, not
v1.0.

Option C is a not-recommended fallback. It solves the problem only at
the cost of a guaranteed mid-demo bounce risk and has no observability
benefit over Option B.

**Follow-up implementation brief:**
- ID: `mission-conductor-gateway-schema-drift-surface`
- Scope: edit `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` to add the
  per-tick drift check (curl + jq + compare against `EXPECTED_SCHEMA_VERSION`),
  add a tiny helper script under `scripts/check-gateway-schema-drift.sh`
  that the tick prompt invokes, document the surfaced format in the §9 summary template.
- Estimated LOC: 30-60 (mostly markdown).

## Rollback

If Option B is shipped and proves more harmful than helpful (false
positives during local-dev checkouts on a different branch, signal
fatigue, etc.):

1. Revert the `PROTOCOL_LOOP_PROMPT.md` edit (one commit; the
   pre-Option-B prompt is reachable via `git log`).
2. Delete `scripts/check-gateway-schema-drift.sh` if it was added.
3. Update this doc with `**Status**: ROLLED BACK on <date> —
   <one-line reason>` and pick a different option.

No data is written; no state needs migrating; nothing in the runtime
depends on the drift check. Rollback is a single PR.

## Operator decision point

This is a **proposal**. The operator may pick a different option,
which produces a different follow-up brief:

- **Operator picks A** → follow-up brief `mission-gateway-auto-restart-hook` (~80-150 LOC: edit the conductor tick prompt's merge step + add a restart helper script + decide between A1/A2 locally).
- **Operator picks B** → follow-up brief `mission-conductor-gateway-schema-drift-surface` (~30-60 LOC).
- **Operator picks C** → follow-up brief `mission-gateway-scheduled-bounce` (~20-40 LOC: cron entry or launchd plist + a tiny wrapper that pkill+respawns).

Pick before the next conductor replenishment pass.
