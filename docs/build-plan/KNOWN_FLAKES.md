# Known pre-existing test flakes

If `cargo test` or `pnpm -C web test` fails on a test in this list AND
that test is OUTSIDE your brief's may-touch list, push the PR with a
"Boundary note" naming the exact failing test id. Do NOT mark BLOCKED
— see HARD RULE 2 in `SWARM_PROTOCOL.md`.

When a fix PR lands, remove the entry. Don't let this list rot.

Last refreshed: 2026-05-22 against `origin/main` `204a0f9f1` (claude-known-flakes-list-refresh).

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

A full web vitest run cold takes ~5 min wall clock (1339 tests, 168
files). Warm cache (`.next/`, `node_modules/.vite/`) ~2-3 min.

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

All four reproduce on clean `origin/main` `204a0f9f1` as of
2026-05-22. They are stable pre-existing failures (always fail), not
intermittent flakes. They surface as "pre-existing" in every recent
web-PR boundary note (#2011, #2013, #2014, #2018, #2026, #2027,
#2030, #2032).

- `web/__tests__/a11y-contrast.test.ts > a11y color contrast (WCAG 2.1 AA, 1.4.3) > no hardcoded #71717a remains in web/ (regression guard — fails AA against #fafaff)`
  — First observed: 2026-05-21 (#1914 introduced the regression
  guard). Classification: stable. Failure: 2 offenders remain in
  `web/app/(app)/workflows/[verticalId]/[name]/build/BuilderSidebarPalette.css:158,167`.
- `web/__tests__/a11y-headings.test.ts > a11y heading order > no in-product page.tsx declares more than one own h1 in its own file`
  — First observed: 2026-05-21 (#1917 heading-order sweep).
  Classification: stable. Failure: `web/app/(setup)/accept-invite/page.tsx`
  declares 4 `<h1>` tags (the test tolerates at most 2 in
  mutually-exclusive branches).
- `web/__tests__/brand-execution-polish.test.tsx > brand-execution polish (closes audit P1-2) > no hardcoded ease-out/ease-in/ease-linear in component files`
  — First observed: 2026-05-18 (#1821 brand-drift sweep; partial fix
  in #1952 wave-5). Classification: stable. Failure: hardcoded
  `ease-out` in `web/components/skycot/builder/UnsavedBanner.tsx:65`
  and `ease-in-out` in
  `web/components/skycot/cohorts/CohortMagazineStyles.tsx:565`.
- `web/__tests__/onboarding/wizard.test.tsx > Welcome step > renders the headline and continues to /onboarding/workspace`
  — First observed: 2026-05-21 (#1898 locked the F1
  `WizardEditorialShell`; welcome hero text changed from "Welcome to
  SKYCOT." to "Build your AI workforce."). Classification: stable.
  Failure: `screen.getByText("Welcome to SKYCOT.")` no longer matches;
  the rendered headline is "Build your AI workforce."

### Rust (cargo) — `cargo test --workspace`

- `argo-cli > setup_steps::tests::vertical_step_skips_empty_registry`
  — First observed: 2026-05-09 (`886d9a6bd`). Classification: flake
  (env leak — sensitive to `ARGO_VERTICALS_DIR` set by sibling tests
  in the same binary). Already enumerated in
  `crates/argo-cli/src/handlers/audit_report.rs::known_flakes()`.
- `argo-cli > worker_actions::worker_actions_tests::gtm_engineer_manifest_declares_expected_action_classification`
  — First observed: 2026-05-09 (`886d9a6bd`). Classification: stable.
  Vertical schema mismatch in the GTM-engineer fixture.

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
