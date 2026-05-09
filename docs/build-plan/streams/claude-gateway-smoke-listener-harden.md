# claude-gateway-smoke-listener-harden — fix the listener-startup race that flakes gateway smokes

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — gating real work.**

**Authored against**: origin/main 3474fb91 on 2026-05-09 (conductor-authored, post-#1079 systematic flake observation).

## Why

Gateway smoke / e2e tests share a listener-bootstrap helper at
`crates/argo-cli/tests/common/mod.rs:177` that polls for "listener
announce" with a hardcoded 2-5 second deadline. Under any meaningful
machine load (parallel cargo runs, other tabs compiling, even just a
loaded laptop) the helper races and fails with:

```
listener did not announce in 2s
```

Confirmed across multiple PRs as a real bug, not a transient flake:

- **#1057** (bearer-auth-principal-stamping): originally failed with
  `listener did not announce in 2s` at common/mod.rs:177. Held for
  weeks before workers force-pushed a workaround.
- **#1079** (security-csp-headers): failed 3-of-3 retries on the
  conductor's loaded machine. Implementation correct (6/6 unit tests
  pass); only the smoke is fragile. Currently held IN_REVIEW.
- **#1080** (graceful-shutdown-handling): failed 1-of-2 retries with
  the same signature; second retry passed. Merged with caution.
- **#960** (gateway-bearer-auth-get-routes): noted in self-exit logs
  as boundary flake during earlier work.

Every gateway-touching PR is at risk. We've been treating each
recurrence as a per-PR flake. It is not — it is a structural bug in
the shared test helper, and shipping one fix here unblocks all current
and future gateway smokes.

## Approach (concrete)

The current pattern (paraphrased from the failure trace):

```rust
// crates/argo-cli/tests/common/mod.rs near line 177
let deadline = Instant::now() + Duration::from_secs(2);
while Instant::now() < deadline {
    if check_listener_alive(port) { return Ok(port); }
    sleep(Duration::from_millis(50)).await;
}
panic!("listener did not announce in 2s");
```

Three structural problems:
1. **Hardcoded 2s deadline** — no environment-knob to extend under load.
2. **No retry-on-bind-fail** — if the gateway process crashes during
   startup (e.g., port already bound), the helper polls until deadline
   then fails with a misleading "did not announce" message.
3. **Polling rather than channel-receive** — the gateway already knows
   when its listener is bound; pushing that signal back via a oneshot
   channel removes the race entirely.

### Fix shape

Replace the polling helper with a channel-receive helper:

```rust
// New signature
pub async fn spawn_gateway_with_listener_signal(
    config: GatewayConfig,
) -> Result<(GatewayHandle, u16)> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let handle = tokio::spawn(async move {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        tx.send(port).ok();  // signal: listener bound, ready
        run_gateway(listener, config).await
    });

    // Configurable deadline; default 10s; override via env
    let deadline = std::env::var("ARGO_TEST_LISTENER_DEADLINE_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);
    let port = tokio::time::timeout(
        Duration::from_millis(deadline),
        rx,
    ).await
        .map_err(|_| anyhow!("listener did not announce in {}ms (set ARGO_TEST_LISTENER_DEADLINE_MS to extend)", deadline))?
        .map_err(|_| anyhow!("gateway crashed before listener bound (no signal received)"))?;

    Ok((handle, port))
}
```

Key changes:
- **Default deadline raised from 2s → 10s.** 5x headroom for loaded
  machines. Still fast enough to catch real hangs.
- **Configurable via `ARGO_TEST_LISTENER_DEADLINE_MS`.** CI can extend
  to 30s if needed; nimble dev loops can shrink to 2s.
- **Channel-based, not poll-based.** Eliminates the race entirely —
  the listener tells us when it's ready, we don't guess.
- **Distinguishes "deadline exceeded" from "process crashed".** The
  oneshot `Receiver` returns `Err` if the sender is dropped (gateway
  died), surfacing the real failure instead of a misleading timeout.
- **Backward-compatible signature.** Existing `spawn_gateway(...)`
  helpers can wrap the new one and silently get the benefit.

### Migration plan

1. Add the new helper alongside the old one in `common/mod.rs`.
2. Migrate the existing `spawn_gateway()` to delegate to the new helper
   (preserves callers).
3. Add a deprecation note to the polling-based helpers if any remain.

## Files you may touch

- `crates/argo-cli/tests/common/mod.rs` — narrowly: replace listener-poll
  helper with channel-receive helper; update `spawn_gateway` to use it
- `crates/argo-cli/tests/common/listener_signal.rs` (NEW, ≤ 150 LOC) —
  the channel-based helper if it's cleaner as a sibling
- `crates/argo-cli/Cargo.toml` — narrowly: add `anyhow` to `[dev-dependencies]`
  if not already present (likely is)

## Files you must NOT touch

- The actual gateway boot logic in `crates/argo-cli/src/handlers/gateway/boot.rs`
  — listener acquisition is already correct; only the test helper is broken
- Any PR's smoke test file (`tests/gateway_*.rs`) other than to switch
  call sites to the new helper if signature changes
- Any non-gateway test helper

## Acceptance criteria

- [ ] New channel-receive helper compiles and passes unit tests
- [ ] Existing `spawn_gateway()` call sites continue to work (backward-
      compatible signature, or all sites updated in this PR)
- [ ] Default deadline 10s (not 2s)
- [ ] `ARGO_TEST_LISTENER_DEADLINE_MS` env var honored
- [ ] All currently-failing smoke tests pass on a loaded conductor
      machine after this lands. Re-run on these in particular:
  - `crates/argo-cli/tests/gateway_security_headers_smoke.rs` (#1079)
  - `crates/argo-cli/tests/shutdown_e2e.rs` (#1080)
  - `crates/argo-cli/tests/gateway_auth_principal_smoke.rs` (#1057)
- [ ] Helper distinguishes "deadline exceeded" from "process crashed"
      in the error message
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean (or boundary-noted pre-existing flakes)
- [ ] Every modified file ≤ 500 LOC

## Branch + PR

- Branch: `claude-gateway-smoke-listener-harden`
- PR title: `cli(test-helper): channel-based gateway-listener signal (claude-gateway-smoke-listener-harden)`

## Notes for the implementer

- Pattern-match on the existing `spawn_gateway` shape — the channel
  pattern is identical to what `argo-channels` does for connector
  health-checks. Read `crates/argo-channels/src/health.rs` (~200 LOC)
  for a working analogue.
- The `tokio::sync::oneshot::Receiver` returns `Err(RecvError)` if the
  sender is dropped before send. That's exactly the "gateway crashed"
  signal — use it.
- Do not bump default deadline higher than 10s without justification.
  Long deadlines mask real hangs. 10s is the empirical sweet spot
  observed across the affected smokes (none ran longer than 8s on
  successful retries; 5x of that is 40s but that's overly conservative).

## Unblocks

- **#1079** claude-security-csp-headers — currently held IN_REVIEW
  with 3/3 smoke fails on the conductor machine. Lifts hold.
- All future gateway-touching PRs (rate-limit / auth-routes / shutdown
  variants / health-check endpoints) — same pattern was going to bite
  each one.
- `claude-rate-limit-per-route` (#1077) is gateway-adjacent and likely
  to inherit the same fragility once tested under load.
