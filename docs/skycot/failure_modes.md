# Failure modes — what to do when X breaks

Customer-facing reference for the failure modes a SKYCOT
operator will hit during normal operation. Each entry follows
the same shape:

* **Symptom** — what the operator sees in the terminal / artifact
  bundle.
* **Cause** — what's actually happening underneath.
* **Recovery** — concrete commands to get back to a working
  state.
* **Prevention** — config / pattern that reduces recurrence.

If you hit something not covered here, run `argo worker doctor`
first — it surfaces the workspace's current state in one
command and is the right starting point for any unknown failure.

---

## Provider failures (Anthropic / OpenAI 503, timeout, rate-limit)

**Symptom.**

`argo work serve` exits with a `[STOP: NEED_INFO transport
error — retry once the upstream issue clears]` line in the
artifact bundle. The completed_jobs row's `stop_reason` is
`need_info`. No actions emitted.

```
processed job <id> → session <s> → 312 chars (steps=1, stop_reason=need_info)
```

**Cause.** Provider returned non-2xx (503, 429, 502, or a
connection error). The worker's per-step `chat::run_turn`
catches transport errors as `[STOP: NEED_INFO transport error]`
rather than crashing the whole worker.

**Recovery.**

1. Check the provider's status page (Anthropic / OpenAI / your
   chosen backend) — confirm it's their problem, not yours.
2. Wait until the provider clears (usually <10 minutes).
3. Re-queue the job:

   ```bash
   argo work add --vertical <id> --prompt "<same prompt>"
   argo work serve --vertical <id> --once
   ```

   The original `pending_jobs` row is consumed; the failed
   `completed_jobs` row stays in the ledger as audit history.

**Prevention.**

* Use a fallback provider in the vertical's manifest:

  ```toml
  [provider]
  default  = "anthropic"
  fallback = "openai"   # auto-switches on persistent failure
  ```

* Set `--max-tool-rounds` low enough that a single rate-limited
  step doesn't burn the budget. Default 8 is reasonable.

---

## Worker crash mid-step

**Symptom.** `argo work serve` exits non-zero with a panic
trace. No `completed_jobs` row written. The pending_jobs row
stays in the queue.

**Cause.** A panic (often during artifact rendering or
manifest validation), an OOM kill, or SIGKILL.

**Recovery.**

1. Inspect the panic message:

   ```bash
   argo work serve --vertical <id> --once 2>&1 | tail -40
   ```

2. The pending_jobs row is intact — the worker is restart-safe
   by design (see #716's design doc). Either:
   - Fix the underlying cause (manifest typo, OOM → bump
     ulimit) and re-run.
   - Or **drop** the job by appending a synthetic
     completed_jobs row that marks it abandoned (advanced;
     see "Abandoning a stuck job" below).

3. Run `argo worker doctor` to see if any half-written
   artifact bundles are present.

**Prevention.**

* Run `./scripts/dev-safe-test.sh` before any session — it
  catches manifest typos via `argo vertical lint` before the
  worker ever boots.
* For production deployments, run the worker under
  `systemd-run --restart=on-failure` or equivalent so a single
  crash doesn't stop processing.

---

## Worker crash mid-execute

**Symptom.** An action transitions into `executing` (visible
via `argo worker actions list --state executing`) but never
reaches `completed` or `failed`. The per-action bundle at
`.skycot/outputs/<job_id>/actions/<action_id>/` may be partial.

**Cause.** Operator killed the process during execute, or the
process crashed after the state-row was persisted but before
the executor finished writing the bundle.

**Recovery.**

The state-row is canonical. The bundle is just a derivative.
You have two options:

1. **Resume** — manually finish the action offline, then mark
   it complete:

   ```bash
   argo worker actions complete <action_id> \
     --note "completed manually after crash"
   ```

2. **Abandon** — mark it failed; the retry policy (#755) lets
   you re-emit if you want to try again:

   ```bash
   argo worker actions fail <action_id> --reason "worker crashed mid-execute"
   argo worker actions retry <action_id>     # if max_attempts allows
   ```

**Prevention.** Don't kill the worker mid-execute. If you must,
prefer SIGTERM (graceful) over SIGKILL (immediate); the worker
catches SIGINT and exits cleanly between steps.

---

## DB lock / `database is locked` error

**Symptom.**

```
Error: storage error: database is locked
```

**Cause.** Two `argo` processes hit the SQLite store
concurrently. Most operator commands take `IMMEDIATE` locks
that don't conflict, but a long-running `argo work serve` plus
a heavy query (e.g. `argo audit list` with a wide time
window) can race.

**Recovery.**

1. Identify the offender:

   ```bash
   pgrep -fl argo
   lsof ~/.argo/argo.db 2>/dev/null | head
   ```

2. Wait for one to release (most contention clears in <1s).
   For wedged locks (rare), the standard fix is to stop ALL
   `argo` processes and re-run.

**Prevention.** Don't run `argo work serve` and heavy `argo
audit` queries concurrently in the same workspace. Production
deployments should split read-heavy queries to a follower
process via the gateway's read API rather than hitting the DB
directly.

---

## DB corruption

**Symptom.**

```
Error: storage error: file is not a database
```

or

```
Error: storage error: malformed database schema
```

**Cause.** Power loss mid-write, disk error, partial restore
from backup. Rare; SQLite is durable on commit but not
atomic across files.

**Recovery.**

1. **Don't** run any further `argo` commands until you've
   backed up the broken DB:

   ```bash
   cp ~/.argo/argo.db ~/.argo/argo.db.broken-$(date +%s)
   ```

2. Try `sqlite3 ~/.argo/argo.db ".recover" | sqlite3
   ~/.argo/argo.db.recovered` — the SQLite recover command
   pulls valid rows out of a corrupt file.

3. Move the recovered DB into place:

   ```bash
   mv ~/.argo/argo.db ~/.argo/argo.db.broken
   mv ~/.argo/argo.db.recovered ~/.argo/argo.db
   argo workspace init        # validates schema
   ```

4. Run `argo worker doctor` to confirm action ledger state.
   Some recent rows may be lost.

**Prevention.** Run on a journaled filesystem (default on
macOS / Linux). For production, snapshot `~/.argo/` daily —
it's small and the schema is stable.

---

## Disk full

**Symptom.** Operator commands fail with `ENOSPC: no space
left on device`. In autonomous coding sessions, the Bash tool
can't even allocate output files.

**Cause.** `target/` cargo cache + `/private/tmp/{argo,claude}-*`
scratch + `.skycot/outputs/` per-job bundles accumulating.

**Recovery.** See `docs/dev-runtime-guardrails.md` for the
canonical procedure. Quick version:

```bash
# Step out of Claude Code to a regular terminal.
rm -rf /private/tmp/claude-* /private/tmp/argo*
cd ~/code/argo && rm -rf target/
df -h /
```

**Prevention.** Run `./scripts/dev-safe-test.sh` before
autonomous sessions. The disk gate (default 50 GiB free)
catches it pre-flight.

---

## Action retry exhausted (`max_attempts` hit)

**Symptom.**

```
retry refused: action_type 'post_listing' has max_attempts = 2
and this would be attempt #3; pass --force to override (the
override is logged for audit)
```

**Cause.** The action's manifest declares a retry budget; the
chain has reached it.

**Recovery.** Two paths:

1. **Force-retry** if you genuinely think one more attempt
   will work (e.g. you fixed something upstream):

   ```bash
   argo worker actions retry <action_id> --force
   ```

   This logs a `WARN` to stderr that audit reviewers see.

2. **Abandon and re-emit fresh** — let the original action
   stay terminal, queue a new job that'll re-emit:

   ```bash
   argo work add --vertical <id> --prompt "<refined prompt>"
   argo work serve --vertical <id> --once
   ```

**Prevention.** When authoring a vertical, set `max_attempts`
to match how transient the action's failures actually are.
Research/draft actions tolerate 3 attempts; public-surface
posts tolerate 2; spend / bid actions should be 1 (no retry —
operator must explicitly re-queue).

---

## Action stuck `pending` for too long

**Symptom.** `argo worker actions list --state pending` shows
rows older than your operator review SLA. Nothing's blocking
them — they just need approval.

**Cause.** Operator backlog. SKYCOT doesn't auto-approve
anything (by design — that's the safety boundary). If
no operator processes the queue, it grows.

**Recovery.**

```bash
argo worker actions render
cat .skycot/approval_queue.md           # full queue review
argo worker actions approve-batch <id1> <id2> ...   # bulk approve safe ones
```

**Prevention.** Set up a daily review reminder. The
launch_readiness doc's pre-launch checklist
recommends `cat .skycot/approval_queue.md` as a daily habit.

---

## Concurrent workers / two `argo work serve` instances

**Symptom.** Two workers race for the same `pending_jobs` row;
one wins (writes `completed_jobs`), the other crashes with a
unique-constraint or "already processed" error.

**Cause.** `argo work serve` doesn't lock the queue; it relies
on the dedup-by-`pending_job_id` semantics which is
restart-safe but not concurrent-safe in the strict sense.

**Recovery.** Stop one of the workers; the other completes its
current job cleanly.

**Prevention.** Run a single `argo work serve` per
vertical-per-workspace. For multi-vertical setups, one serve
per vertical is fine (memory rows are vertical-scoped).
Production deployments use a process supervisor (systemd, k8s
Deployment with replicas=1) to enforce singleton.

---

## Vertical fails to load

**Symptom.**

```
Error: vertical '<id>' not found in /Users/.../verticals — run
`argo vertical list` to see what's loaded
```

**Cause.** Manifest is malformed, deny-listed, or the
verticals dir isn't discoverable.

**Recovery.**

1. Confirm the dir:

   ```bash
   echo $ARGO_VERTICALS_DIR
   ls $ARGO_VERTICALS_DIR     # or `ls verticals/` from the repo
   ```

2. Lint the manifest:

   ```bash
   argo vertical lint <id>
   ```

3. If lint passes but load still fails, try `argo vertical
   show <id>` for the structural error.

**Prevention.** `./scripts/dev-safe-test.sh` runs lint as
part of step 14; CI catches manifest issues before ship.

---

## Abandoning a stuck job (advanced)

If a `pending_jobs` row is irrecoverable (e.g. references a
vertical that's been removed), append a synthetic
`completed_jobs` row to mark it abandoned:

```bash
argo memory append memory.md "$(cat <<EOF
{
  "schema_key": "completed_jobs",
  "pending_job_id": "<the-id>",
  "vertical_id": "<vertical>",
  "stop_reason": "abandoned",
  "stop_detail": "vertical removed; manual abandonment",
  "completed_at": "$(date -u +%FT%TZ)",
  "steps_completed": 0,
  "actions_emitted": 0
}
EOF
)" --vertical <vertical>
```

The dedup logic skips `pending_jobs` whose IDs appear in
`completed_jobs`, so this gets the row out of the way.

This is a sharp tool — use it only when the alternatives
fail. Don't abandon jobs to skip their action emissions; if an
action shouldn't be emitted, fix the prompt or the manifest
instead.
