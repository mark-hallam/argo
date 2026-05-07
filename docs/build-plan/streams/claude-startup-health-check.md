# Startup health check before binding

**Owner**: claude. **Depends on**: none.

> SOURCE: production hardening Phase C. The gateway binds the
> listener immediately on startup, then discovers DB / migration
> / secrets problems on the first inbound request — at which
> point the orchestrator already thinks the pod is healthy.
> This brief runs a critical-dependency health check during
> startup and refuses to bind if any check fails.

## Scope

Run a synchronous startup health check before
`axum::Server::bind()`. Checks:

1. **Store reachable** — open `ArgoStore`, run a trivial
   `SELECT 1`, close. Failure: critical, refuse bind.
2. **Migrations applied** — query the schema version, compare
   to the binary's compiled-in expected version. Failure
   modes: behind (binary newer than schema) → critical, refuse;
   ahead (schema newer than binary) → critical, refuse with a
   clear "binary is too old, roll back" message.
3. **Secrets present** — verify required env vars are set:
   `ANTHROPIC_API_KEY` OR `OPENAI_API_KEY` (at least one),
   `ARGO_GATEWAY_BIND` (or its default applies). Failure on
   no provider keys: critical, refuse.
4. **Channels reachable** — for each configured channel
   (Slack, iMessage, etc.), do a "hello" probe (e.g., Slack:
   `auth.test` API call). Failure: warn-only, log and proceed
   (channels are often misconfigured during initial setup; not
   refusing bind is the right call here).

On critical failure: print a clear error to stderr, exit code 78
(`EX_CONFIG`), don't bind.
On all-pass: log a one-line `startup_check ok in <ms>` and
proceed to bind.

A `--skip-startup-check` flag exists for emergency dev cases;
do NOT document it prominently — it's an escape hatch only.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/startup_check.rs` (NEW,
  ~180 LOC) — the check runner, the four checks, the exit-78
  path
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly:
  invoke `run_startup_checks()` before `Server::bind`
- `crates/argo-cli/src/cli.rs` — narrowly: the
  `--skip-startup-check` flag on the gateway start subcommand
- `crates/argo-cli/tests/startup_check_e2e.rs` (NEW) — 5 tests:
  all-pass path, store unreachable, schema-mismatch, no
  provider keys, skip flag bypasses
- `docs/skycot/RUNBOOK.md` — narrowly: extend the
  troubleshooting section with what each exit-78 message means

## Files you must NOT touch

- `argo-store` migration code — the schema-version helper
  presumably exists; if not, add a narrow getter, but don't
  change migration semantics.
- Provider crates — secret presence is checked by env-var
  read, not by issuing real API calls (those happen later).
- Channel crates — the "hello" probe should reuse each
  channel's existing health check helper if present, otherwise
  fall back to "warn, skip" — don't add new probe code per
  channel.

## Acceptance criteria

- [ ] Critical failures exit 78 before bind
- [ ] Channel-failure is warn-only, gateway still binds
- [ ] One-line log on success with elapsed ms
- [ ] `--skip-startup-check` bypasses the four checks
- [ ] Schema-mismatch error message clearly states which
      direction (binary newer vs older)
- [ ] 5 e2e tests pass (use a temp store + env munging)
- [ ] RUNBOOK lists each exit-78 reason
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-startup-health-check`
- PR title: `gateway: startup health check (refuse to bind on critical failure) (claude-startup-health-check)`

## Notes for the implementer

- Exit code 78 (`EX_CONFIG`) is the BSD `sysexits.h` convention
  for "configuration problem" — orchestrators handle it as
  CrashLoopBackoff with no restart spam (paired with
  `restartPolicy: Never` for the init container, or a sane
  backoff for the main container).
- DO NOT make startup-check async-parallel — sequential is
  simpler and these are fast (< 100ms each on a healthy host).
- For schema-version, the compiled-in expected version should
  be a constant in `argo-store::lib`, generated at build time
  from the highest migration filename. If that constant
  doesn't exist yet, that's a separate prep brief — flag it
  but don't fold the work in.
