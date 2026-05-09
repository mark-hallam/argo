# Graceful shutdown on SIGTERM

**Owner**: claude. **Depends on**: none.

> SOURCE: production hardening Phase C. The gateway today exits
> hard on SIGTERM — in-flight requests get RST, the store may
> close mid-transaction, and orchestrators (k8s, systemd) see
> failed health checks during rolling deploys. This brief adds
> graceful shutdown: SIGTERM stops accepting new connections,
> drains in-flight requests with a deadline, then closes the
> store cleanly.

## Scope

On SIGTERM (or SIGINT in dev), the gateway:

1. Stops accepting new TCP connections (axum's
   `with_graceful_shutdown` hook).
2. Returns 503 with body `{"error": "shutting_down"}` for any
   request that arrives after step 1 but before connections
   fully drain (race window).
3. Waits up to `ARGO_SHUTDOWN_DEADLINE_SECS` (default 30) for
   in-flight requests to complete. Logs the count of in-flight
   requests every 5s during drain.
4. After deadline OR all-in-flight-done, calls
   `ArgoStore::close()` (which checkpoints WAL, releases the
   lock file, etc. — verify the store has a close path; if
   not, add `close()` as a narrow extension).
5. Exits 0.

Hard timeout: if step 3 hangs > deadline + 5s, force-exit with
status 124 and a final log line.

Behind a feature flag? **No.** Graceful shutdown is correct
default behavior; the previous hard-exit was a bug, not a
feature.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/shutdown.rs` (NEW,
  ~150 LOC) — `ShutdownController` struct holding the
  `Notify` + in-flight counter; SIGTERM/SIGINT handler;
  drain loop with deadline; force-exit fallback
- `crates/argo-cli/src/handlers/gateway/boot.rs` —
  narrowly: spawn the SIGTERM/SIGINT handler in
  `spawn_bg_tasks`; wire the shutdown signal into the
  accept-loop's `tokio::select!` so new connections stop being
  accepted on signal; call `ArgoStore::close()` after drain.
- `crates/argo-cli/src/handlers/gateway/routes.rs` —
  narrowly: wrap dispatch in an in-flight-counter increment/
  decrement (drop guard pattern). The gateway is **hand-rolled
  TcpStream + manual chunked HTTP**, NOT axum — no
  `with_graceful_shutdown` or tower middleware applies. The
  per-request guard goes at the top of `dispatch_route`.
- `crates/argo-store/src/lib.rs` — narrowly (only if needed):
  expose `close()` if not already present. Quick check first.
- `crates/argo-cli/tests/shutdown_e2e.rs` (NEW) — 3 tests:
  in-flight request completes before exit, post-shutdown
  request gets 503, deadline-exceeded forces exit

## Files you must NOT touch

- Tokio runtime config — the existing runtime handles signals
  fine; just add the handler.
- Provider crates — they're called from in-flight requests;
  drain handles them transparently.
- Migrations — store close is read-only re schema.

## Acceptance criteria

- [ ] SIGTERM triggers graceful shutdown, not hard exit
- [ ] In-flight requests complete (within deadline)
- [ ] New connections after SIGTERM get 503 with documented body
- [ ] Drain progress logged every 5s
- [ ] `ArgoStore::close()` called on clean shutdown
- [ ] Force-exit (status 124) fires on hung drain
- [ ] `ARGO_SHUTDOWN_DEADLINE_SECS` env honored (default 30)
- [ ] 3 e2e tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-graceful-shutdown-handling`
- PR title: `gateway: graceful SIGTERM shutdown with drain (claude-graceful-shutdown-handling)`

## Notes for the implementer

- `tokio::signal::unix::signal(SignalKind::terminate())` for
  SIGTERM; `ctrl_c()` for SIGINT in dev.
- The gateway uses a hand-rolled `TcpListener::accept()` loop in
  `boot.rs` — wrap the loop in `tokio::select!` between
  `listener.accept()` and the shutdown signal future so the
  loop returns cleanly on signal.
- For the in-flight counter, use a drop-guard struct (e.g.
  `RequestGuard` that decrements an `Arc<AtomicUsize>` on
  Drop) created at the top of `routes::dispatch_route` — the
  guard's Drop is exception-safe so no path leaks the counter.
- DO NOT introduce a separate `--graceful` flag; the new
  behavior is the only behavior. Document it in the runbook
  changelog instead.
