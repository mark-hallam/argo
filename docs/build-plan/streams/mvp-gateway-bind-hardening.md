# mvp-gateway-bind-hardening — refuse non-loopback bind without `--insecure-bind` flag

**Owner**: claude. **Depends on**: nothing. **Tier P (production-readiness, MVP).**

**Authored against**: origin/main 5baf15a6 on 2026-05-10.

## Why

`argo gateway` currently binds whatever address is passed via `--bind`
without checking. Operators who pass `--bind 0.0.0.0` (intentionally or
by copy-paste from a tutorial) expose the gateway publicly with no
TLS and only the recently-landed bearer-auth middleware between the
internet and the agent. MISSION_BACKLOG item #23 calls for "document
and harden the 127.0.0.1:7400 bind default."

Hardening rule: refuse to start if `bind` resolves to anything other
than `127.0.0.1` / `::1` / a loopback range UNLESS the operator passes
an explicit `--insecure-bind` flag. The flag exists so operators
running behind a reverse proxy can opt in deliberately.

## Approach (concrete)

1. **Add `--insecure-bind` boolean flag** to the gateway start
   subcommand (look at how existing booleans like `--profile-startup`
   are wired in `crates/argo-cli/src/cli.rs` / handler dispatch).
2. **Inside `start()` in `crates/argo-cli/src/handlers/gateway/boot.rs`**
   (currently 427 LOC, ~70 LOC headroom):
   - After `let addr = format!("{bind}:{port}");`, parse `bind` as
     `IpAddr` (or via `format!("{bind}:{port}").parse::<SocketAddr>()`).
   - If `addr.ip().is_loopback()` → proceed.
   - Else if `insecure_bind` flag set → log a single yellow warning to
     stderr ("argo gateway: binding non-loopback {addr} per
     --insecure-bind; ensure TLS-terminating proxy is in front") and
     proceed.
   - Else → return
     `ArgoError::Provider("refusing to bind non-loopback {addr} \
      without --insecure-bind; see docs/skycot/SECURITY.md".into())`.
3. **Two unit tests** in `boot.rs` (or a sibling `boot_tests.rs` if
   `boot.rs` would cross 500 LOC):
   - `bind_loopback_accepted` — `127.0.0.1` proceeds without flag.
   - `bind_non_loopback_rejected_without_flag` — `0.0.0.0` returns
     Provider error containing the phrase "--insecure-bind".
4. **Documentation:** add a 1-paragraph note in
   `docs/skycot/SECURITY.md` (create if missing — single short file
   under `docs/skycot/`) explaining the loopback-default + opt-in
   model.

## Files you may touch

- `crates/argo-cli/src/cli.rs` (narrowly: 1 new boolean flag on the
  gateway start subcommand)
- `crates/argo-cli/src/handlers/gateway/boot.rs` (parse bind, conditional)
- `crates/argo-cli/src/handlers/gateway/boot_tests.rs` (NEW, only if
  inline tests would push `boot.rs` ≥ 500 LOC)
- `docs/skycot/SECURITY.md` (NEW or APPEND, ≤30 LOC of prose)

## Files you must NOT touch

- Any other source file. The check is purely additive in `boot.rs`.

## Acceptance criteria

- [ ] `argo gateway start --bind 127.0.0.1 --port 7400` works as before.
- [ ] `argo gateway start --bind 0.0.0.0 --port 7400` exits non-zero
      with a clear error mentioning `--insecure-bind`.
- [ ] `argo gateway start --bind 0.0.0.0 --port 7400 --insecure-bind`
      starts and emits a single yellow warning to stderr.
- [ ] 2 new unit tests pass.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] No file crosses 500 LOC.

## Branch + PR

- Branch: `claude-mvp-gateway-bind-hardening`
- PR title: `gateway: refuse non-loopback bind without --insecure-bind (mvp-gateway-bind-hardening)`
