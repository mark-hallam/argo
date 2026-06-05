# sprint-ws1-1c-fix — make the JWT boot-guard bind-aware (close the #2706 hole)

**Owner**: claude or codex.
**Depends on**: PR #2706 (the boot-guard) — this UPDATES #2706's branch, does not supersede it.
**Authored against**: origin/main `0a7ebbbeb` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-1c-fix-bind-aware-production-detection`.

## Why (adversarial review of #2706 found a real hole)
#2706 added the fail-closed JWT-secret boot-guard, but `jwt_guard_active` (`startup_check.rs:236`) gates
it on the **bind-blind** `is_production_mode()`. The actual socket bind comes from the **`--bind` CLI
flag** (`boot_serve.rs:80`), NOT the `ARGO_GATEWAY_BIND` env var (which `is_production_mode()` reads but
which binds nothing). Net effect: a standard production deploy —
`argo gateway start --bind 0.0.0.0 --port 7400` with `[gateway.behind_proxy] expected_proxy=true` and no
`ARGO_GATEWAY_PROFILE=production` export — yields `is_production_mode() == false`, the guard is **skipped**,
and the fail-open hole survives. **The repo's own `scripts/restart-live-gateway.sh:372` binds via
`--bind "$HOST"` and sets neither signal env var**, so it would evade the guard. (Also: the compiled
default host is `127.0.0.1` per `argo-config/src/defaults.rs:183`, so that disjunct is dead.)

The two sibling boot gates already do this correctly: bearer-token (`boot.rs:123`) and enterprise-encryption
(`boot.rs:130/200`) both use `is_production_mode_for_bind(bind)`. This guard is the ONLY production
boot-check using the bind-blind detector.

## Task (work ON PR #2706's branch — do not start a fresh PR)
1. `gh pr checkout 2706`.
2. In `jwt_guard_active` (`startup_check.rs`), replace `is_production_mode()` with
   `is_production_mode_for_bind(bind)` (both live in `routes_auth_production_mode.rs` — :15 vs :21).
3. Thread `bind` through the call chain so it reaches `jwt_guard_active`:
   `init_boot` (`boot.rs:138`, where `bind` is already in scope) → `boot_serve.rs:67 run_startup_checks`
   → `run_startup_checks_inner` → `jwt_secret_boot_guard` → `jwt_guard_active`. Match how the sibling
   gates already receive `bind`.
4. Keep the `auth_bypass` / `ARGO_DEMO_MODE` escape hatches exactly as they are.
5. **Add one regression test**: force production via a **non-loopback `--bind`** (e.g. `0.0.0.0` /
   a public IP) WITHOUT setting the profile env, and assert a missing secret **refuses** (exit/`Err`).
   This is the case the current tests miss (they force production via the profile env only).

## Verify
- `cargo test -p argo-cli` startup-check module GREEN — all existing #2706 tests still pass (the ~35
  loopback integration boots stay green because their bind is loopback → still excluded) PLUS the new
  non-loopback-bind test. clippy clean on touched files (boundary-note the pre-existing
  `await_holding_lock` in `routes_settings_credentials_tests.rs`, untouched).
- Push to the #2706 branch so the PR updates.

## Guardrails
- Do NOT regress the loopback/dev/demo/bypass boots (they must still start without a secret).
- WS1/security → the conductor re-reviews the diff before merge. Do NOT self-merge.

## PR
Updates existing PR #2706 (`sprint(ws1-1c) — security — fail-closed boot-guard`). No new PR.
