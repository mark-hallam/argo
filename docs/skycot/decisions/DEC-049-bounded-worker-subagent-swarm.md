# DEC-049 — Workers may spawn a bounded subagent swarm (Subagent-only, default-off)

- **Status**: active
- **Date**: 2026-06-11
- **Operator-locked**: yes ("we want a worker to be able to spawn a swarm where necessary")
- **Supersedes the block on**: `fix-delegate-handoff-test-provision-spawn-capable-worker`
  (the claim-2 delegate tests were RED because no worker could be granted the
  `Spawn{Subagent}` capability `delegate_task` requires — see that brief's
  triage + DEC-041(D)).

## Context

Claim 2 ("Delegation is the unit") needs a worker to be able to hand work to
another teammate (`delegate_task` → a `Spawn{Subagent}` capability). The
DEC-041(D) capability fence correctly fences a worker-bound session to deny-all,
and `WorkerCapabilityProfile::to_capability_set()` only ever emitted
`ToolDispatch` grants — never `Spawn`. So **no worker, under any manifest, could
delegate**, and the claim-2 regression tests panicked (`0 handoffs written`).

The operator's requirement: a worker should be able to spawn a swarm of helper
teammates when the work calls for it.

## The first-principles distinction (why this is safe)

`SpawnKind` has four variants. They are NOT equal:

- **`Subagent`** — start a CHILD ARGO TEAMMATE (`delegate_task`). The child is
  itself capability-fenced by its OWN operator-set manifest, tenant-scoped,
  audited. **This is the swarm. Safe by construction.**
- **`Subprocess` / `Container` / background-process** — start an OS process or
  container on the HOST. This is the host-escape vector DEC-041(D) exists to
  stop.

A worker delegating to another fenced teammate is categorically different from a
worker reaching the host. Granting the first while still denying the rest gives
swarms without weakening the fence's real purpose. **No privilege laundering:**
each spawned teammate is fenced by its own manifest (operator-configured), so a
parent cannot make a child more powerful than the operator intended — delegation
routes work to a teammate that already holds the needed capability; it does not
transfer the parent's capabilities to the child.

## Decision

Workers MAY be granted `Spawn{Subagent}` — and ONLY `Subagent` — under a
**bounded swarm spine**, all default-off:

1. **Subagent-only.** A `WorkerCapabilityProfile` can emit `Spawn{Subagent}`.
   It NEVER emits `Spawn{Subprocess}` / `{Container}` / background-process —
   those stay `owner`/`admin`-only, always.
2. **Opt-in via the existing manifest mechanism.** A vertical grants delegation
   by listing `delegate_task` in its manifest `[tools].allowed`. No new manifest
   schema; the same grant path as any other tool.
3. **Global default-off flag `ARGO_WORKER_SWARM`.** Even a manifest that allows
   `delegate_task` does NOT get `Spawn{Subagent}` unless the runtime flag is on.
   Operator-only flip after a blast-radius review (same posture as
   `ARGO_DUTY_AUTOFIRE`).
4. **Depth cap** (`MAX_DELEGATION_DEPTH`, default 4), read off the existing
   escalation chain (`inherit_active_chain`). A `delegate_task` beyond the cap is
   refused. Stops infinite recursion.
5. **Fan-out cap** (`MAX_DELEGATION_FANOUT`, default 8) — max active outbound
   handoffs per delegating session. Stops one worker exploding into hundreds.
   Depth × fan-out makes the whole swarm FINITE by construction
   (4 × 8 → bounded), the anti-fork-bomb guarantee.
6. **Tenant containment** — a delegation handoff is written under the parent's
   `tenant_id`; a worker can never spawn into another tenant (verified by the
   per-PR adversarial review).
7. **Audited** — every delegation is a recorded handoff (and, post-#2805, the
   audit hash chain it threads through is fork-proof).

The depth + fan-out guards run ALWAYS (independent of the grant flag), so the
bounds are in force the moment the grant is ever enabled.

## Staged rollout (bounds before grant)

- **Stage 1 (this DEC's brief):** the spine above — profile can emit
  `Spawn{Subagent}`, manifest opt-in, `ARGO_WORKER_SWARM` flag, depth + fan-out
  guards, tests, and the claim-2 delegate tests provisioned via the real path
  (closing the superseded brief + removing its KNOWN_FLAKES entry). Ships
  default-off; the flag flip is operator-only.
- **Stage 2 (follow-up):** a swarm-wide shared BUDGET ceiling inherited from the
  root teammate (cost guard on top of the finite depth×fan-out bound), and an
  optional per-vertical delegation allow-list (which child verticals a parent
  may spawn). Not load-bearing for safety in Stage 1 because depth×fan-out
  already bounds the swarm and each child is independently fenced.

## ⛔ BEFORE `ARGO_WORKER_SWARM` IS FLIPPED ON IN PRODUCTION (adversarial review of #2806)

Stage 1 ships SAFE default-off: the irreversible risks — a worker escaping to the
HOST (`Spawn{Subprocess}`/`{Container}`/`{BackgroundProcess}`) or reaching another
TENANT — are **structurally impossible** (Subagent-only via exact-equality
capability covers; handoffs tenant-scoped). But the adversarial review found the
**anti-runaway bounds are not yet fully load-bearing**, so these are BINDING
preconditions on the operator flag-flip (NOT on merging #2806, which is off):

1. **Depth bound is conditional on the spawn wiring.** `swarm_bounds_refusal`
   measures depth off the escalation chain, which only inherits across hops if the
   spawned child runs in a session that carries the parent handoff id. That real
   child-execution path "lands separately" (it is NOT in #2806), so today the
   depth cap is exercised only by the escalation walker, not a recursive-execution
   path. The Stage-2 spawn wiring MUST carry chain inheritance across the spawn
   boundary AND add an end-to-end depth test, or `MAX_DELEGATION_DEPTH` is a no-op
   and the "depth × fan-out → finite" guarantee does not hold. **Until then the
   global finiteness claim is CONDITIONAL, not proven.**
2. **Fan-out has a TOCTOU race.** The active-count read and the handoff insert are
   not atomic, so a burst of concurrent `delegate_task` calls can overshoot
   `MAX_DELEGATION_FANOUT` (bounded by concurrency width, not infinite). Make the
   cap atomic with the insert (count-and-insert in one transaction) before flag-on.
3. **Target tenant validation.** The delegated `worker_id` is not validated to
   belong to the caller's tenant. It cannot cause cross-tenant EXECUTION (the
   handoff lives under the caller's tenant and consumption is tenant-scoped), but a
   mis-aimed target produces an orphaned handoff. Add an explicit same-tenant check
   so it fails loudly.

These three + the end-to-end depth test and a fan-out TOCTOU test are the Stage-2
gate. The conductor/operator MUST NOT set `ARGO_WORKER_SWARM=1` in production until
all land and are adversarially re-verified.

## Recovery / reversibility

Default-off; the grant is inert until `ARGO_WORKER_SWARM=1`. Reverting the flag
disables all worker delegation instantly without a code change. The bounds
remain even when the flag is on.

## Verifier

- A worker whose manifest allows `delegate_task`, with `ARGO_WORKER_SWARM=1`,
  gets `Spawn{Subagent}` in its capability set and CAN delegate (claim-2 tests
  pass).
- The same worker with the flag OFF gets NO `Spawn` capability (delegate denied).
- A delegation chain deeper than `MAX_DELEGATION_DEPTH`, or wider than
  `MAX_DELEGATION_FANOUT`, is refused.
- A worker NEVER gets `Spawn{Subprocess}` / `{Container}` from any manifest.
