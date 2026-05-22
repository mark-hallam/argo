# mission-gateway-freshness-conductor-tick

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `6278d31c1` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-freshness-conductor-tick`.

🟡 **P1 OPERATIONAL — gateway-stale-vs-codebase silently breaks demos.** Source: evaluator round-2 (`docs/skycot/evals/scheduled/2026-05-22/gateway-process-stale-vs-codebase.md`):

> "The running gateway works for what it knows, but is silently missing every route added in the last 4 days. ... Suggested concrete: add a `scripts/check-gateway-freshness.sh` that compares `:7400`'s schema_version to `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1` and exits 1 if drift > 2. Conductor reads exit code; appends a 'GATEWAY STALE: v70 vs v76' line to the next tick prompt."

Context: the operator boots `argo gateway start` once. Days later, new routes / new audit kinds / new migrations land via the swarm. The running gateway process is now silently stale — it serves the OLD routes table while the codebase has new ones. Demo recordings fail in subtle ways ("the route should exist! why is it 404?") because everyone forgot the gateway needs a restart.

`make restart-live-gateway` (shipped via #2043) makes the restart safe. But nothing surfaces the drift — a human has to remember to run it.

## Scope

Two changes:

1. **`scripts/check-gateway-freshness.sh`** — sidecar status script.
   - Probes `http://127.0.0.1:7400/health` (or the canonical endpoint — check `routes_health.rs`); extracts `schema_version` from JSON.
   - Reads the latest migration slot via `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -1 | grep -oE 'v[0-9]+'`.
   - Compares: if `gateway_schema < codebase_latest - 2` → exit 1; else exit 0.
   - Output format:
     ```
     gateway_schema=v70 codebase_latest=v77 drift=7
     STATUS: STALE  (drift > 2, restart recommended: `make restart-live-gateway`)
     ```
   - If `/health` is unreachable → exit 2 (gateway not running, distinct from stale).
   - If parse fails → exit 3 (operator error, distinct from drift).
   - All exit codes documented in the script header comment.

2. **Conductor-tick integration** — add to `docs/build-plan/PROTOCOL_LOOP_PROMPT.md`:
   - New step 4.7 between heartbeat (4.6) and PR review (5):
     `bash scripts/check-gateway-freshness.sh > /tmp/gateway-freshness.txt 2>&1 || echo "GATEWAY FRESHNESS: $(cat /tmp/gateway-freshness.txt)"`
   - The conductor reads the exit code; if non-zero, prepends a `🟡 GATEWAY-STALE` line to the tick summary so the operator sees it.

3. **Tests** (smoke) at `scripts/check-gateway-freshness-tests.sh`:
   - Python fake gateway that returns a chosen `schema_version` in `/health`.
   - Test cases: drift=0 → exit 0; drift=3 → exit 1; gateway unreachable → exit 2; bad JSON → exit 3.
   - 4 cases at ~15 LOC each.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: scripts/check-gateway-freshness.sh` (~60-80 LOC bash).
- `NEW: scripts/check-gateway-freshness-tests.sh` (~80-100 LOC bash, follows the pattern of `restart-live-gateway-tests.sh`).
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — narrowly scoped: add step 4.7 (~8 LOC).
- `docs/skycot/operator/gateway-restart-policy.md` — narrowly scoped: add a "Freshness check" section (~20 LOC).
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/routes_health.rs` — the source. This brief is a CHECKER, not a producer. The `/health` shape is the input contract, not under this brief's scope.
- The gateway runtime itself.
- `Makefile` — `restart-live-gateway` already exists; this brief only points at it.

## Acceptance criteria

- [ ] `scripts/check-gateway-freshness.sh` exists, executable.
- [ ] Exit codes per the contract: 0 (fresh), 1 (stale-drift), 2 (unreachable), 3 (parse error).
- [ ] Output line `gateway_schema=vN codebase_latest=vM drift=K` is machine-grepable.
- [ ] `scripts/check-gateway-freshness-tests.sh` covers 4 cases; all pass.
- [ ] `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` step 4.7 added.
- [ ] `docs/skycot/operator/gateway-restart-policy.md` has a "Freshness check" section.
- [ ] shellcheck clean on both scripts.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-gateway-freshness-conductor-tick`
- PR title: `ops(gateway): freshness-drift check + conductor tick integration (mission-gateway-freshness-conductor-tick)`

## Notes for the implementer

- ~150-220 LOC expected. Sidecar script ~70, tests ~90, docs ~30, protocol update ~10.
- **The "drift > 2" threshold is deliberately permissive.** A 1-migration drift might just be a worker that just merged; not worth flagging. Drift of 3+ indicates the operator hasn't restarted in a while.
- **The check is read-only / non-blocking.** Conductor surfaces, doesn't fail. Operator chooses when to restart.
- **`/health` parse must be robust** — if the gateway upgrades its `/health` JSON shape, the check should fail with exit 3 (parse error), not silently say "fresh".
- Cite the evaluator report verbatim in the PR body.
