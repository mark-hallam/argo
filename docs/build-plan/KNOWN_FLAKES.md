# Known pre-existing test flakes

If `cargo test` fails on a test in this list AND that test is OUTSIDE
your brief's may-touch list, push the PR with a "Boundary note" naming
the exact failing test id. Do NOT mark BLOCKED — see HARD RULE 2 in
`SWARM_PROTOCOL.md`.

When a fix PR lands, remove the entry. Don't let this list rot.

## Current flakes

- `gateway_smoke::*` — resolved in #858; ignore if still surfacing.
- `setup_steps::vertical_step_skips_empty_registry` — env leak.
- `worker_actions::gtm_engineer_manifest_declares_expected_action_classification`
  — vertical schema mismatch.

## Post-#1039 strict-mode regressions

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

Follow-up brief: TBD (fixture refresh — tag each inline manifest
with the now-required binding fields, or move the fixtures behind a
helper that injects them). Until then, treat these as pre-existing —
do NOT mark BLOCKED when they surface from a brief outside the
may-touch lines that would fix them.

- `worker_actions_executor::stream_a_tests::resolve_executor_dispatches_each_kind_through_tagged_form`
  — iterates `kind` ∈ {human, mcp, computer_use, api, filesystem,
  code_execution} with bare `executor = { kind = "..." }`; strict
  mode rejects mcp (no `server`/`tool`), computer_use (no `profile`),
  and api (no `binding`).
- `worker_actions_executor::worker_actions_executor_tests::resolve_executor_rejects_unknown_executor_kind`
  — uses `executor = "browser"`; strict mode now rejects the unknown
  kind at parse time, so the test's `expect("manifest parses")`
  panics before it reaches the dispatch-time error it was originally
  written to assert on.
- `worker_actions_executor_cu_macos::cu_macos_tests::dispatch_on_macos_writes_bundle`
  — macOS-only; `manifest_with_kind("computer_use_macos")` builds
  `executor = { kind = "computer_use_macos" }` with no `profile`,
  rejected by strict mode.
- `worker_actions_executor_cu_macos::cu_macos_tests::manifest_hyphen_alias_resolves_to_macos_kind`
  — same fixture shape, hyphenated alias `computer-use-macos`, no
  `profile`.
- `worker_actions_executor_cu_macos::cu_macos_tests::manifest_underscore_form_resolves_to_macos_kind`
  — same fixture shape, underscore form `computer_use_macos`, no
  `profile`.
- `worker_actions_executor_cu_macos::cu_macos_tests::unknown_kind_with_macos_in_name_does_not_resolve`
  — feeds `kind = "computer_use_macos_garbage"`; strict mode rejects
  the unknown kind at parse, so `expect("manifest parses")` panics
  before the test reaches its assertion.
