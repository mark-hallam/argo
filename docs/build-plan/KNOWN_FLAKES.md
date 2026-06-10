# Known pre-existing test flakes

If `cargo test` or `pnpm -C web test` fails on a test in this list AND
that test is OUTSIDE your brief's may-touch list, push the PR with a
"Boundary note" naming the exact failing test id. Do NOT mark BLOCKED
— see HARD RULE 2 in `SWARM_PROTOCOL.md`.

When a fix PR lands, remove the entry. Don't let this list rot.

Last refreshed: 2026-06-10 against `origin/main` `78b049318` (refresh-known-flakes-and-track-env-lock).

## How to verify a flake claim

Before declaring a boundary-note exemption, confirm the test fails on
clean `origin/main` without your changes. Two recipes — one per
language. Both should be run from a worktree pinned to `origin/main`
or with your branch's changes stashed.

**Web (vitest):**

```bash
cd web
pnpm install --frozen-lockfile   # only on cold checkout
pnpm test --run 2>&1 | tee /tmp/vitest.log
# Failing tests appear as lines starting with "   × " or "❯ … failed"
grep -E "^( *× |❯ .*failed)" /tmp/vitest.log
```

A full web vitest run cold takes ~5 min wall clock (383 `*.test.ts(x)`
files under `web/`, per `find web -name '*.test.ts*' -not -path
'*/node_modules/*' | wc -l` on 2026-06-10). Warm cache (`.next/`,
`node_modules/.vite/`) ~2-3 min.

**Rust (cargo):**

```bash
# Workspace-wide (~28 min cold, ~9 min warm under a shared target dir):
cargo test --workspace 2>&1 | tee /tmp/cargo.log
grep -E "^(test result|FAILED|---- )" /tmp/cargo.log

# Or crate-scoped — much faster (~30s-2min per crate):
cargo test -p argo-cli 2>&1 | grep -E "^(test result|FAILED|---- )"
```

Each entry below tags its crate so reviewers can re-run targeted with
`-p <crate>` rather than the full workspace.

If a test in this list now passes, remove the entry. If a test NOT in
this list fails, either (a) it's a genuine break — fix it, or (b)
it's a new flake — add an entry here in a separate doc-only brief.

## Active flakes

### Web (vitest) — `pnpm -C web test --run`

None currently active. The three stable web failures listed here on
2026-05-22 (a11y-contrast `#71717a` guard, a11y-headings `accept-invite`
h1-count, brand-execution-polish hardcoded-easing) have all been FIXED on
current main and moved to the Resolved section below — re-verified
2026-06-10 (`npx vitest run` on the three files: 16/16 pass).

### Rust (cargo) — `cargo test --workspace`

- `argo-cli > setup_steps::tests::vertical_step_skips_empty_registry`
  — First observed: 2026-05-09 (`886d9a6bd`). Classification: flake
  (env leak — sensitive to `ARGO_VERTICALS_DIR` set by sibling tests
  in the same binary). Already enumerated in
  `crates/argo-cli/src/handlers/audit_report.rs::known_flakes()`.
- `argo-cli > worker_actions::worker_actions_tests::gtm_engineer_manifest_declares_expected_action_classification`
  — First observed: 2026-05-09 (`886d9a6bd`). Classification: stable.
  Vertical schema mismatch in the GTM-engineer fixture.
- `argo-cli > tests/approvals_list_seeded_rows_surface.rs::list_surfaces_seeded_approval_request_rows`
  — First observed: 2026-06-10 (`fix-gateway-tests-shared-env-lock-migration`).
  Classification: stable (NOT a parallelism flake — reproduces in full
  isolation: `cargo test -p argo-cli --test approvals_list_seeded_rows_surface`
  fails the same way single-threaded). This is a spawned-gateway
  end-to-end test (separate `tests/` binary) whose seeded-rows request
  comes back `401 {"error":"unauthorized","reason":"authentication
  required"}` against the gateway it boots, so the `assert_eq!(status,
  200)` at `:77` panics. It is a gateway-spawn auth-setup gap, unrelated
  to the in-process unit-test env/cwd lock work; the migration cannot
  affect it because `#[cfg(test)]` modules in `src/` aren't compiled into
  an integration binary. Needs its own follow-up brief (likely the same
  class as the gateway-spawn keychain note — see the
  `SKYCOT_DB_KEY_BASE64` auto-memory).

- `argo-agent-runner > tests/delegate_task_handoff_chain_runtime.rs::delegate_task_without_inbound_handoff_leaves_chain_unset` and `::delegate_task_inherits_active_inbound_handoff_chain`
  — First observed RED: by 2026-06-10 (PR #2801 boundary note). Classification:
  **stable / deterministic** (NOT a flake — both fail single-threaded with
  `SKYCOT_DB_KEY_BASE64` set; 0 passed, 2 failed in 0.13s). Both panic at
  `:219` (`.expect("delegated handoff")`). VERIFIED root cause (conductor
  triage 2026-06-10): the fixture creates a worker-bound session with NO
  capability profile, so the DEC-041 (D) deny-all fence (`dispatch.rs:146-166`,
  commits `107b4e6b1` 2026-06-07 + `60b80e7d6` 2026-05-31 — both landed AFTER
  the test's #2256) denies `delegate_task`'s required `Spawn{Subagent}`
  capability, so `insert_handoff` never runs. The fence is correct-by-design;
  the TEST is stale. Tracked + fix approach in
  `streams/fix-delegate-handoff-test-provision-spawn-capable-worker.md`
  (provision a Spawn-capable worker via the real fence path). Remove this
  entry when that brief lands.

#### RESOLVED 2026-06-10 — gateway env/cwd cross-module parallel races

The ~19-test parallel-flake class boundary-noted on PR #2752
(`handlers::gateway::routes_approvals_json::*`, `resolve_tenant::*`,
`routes_worker_overview/detail::*`, `routes_genesis_stream::*`,
`routes_channels_voice::* / webinbox::*`, etc.) is **fixed** by
`fix-gateway-tests-shared-env-lock-migration` (merged as
[#2781](https://github.com/mark-hallam/argo/pull/2781), commit
`fb6347a9a`): ~57 gateway test modules
that mutated process-global env/cwd under their OWN per-module lock now
acquire the canonical gateway-wide `shared_env_lock()` /
`shared_env_mutex()` (`gateway/mod.rs`), so cross-module tests serialize
on ONE mutex instead of racing. `cargo test -p argo-cli` unit binary is
green 2× consecutively at DEFAULT parallelism (2940/0). Future
`--test-threads=1` workarounds for THIS class should no longer be
needed. (The `approvals_list_seeded_rows_surface` integration entry
above is a separate, deterministic pre-existing failure, not part of
this class.)

#### Post-#1039 strict-mode regressions (argo-cli)

These all live in `argo-cli` and trip the same root cause:
`argo-vertical`'s `parse()` runs the strict-mode pass added in
[#1039](https://github.com/mark-hallam/argo/pull/1039), which rejects
tagged-form manifests that omit the binding fields each kind now
requires (`mcp` → `server` + `tool`, `api` → `binding`,
`computer_use[_macos]` → `profile`) and rejects executor `kind` values
outside `KNOWN_EXECUTOR_KINDS`. The tests below build inline TOML
fixtures that pre-date that pass, so they panic on
`argo_vertical::parse(&toml).expect("manifest parses")` before
reaching their actual assertion. The production code path is fine —
checked-in manifests under `verticals/` already satisfy strict mode.

The 4 `cu_macos_tests::*` entries are also tracked under brief
`codex-cu-macos-tests-restore` for a future fixture refresh. Until
that lands, treat them as pre-existing — do NOT mark BLOCKED.

- `argo-cli > worker_actions_executor::stream_a_tests::resolve_executor_dispatches_each_kind_through_tagged_form`
  — First observed: 2026-05-10 (`ec6f473a1`). Classification: stable.
  Iterates `kind` ∈ {human, mcp, computer_use, api, filesystem,
  code_execution} with bare `executor = { kind = "..." }`; strict
  mode rejects `mcp` (no `server`/`tool`), `computer_use` (no
  `profile`), and `api` (no `binding`).
- `argo-cli > worker_actions_executor::worker_actions_executor_tests::resolve_executor_rejects_unknown_executor_kind`
  — First observed: 2026-05-10 (`ec6f473a1`). Classification: stable.
  Uses `executor = "browser"`; strict mode now rejects the unknown
  kind at parse time, so `expect("manifest parses")` panics before
  the test reaches the dispatch-time error it was originally written
  to assert on.
- `argo-cli > worker_actions_executor_cu_macos::cu_macos_tests::dispatch_on_macos_writes_bundle`
  — First observed: 2026-05-10 (`ec6f473a1`). Classification: stable
  (macOS-only). `manifest_with_kind("computer_use_macos")` builds
  `executor = { kind = "computer_use_macos" }` with no `profile`,
  rejected by strict mode. Compounded by #1722 changing the adapter
  return shape — see `streams/codex-cu-macos-tests-restore.md`.
- `argo-cli > worker_actions_executor_cu_macos::cu_macos_tests::manifest_hyphen_alias_resolves_to_macos_kind`
  — First observed: 2026-05-10 (`ec6f473a1`). Classification: stable.
  Same fixture shape, hyphenated alias `computer-use-macos`, no
  `profile`.
- `argo-cli > worker_actions_executor_cu_macos::cu_macos_tests::manifest_underscore_form_resolves_to_macos_kind`
  — First observed: 2026-05-10 (`ec6f473a1`). Classification: stable.
  Same fixture shape, underscore form `computer_use_macos`, no
  `profile`.
- `argo-cli > worker_actions_executor_cu_macos::cu_macos_tests::unknown_kind_with_macos_in_name_does_not_resolve`
  — First observed: 2026-05-10 (`ec6f473a1`). Classification: stable.
  Feeds `kind = "computer_use_macos_garbage"`; strict mode rejects
  the unknown kind at parse, so `expect("manifest parses")` panics
  before the test reaches its assertion.

## Resolved (kept for historical reference)

- `web/__tests__/a11y-contrast.test.ts > … no hardcoded #71717a remains
  in web/` — RESOLVED by 2026-06-10. The `#71717a` offenders in
  `BuilderSidebarPalette.css` are gone (`grep -rn '#71717a' web/` returns
  nothing), so the regression guard passes.
- `web/__tests__/a11y-headings.test.ts > … no in-product page.tsx
  declares more than one own h1` — RESOLVED by 2026-06-10.
  `web/app/(setup)/accept-invite/page.tsx` now declares a single `<h1>`,
  so the heading-order guard passes.
- `web/__tests__/brand-execution-polish.test.tsx > … no hardcoded
  ease-out/ease-in/ease-linear in component files` — RESOLVED by
  2026-06-10. `UnsavedBanner.tsx:65` and `CohortMagazineStyles.tsx:559`
  now use the `var(--motion-ease)` token instead of hardcoded easing.
  (All three above re-verified together: `npx vitest run
  __tests__/a11y-contrast.test.ts __tests__/a11y-headings.test.ts
  __tests__/brand-execution-polish.test.tsx` → 16/16 pass.)
- `gateway_smoke::*` — RESOLVED in [#858](https://github.com/mark-hallam/argo/pull/858)
  (codex-gateway-smoke-flake-fix, 2026-05-09). The hardened fixtures
  (TOCTOU port-handoff fix, isolated `ARGO_HOME` setup, gateway boot
  serialization, schema/lock retry) eliminated this flake. If
  `gateway_smoke::*`, `gateway_smoke_writes::*`,
  `gateway_smoke_streaming::*`, or `gateway_smoke_audit_costs::*`
  surfaces a new failure, treat it as a genuine break and open a
  follow-up brief — do not boundary-note it.
- `web/__tests__/brand-execution-polish.test.tsx > no off-grid 1.5
  spacing utilities` — RESOLVED in
  [#1952](https://github.com/mark-hallam/argo/pull/1952) (the
  assertion was deleted; the `py-1.5` on `SingleBinaryFooterChip`
  was deliberate at 10.5px text).
- `web/__tests__/workflow-templates.test.tsx > prefers-reduced-motion
  guard` — RESOLVED in #1952 (the guard now reads `page.css` directly
  instead of JSDOM `<style>` tags).
- `web/components/skycot/TeammateCard.test.tsx > click routing` (3
  variants) — RESOLVED in #1952 (production now routes to
  `/workers/<id>/overview`).
- `web/app/(app)/team/page.test.tsx > heading regex` — RESOLVED in
  #1952 (tightened to `/^Team$/`).
- `web/components/skycot/worker/Trust.test.tsx > missing recompute +
  viewer-policy mocks` — RESOLVED in
  [#1985](https://github.com/mark-hallam/argo/pull/1985).

## [Silent-Skip Nightly]

Tracks pre-existing flakes surfaced by the nightly silent-skip
integration runner (`scripts/silent-skip-nightly.sh` driven by
`.github/workflows/silent-skip-nightly.yml.disabled`). Unlike the
unit-test flakes above, these run against real provider test
accounts, so the failure modes are dominated by external state
(quota, real-API drift, network jitter) rather than internal
schema/parser issues. See the runbook at
`docs/skycot/runbook/silent-skip-nightly.md` for the 3-mode
classification (API breakage / quota / flake).

No entries at time of writing — this is the first run. As nightly
results accumulate, add specific `crate::test` ids here when:

- A test fails >20% of nightly runs over a 2-week window AND the
  root cause is provider-side (quota, rate-limit, intermittent 5xx).
- The fix is "add retry+backoff to that test" rather than a code
  change in the crate under test.
- The conductor has reviewed and accepts the flake classification.

Do NOT add an entry for a test that fails on a real API breakage —
that needs a fix-brief, not a flake exemption.
