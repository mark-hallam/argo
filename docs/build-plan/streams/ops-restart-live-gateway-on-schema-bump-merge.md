# ops-restart-live-gateway-on-schema-bump-merge — auto-restart gateway after schema-bump PRs merge so /health flips 503→200 without operator manual step

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `24cb565b8` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-restart-live-gateway-on-schema-bump-merge`.

## Why this is P3 (ν-r69 — operator-friction loop)

Evaluator round-69 (`ν` finding) names a recurring 12+ hour gap pattern: a schema-bump PR (like #2331) merges and resolves the schema mismatch in code, but the running gateway binary (`/tmp/argo-shared-target/release/argo`) still serves the old `EXPECTED_SCHEMA_VERSION`, returning 503 until the operator manually runs `bash scripts/restart-live-gateway.sh`. On 2026-05-25 this gap was ~12h before T18-08 conductor surfaced it (gateway PID 37695 started 06:02Z, schema-bump merged 18:10Z, gateway still stale at 19:25Z when this brief was authored).

This brief codifies an opt-in conductor-tick automation: after admin-merging a schema-bump PR (signature: PR body or commit message contains `EXPECTED_SCHEMA_VERSION` token, OR `crates/argo-store/src/lib.rs` is in the squashed diff with `+pub const EXPECTED_SCHEMA_VERSION`), append a `🔄 OPERATOR ACTION: restart gateway` line to `docs/skycot/agents/SHARED.md` AND attempt `bash scripts/restart-live-gateway.sh` from the conductor tick IF the gateway is locally running on `127.0.0.1:7400` (else just surface the action). Operator-confirmable, non-destructive.

Round-69 source: `docs/skycot/evals/scheduled/2026-05-25/round-69-pr-2331-expected-schema-version-merged-but-gateway-binary-stale.md`.

## Scope

New file `scripts/auto-restart-gateway-on-schema-bump.sh` — bash, ~40 LOC. Inputs: a merged commit SHA. Behaviors:

1. Run `git show --stat <SHA>` to inspect changed files.
2. If `crates/argo-store/src/lib.rs` is in the diff AND `git show <SHA> -- crates/argo-store/src/lib.rs | grep -E '^\+pub const EXPECTED_SCHEMA_VERSION'` matches, this is a schema-bump merge.
3. Probe `curl -sS --max-time 3 http://127.0.0.1:7400/health` — if reachable (any HTTP status), call `bash scripts/restart-live-gateway.sh` and log to `docs/skycot/agents/SHARED.md` with `[conductor-auto-restart-gateway] post #PR_NUMBER schema-bump merge — restarted local gateway, new /health: STATUS`.
4. If `/health` unreachable, log `[conductor-auto-restart-gateway] post #PR_NUMBER schema-bump merge — gateway not running locally, no-op` (informational, not an error).
5. Idempotent: if the gateway is already FRESH (drift=0), no-op.

Wire from `scripts/conductor-pre-merge-gate.sh`'s post-merge hook OR document it as a manual step the conductor tick runs after `gh pr merge` succeeds — choose whichever is simpler. The brief author has discretion on placement.

**Not in scope**: changing `restart-live-gateway.sh` itself, auto-restarting on EVERY merge (only on schema bumps), running this from a cron (only from conductor tick after admin-merge).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `scripts/auto-restart-gateway-on-schema-bump.sh` — new file.
- `scripts/conductor-pre-merge-gate.sh` OR `scripts/conductor-merge-and-flip.sh` — add invocation after a successful merge (whichever script is the post-merge hook today).
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — add a 1-line bullet under Step 5 documenting the auto-restart on schema-bump merges.

## Files you must NOT touch

- `scripts/restart-live-gateway.sh` — out of scope.
- Any `crates/` file.

## Acceptance criteria

- [ ] `bash scripts/auto-restart-gateway-on-schema-bump.sh <SHA>` correctly identifies #2331's squashed merge SHA `8abb093b1` as a schema-bump (manual smoke test).
- [ ] When invoked against a non-schema-bump SHA (e.g., a docs-only PR), the script no-ops cleanly with exit 0.
- [ ] When the local gateway is unreachable, the script logs informationally and exits 0 (not an error).
- [ ] BUILD_STATUS.md updated with PR number on flip to IN_REVIEW.

## Branch + PR

- Branch: `claude-ops-restart-live-gateway-on-schema-bump-merge` or `codex-ops-restart-live-gateway-on-schema-bump-merge`.
- PR title: `ops: auto-restart local gateway after schema-bump PRs merge (ops-restart-live-gateway-on-schema-bump-merge)`.

## Notes for the implementer

- Detect schema-bump via `git show --name-only <SHA> | grep -qF 'crates/argo-store/src/lib.rs' && git show <SHA> -- crates/argo-store/src/lib.rs | grep -qE '^\+pub const EXPECTED_SCHEMA_VERSION'`. Both clauses must hold.
- `scripts/restart-live-gateway.sh` is operator-facing and may prompt or take a while — wrap with `--max-time 60` timeout or run with `nohup` and check `/health` after.
- Mind concurrent ticks: if two conductor ticks both try to restart the gateway, the second should detect "drift=0 already" and no-op. Probe `check-gateway-freshness.sh` first.
- Fast-mode candidate. ~40 LOC bash.
