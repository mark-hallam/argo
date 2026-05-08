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
