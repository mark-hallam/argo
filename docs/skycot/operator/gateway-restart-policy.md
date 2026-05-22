# Gateway restart policy — decision doc

**Authored against**: `origin/main` `209a15e11` on `2026-05-22`.
**Status**: SPEC — Option B picked. Runnable counterpart shipped via
`mission-gateway-restart-executable-runbook` (see *Quick reference*
below). First-run bootstrap shipped via
`mission-gateway-env-local-bootstrap` (see *First-run setup* below).

## First-run setup

Setting the gateway up on a fresh machine? Two paths, both safe:

### Path A — `make restart-live-gateway` does it all (recommended)

Just run the normal restart target. On first run (no
`$ARGO_HOME/env.local` and `SKYCOT_DB_KEY_BASE64` unset in the parent
shell), the script auto-bootstraps:

1. Generates a 32-byte base64 `SKYCOT_DB_KEY_BASE64` (verifies the
   decoded length is exactly 32 bytes before writing — no more 42-byte
   "fix-strings" silently rejected by the gateway).
2. Reuses `ANTHROPIC_API_KEY` or `OPENAI_API_KEY` if either is already
   exported in your shell. Otherwise prompts interactively (input
   hidden).
3. Writes `$ARGO_HOME/env.local` with `chmod 600`.
4. Sources the new file inline so the gateway in the same invocation
   picks up the key — no second run, no `source ... &&` prefix
   required.

Every subsequent `make restart-live-gateway` auto-sources
`$ARGO_HOME/env.local` if it exists, so you never need to remember the
`source ~/.argo/env.local &&` prefix again.

### Path B — `make first-run-setup` (explicit)

If you want to bootstrap env.local without launching the gateway in
the same step (e.g. you want to inspect the generated file first):

```
$ make first-run-setup
[0/8] First-run setup: no /Users/you/.argo/env.local and SKYCOT_DB_KEY_BASE64 unset.
      found ANTHROPIC_API_KEY in parent environment; reusing.
      wrote /Users/you/.argo/env.local (chmod 600, DB key decodes to 32 bytes)

      Source it manually with:
          source /Users/you/.argo/env.local && make restart-live-gateway
      Or just re-run 'make restart-live-gateway' — it auto-sources.
✅ --bootstrap-only requested; exiting without launching the gateway.
```

### Path C — copy the template manually

If you'd rather skip the script entirely, copy
[`scripts/env.local.example`](../../../scripts/env.local.example) to
`~/.argo/env.local` and edit the values in by hand:

```
mkdir -p ~/.argo
cp scripts/env.local.example ~/.argo/env.local
chmod 600 ~/.argo/env.local
# ...edit SKYCOT_DB_KEY_BASE64 and ANTHROPIC_API_KEY...
make restart-live-gateway
```

### Recovering from a lost DB key

If `~/.argo/env.local` is deleted or the key inside it is changed
after sessions were written, the SQLCipher sessions DB becomes
unreadable. To recover:

```
mv ~/.argo/sessions.db ~/.argo/sessions.db.cant-decrypt-old-key-lost
rm ~/.argo/env.local      # forces bootstrap to generate a new key
make restart-live-gateway # bootstraps env.local + launches with a fresh DB
```

You lose runtime session history (audit rows, ongoing workflows). All
durable state lives in checked-in files / external providers; the
sessions DB is in-flight runtime state only. Back up
`~/.argo/env.local` if you care about preserving session history
across machine moves.

### Rotating the API key safely

Edit the `export ANTHROPIC_API_KEY='…'` line in `~/.argo/env.local`.
The gateway holds the previous key in memory until the next restart —
no immediate action needed if you just want the new key picked up on
the next bounce. To force the running gateway to switch right now,
restart with `--force` (the no-op-on-schema-current optimization
otherwise leaves the old in-memory key alive):

```
make restart-live-gateway FORCE=1
```

## Quick reference

When the conductor tick prompt flags `🔴 GATEWAY STALE — live vN, code
vM`, run:

```
$ make restart-live-gateway
[1/8] Capturing old gateway PID + pre-restart schema_version...
      found gateway pid=28198 listening on 127.0.0.1:7400
      pre-restart schema_version=70 (expected 77)
[2/8] Comparing live schema_version against EXPECTED_SCHEMA_VERSION...
[3/8] May briefly interrupt active integration tests.
      Press Ctrl-C within 5s to abort...
[4/8] SIGTERM pid=28198 (10s grace)...
      pid 28198 exited after 2s
[5/8] Checking whether the release binary needs a rebuild...
      built-from 4a1c... != HEAD 8683d... — will build.
      running: cargo build --release --bin argo
[6/8] Launching /tmp/argo-shared-target/release/argo gateway start ...
[7/8] Polling http://127.0.0.1:7400/readyz up to 30s...
      /readyz returned 200 after 12s
[8/8] Confirming new schema_version >= EXPECTED_SCHEMA_VERSION...
✅ Gateway live at schema_version 77 (was 70).
```

Underlying script: [`scripts/restart-live-gateway.sh`](../../../scripts/restart-live-gateway.sh).
Smoke tests: `make test-restart-live-gateway` (stopped / current /
stale / `--force` scenarios plus first-run bootstrap, ~10s).

**Flags**:

- `make restart-live-gateway NO_PROMPT=1` — skip the 5s warning (for
  cron-driven invocation later).
- `make restart-live-gateway FORCE=1` — restart even when the live
  schema_version already matches EXPECTED. Use when switching from a
  one-time inline `SKYCOT_DB_KEY_BASE64` to a persistent
  `~/.argo/env.local` key.
- `make restart-live-gateway PORT=7500` — non-default port.
- `make restart-live-gateway HOST=127.0.0.1` — non-default host.

**Common scenarios**:

| State                                  | What the script does                                |
| -------------------------------------- | --------------------------------------------------- |
| first run (no env.local + key unset)   | bootstraps `~/.argo/env.local` with a fresh 32-byte DB key + API key, then launches |
| env.local present                      | auto-sources it (no need for `source ~/.argo/env.local &&` prefix) |
| no gateway running                     | builds-if-stale, launches detached, polls `/readyz` |
| running + already at EXPECTED          | no-op + reports "already at schema_version N" (overridden by `FORCE=1`) |
| running + stale schema_version         | SIGTERM (10s) → SIGKILL → build-if-stale → relaunch |
| binary missing or HEAD has advanced    | runs `cargo build --release --bin argo` first      |

**Binary-path footgun**: the script uses
`${CARGO_TARGET_DIR:-./target}/release/argo`. This matches the swarm
default of `CARGO_TARGET_DIR=/tmp/argo-shared-target`; without that
env var it falls back to `./target/release/argo`. Do not hardcode
either path in operator runbooks — use `make restart-live-gateway`.

**Exit codes**: 0 on success or no-op; non-zero on any step failure
with a clear diagnostic on stderr. The script never returns 0 on a
half-finished restart — the schema_version check is the final gate.

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
