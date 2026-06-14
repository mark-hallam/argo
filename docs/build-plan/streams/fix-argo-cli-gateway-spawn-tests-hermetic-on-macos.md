# fix-argo-cli-gateway-spawn-tests-hermetic-on-macos

**Authored against:** origin/main `079afccf0` on 2026-06-15
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-argo-cli-gateway-spawn-tests-hermetic-on-macos`
**Priority:** P3 (test-coverage / CI-substitute hardening — NOT a product or ship blocker).

## Problem (2026-06-15 full-workspace e2e finding)

A full `cargo test --workspace` on macOS produced **10,114 passed / 181 failed / 0
compile errors / 0 core-logic-crate `--lib` unit-test failures**. ALL 181 failures
are integration tests: **74 in argo-cli** (gateway-spawn `--test` targets), 2 in
argo-sandbox (Docker), 1 in argo-config (keychain mock). They fail on a dev
machine because they spawn a real gateway/binary that hits the macOS Keychain (the
spawned child does not inherit `SKYCOT_DB_KEY_BASE64`; the shared harness
`crates/argo-cli/tests/common/mod.rs` then fails to bootstrap — panics at
`common/mod.rs:199`/`:238` + "authentication required" assertion failures), need
Docker running, or are parallelism-sensitive. They pass in a proper integration
environment but NOT in a headless bulk dev-machine run.

**Why it matters:** the DEC-055 main-verify post-merge sweep deliberately runs only
the curated *hermetic* critical set, because these 74+ integration tests can't run
green hermetically — so they are **NOT guarded against a silent regression**. With
CI permanently off (DEC-055), that is a real coverage gap.

## Fix — make the gateway-spawn harness hermetic on macOS

- Propagate `SKYCOT_DB_KEY_BASE64` (or an ephemeral per-run key) to the **spawned
  gateway child** so it never blocks on the Keychain (see the keychain-hang note in
  auto-memory `project_gateway_spawn_tests_keychain_hang`).
- Make the `tests/common/mod.rs` bootstrap deterministic: the spawned-gateway
  auth/CSRF/session setup must not 401 (`Authorization: Bearer {mint_jwt}` for setup
  requests, NOT a session cookie — a cookie trips the CSRF gate, per #2829).
- Gate the Docker-dependent argo-sandbox `docker::tests::*` to **skip cleanly** when
  Docker is unavailable (not fail).
- Gate/mock the argo-config keychain test so it doesn't require a real Keychain.

## Verifier (must be RUN on macOS, not just compile)

- `cargo test -p argo-cli` runs green on a clean macOS dev machine (no Docker, no
  Keychain grant) — or env-dependent tests **skip cleanly with a documented reason**,
  never fail.
- The newly-hermetic critical subset is added to
  `scripts/main-verify-critical-tests.txt` so the post-merge sweep guards it (the
  whole point — close the DEC-055 coverage gap).

## Out of scope
- Do NOT weaken what the tests assert. The goal is hermeticity (deterministic setup
  + clean environment skips), not deletion.

**LOC budget:** ~150–400 (mostly the shared harness `common/mod.rs` + per-test
environment gating).
