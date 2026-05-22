# mission-gateway-restart-executable-runbook

**Owner**: claude.
**Depends on**: `mission-gateway-restart-on-merge-spec` MERGED (✅ via #2023).
**Authored against**: origin/main `8683d94d1` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-restart-executable-runbook`.

🟡 **P2 OPERATOR-OPS.** Source: auditor T09-40 §4 P2. PR #2023 shipped the policy spec at `docs/skycot/operator/gateway-restart-policy.md` (recommended Option B — surface drift in conductor tick prompt). The next step is the **operator's runnable counterpart**: a one-command way to flip the live gateway from a stale schema_version to current. Currently it's a 5-step sequence with a known footgun (the binary path is `/tmp/argo-shared-target/release/argo`, not `./target/release/argo` — operators learn this the hard way).

This brief converts the procedure into a `make` target + an idempotent shell script.

## Scope

Add a `Makefile` target (or `justfile` if the repo uses just) plus an underlying shell script that performs the full safe-restart sequence:

```bash
# 1. Capture old PID
# 2. SIGTERM gracefully
# 3. Wait for exit (max 10s, then escalate to SIGKILL)
# 4. Rebuild the release binary if source has changed since the running binary's mtime
# 5. Launch the new binary detached via nohup
# 6. Wait up to 30s for /healthz to return 200
# 7. Confirm new schema_version >= code's EXPECTED_SCHEMA_VERSION
# 8. Print before/after schema_version diff
```

Target name: `make restart-live-gateway` (or `just restart-live-gateway`).

Script behavior:
- Idempotent: if no gateway is running, just start one. If one is running but already current, no-op + report.
- Safe across worker pool: pre-restart, prints "may briefly interrupt N active integration tests; abort with Ctrl-C if not safe". Default proceeds after 5s; `--no-prompt` flag skips the prompt.
- Fails loud: any step that doesn't succeed prints a diagnostic line + exits non-zero. Operator sees the failure rather than a silently-broken half-restart.

Also extend `docs/skycot/operator/gateway-restart-policy.md` with a "Quick reference" section at top:
```
$ make restart-live-gateway
[1/8] Capturing old PID...
[2/8] ...
[8/8] ✅ Gateway live at schema_version 77 (was 70).
```

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: scripts/restart-live-gateway.sh` — the underlying script. ~80-150 LOC bash.
- `Makefile` — narrowly scoped: add the `restart-live-gateway` target. If no Makefile exists, NEW Makefile (small).
- `docs/skycot/operator/gateway-restart-policy.md` — narrowly scoped: add the "Quick reference" section at top + a link to the script.
- `NEW: scripts/restart-live-gateway-tests.sh` (or inline) — minimal smoke: no gateway running → start; gateway running + current → no-op; gateway running + stale → restart sequence completes.

## Files you must NOT touch

- The gateway binary's source code (no behavior changes; we're just managing its lifecycle).
- Any worker-loop / conductor / auditor script.
- The build system (no Cargo changes; the script just invokes `cargo build --release` if needed).
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] `make restart-live-gateway` exists and is documented in `gateway-restart-policy.md`.
- [ ] Running it against a stopped gateway → starts a fresh one + confirms healthy.
- [ ] Running it against a running, current gateway → no-op, prints "already at schema_version N (current)".
- [ ] Running it against a running, stale gateway → kills + rebuilds-if-needed + restarts + confirms `schema_version` advanced.
- [ ] Binary path uses `${CARGO_TARGET_DIR:-./target}/release/argo` so it works whether or not `CARGO_TARGET_DIR` is set.
- [ ] Pre-restart 5s warning prompt + `--no-prompt` flag for automation.
- [ ] On any failure step, exits non-zero with a clear diagnostic.
- [ ] Smoke test passes (3 scenarios: stopped, current, stale).
- [ ] Quick-reference section added to the operator-facing policy doc.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-gateway-restart-executable-runbook`
- PR title: `ops(gateway): make restart-live-gateway target + safe-restart script (mission-gateway-restart-executable-runbook)`

## Notes for the implementer

- ~150-200 LOC expected. Mostly bash; the Makefile target is 1 line.
- **The binary path footgun is load-bearing.** Test specifically: with `CARGO_TARGET_DIR=/tmp/argo-shared-target` set, the script must use `/tmp/argo-shared-target/release/argo`. Without it, `./target/release/argo`. Don't hardcode either.
- The "rebuild if source has changed" check: compare `git rev-parse HEAD` to the binary's `mtime` (or to a `.argo/gateway-built-from` sidecar file). If different, run `cargo build --release --bin argo` before launching. Time-budget: 5-15 min on warm cache, but worth it to avoid the May-20-binary problem the operator hit.
- The `/healthz` poll: try every 1s for up to 30s. Most fresh boots take 10-20s.
- If `EXPECTED_SCHEMA_VERSION` from `crates/argo-store/src/lib.rs` is HIGHER than what the gateway reports after restart, something failed silently — exit non-zero. Don't claim success on a stale binary.
- The 5s pre-restart prompt isn't strictly necessary, but it gives the operator a chance to abort if they remember a worker is mid-integration-test. The `--no-prompt` flag is for cron-driven invocation later.
