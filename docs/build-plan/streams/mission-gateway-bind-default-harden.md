# Mission load-bearing wire — gateway bind-default harden + runbook

**Owner**: claude. **Depends on**: `crates/argo-cli/src/handlers/gateway/boot.rs` already implements `check_bind_loopback(bind, insecure_bind)?` + `insecure_bind: bool` flag. The TLS-or-proxy brief (`mission-gateway-tls-termination-or-proxy-doc`) covers the broader bind-non-loopback contract; this brief is the narrower DOCUMENTATION + STARTUP-AUDIT pass for the `127.0.0.1:7400` default.
**Authored against**: origin/main `f1b6b0d1` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-bind-default-harden`.

**Drift-fix amendment 2026-05-13** (vs HEAD 6159b789): NOT a path-drift issue — brief targets are valid. This row is BLOCKED on coordination collision: `mission-gateway-tls-termination-or-proxy-doc` is CLAIMED and BOTH briefs mutate the non-loopback bind contract and `crates/argo-cli/src/handlers/gateway/boot.rs`. Once TLS brief lands (worker picks Path A vs Path B), rebase this brief's flag rename/co-flag/audit behavior on top. Note `boot.rs` is now at 361 LOC (post-split) with headroom — `boot_spawns.rs` is the canonical place for new spawn helpers but this brief's edits stay in `boot.rs::start` proper (bind guard logic, not a spawn).

> SOURCE: MISSION_BACKLOG.md Section 2 #23 ("Document and harden the 127.0.0.1:7400 bind default"). **Tier M (security).** Today the gateway defaults to `127.0.0.1:7400` and `check_bind_loopback` refuses to bind to a non-loopback address unless `--insecure-bind` is passed. The guard exists, but: (1) no audit-chain row is emitted when the operator passes `--insecure-bind` (so a compromise can't be reconstructed from the audit log alone), (2) no runbook documents the security model, (3) `--insecure-bind` is the wrong name — it should be `--bind-non-loopback` (descriptive) and require a co-flag `--accept-insecure-bind-warning` (deliberate). This brief closes those gaps.

## Scope

### Audit-chain emission on insecure boot

EXTEND `crates/argo-cli/src/handlers/gateway/boot.rs`:

- When `insecure_bind=true` AND `bind` is non-loopback, before binding, emit a `Severity::Warn` audit-chain row via the existing audit-emit path:
  - `kind = "gateway_insecure_bind"`
  - `subject = format!("{bind}:{port}")`
  - `actor = "system"`
  - body JSON: `{ bind, port, pid, tls_enabled: bool, bind_source: "cli" | "config" | "env" }`
- The row is emitted BEFORE `TcpListener::bind` so a failed bind still produces a forensic record of the attempt.
- If the audit-chain write itself fails, log + continue (boot should not block on audit-chain hiccup; the stderr "argo gateway: ⚠️  ..." warning is the fallback).
- The existing `eprintln!("argo gateway: ⚠️  ...")` warning stays — audit-chain row is additive.

### Flag rename + co-flag requirement

RENAME `--insecure-bind` → `--bind-non-loopback` (more descriptive — "insecure" overlaps with `--insecure-tls`):

- Add a deprecation-alias for `--insecure-bind` that prints `"--insecure-bind is deprecated; use --bind-non-loopback (and pass --accept-insecure-bind-warning to confirm)"` and exits non-zero if used alone.
- Require a SECOND co-flag `--accept-insecure-bind-warning` whenever `--bind-non-loopback` is passed (forces deliberate two-flag commitment — single-flag typos can't bypass the loopback guard).
- Document both flags in `argo gateway --help`.

### Runbook

NEW `docs/skycot/runbook/gateway-bind.md` (≤ 200 LOC):

- Why the loopback default exists (gateway has no auth in `[gateway].auth_required=false` mode; binding non-loopback without TLS exposes the API surface).
- The default security model: `127.0.0.1:7400` + Argo CLI uses the loopback + only operator-on-the-host can reach.
- How to expose externally (3 modes):
  1. **Behind a reverse proxy (recommended)**: nginx/caddy/traefik front-doors TLS termination + auth. Gateway stays on loopback. Operator runs `argo gateway --bind 127.0.0.1 --port 7400`.
  2. **TLS termination in-process (alternative)**: see `mission-gateway-tls-termination-or-proxy-doc` brief once it lands. Not yet wired.
  3. **Non-loopback bind without TLS (DEVELOPMENT ONLY)**: `--bind 0.0.0.0 --bind-non-loopback --accept-insecure-bind-warning`. The audit-chain row + stderr warning + ops-channel page makes this visible.
- How to verify the bind: `ss -tlnp | grep 7400` shows `127.0.0.1:7400` not `0.0.0.0:7400`.
- How to read the audit-chain `gateway_insecure_bind` row + alert on it.

### Tests

EXTEND `crates/argo-cli/src/handlers/gateway/boot.rs` or its tests module (or NEW `boot_bind_tests.rs` ≤ 200 LOC):

- 3 unit tests: (a) loopback bind succeeds without flag, (b) non-loopback bind without flag refuses (existing behaviour — verify it still works), (c) non-loopback bind WITH both flags emits the audit-chain row + boots.
- 1 deprecation test: `--insecure-bind` alone (no rename + no co-flag) exits non-zero with the deprecation message.

NO Rust changes outside the handlers + new boot_bind_tests. NO migrations. NO config schema change (the flags are CLI-only).

This is NOT in scope: the TLS-termination implementation (separate brief `mission-gateway-tls-termination-or-proxy-doc`); the audit-chain emission infrastructure (already shipped — this brief consumes it); the gateway auth middleware (already shipped); the reverse-proxy config snippets for nginx/caddy/traefik (mentioned by name in runbook but not authored — the operator chooses); IPv6 bind defaults (`::1` is the IPv6 loopback equivalent — this brief is IPv4-only and notes the gap in the runbook).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/boot.rs` (EXTEND, ≤ 80 LOC delta)
- `crates/argo-cli/src/handlers/gateway/boot_bind_tests.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/cli_command_enum.rs` (EXTEND only — add the 2 new flags + alias to existing `gateway start` subcommand)
- `docs/skycot/runbook/gateway-bind.md` (NEW, ≤ 200 LOC)

## Files you must NOT touch

- `crates/argo-store/migrations/**`
- Any other crate's `src/` (audit-chain emission goes through existing API; no new audit-chain API)
- `crates/argo-channels/**` (separate from gateway bind)

## Acceptance criteria

- [ ] `argo gateway start --help` shows `--bind-non-loopback` + `--accept-insecure-bind-warning` + (deprecated) `--insecure-bind`
- [ ] `argo gateway start --bind 0.0.0.0 --insecure-bind` (old single flag) exits non-zero with deprecation message
- [ ] `argo gateway start --bind 0.0.0.0 --bind-non-loopback` alone (missing co-flag) exits non-zero
- [ ] `argo gateway start --bind 0.0.0.0 --bind-non-loopback --accept-insecure-bind-warning` boots + emits `gateway_insecure_bind` audit-chain row
- [ ] Audit-chain row body includes `bind`, `port`, `pid`, `tls_enabled`, `bind_source`
- [ ] Runbook at `docs/skycot/runbook/gateway-bind.md` covers default + 3 expose-externally modes + audit-row schema
- [ ] All 4 new tests pass (`cargo test -p argo-cli --test boot_bind_tests`)
- [ ] `boot.rs` LOC delta ≤ 80; total LOC ≤ 200 in new tests file
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-gateway-bind-default-harden`
- PR title: `gateway: bind-default harden — audit-chain row on insecure-bind + flag rename + runbook (mission-gateway-bind-default-harden)`

## Notes for the implementer

- The audit-chain emit path: look for existing `audit_chain_append` or similar call sites in boot.rs / nearby. Re-use the same API the rest of the boot uses (don't invent a new audit-chain path).
- `bind_source` should be: `"cli"` if `--bind` was on the command line, `"config"` if from argo.toml `[gateway].bind`, `"env"` if from `ARGO_GATEWAY_BIND`. Detect via the existing precedence chain.
- The flag rename: `--insecure-bind` is a clap `#[arg(...)]` somewhere — add an `#[arg(alias = "insecure-bind")]` that triggers the deprecation message rather than silently working. Operators upgrading from older versions need a clear migration message.
- Two-flag requirement: this is intentional friction. The single-flag path is the most common typo target ("oh I'll just use `--insecure-bind` for dev"); the two-flag path makes the operator say "yes, I deliberately accept the insecure-bind warning."
- The runbook should be the FIRST stop for anyone debugging "argo gateway refused to bind to 0.0.0.0." Link from the stderr warning text itself: `"⚠️  Refusing to bind to non-loopback address {bind} — see docs/skycot/runbook/gateway-bind.md"`.
- IPv6 (`::1` loopback) is OUT OF SCOPE. The check_bind_loopback function may or may not handle `::1` correctly today; document the gap in the runbook + a TODO comment in boot.rs. A future brief can extend.
- DO NOT add a `--force-insecure-bind` or `--yes-i-really-mean-it` even-stronger flag. Two-flag is sufficient friction; more flags make the help confusing.
- Operator UX: `argo gateway start --bind 0.0.0.0` should print a one-line tip:
  `"Pass --bind-non-loopback --accept-insecure-bind-warning to override (see docs/skycot/runbook/gateway-bind.md)."`
  rather than a wall of text. The runbook carries the detail.
