# mission-demo-seed-runbook-integration — Wire `argo demo seed` CLI, update runbook + dry-run, verify all 8 scenes GREEN

**Owner**: claude.
**Depends on**: `mission-demo-seed-real-llm-driver` MERGED (which in
turn depends on `mission-demo-seed-crate-scaffold` MERGED).
**Blocks**: v1.0 demo MP4 recording (operator-only).
**Authored against**: origin/main `ea0e00353` on 2026-05-25.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-seed-runbook-integration`.

🔴 **P0 DEMO-GATE — split-child #3 of 3.** Parent context in
`mission-demo-seed-data-bootstrap` (BLOCKED) + briefs #1 and #2
(the prereq scaffold + driver).

## Why this brief exists

Briefs #1 and #2 give us a `argo-demo-seed` library crate with
real workers + a real-LLM driver that produces real audit +
approval rows. But there's no operator-facing surface to invoke
it, the runbook still tells the operator to run a non-existent
command, and the dry-run script's scene assertions still target
hypothetical endpoints + button labels that don't match reality.

This brief closes the loop: CLI subcommand + runbook pre-flight
edit + dry-run script alignment + all-8-scenes GREEN verification.

## Acceptance criteria

1. CLI subcommand:
   - Add `DemoCmd::Seed { tenant: Option<TenantId>, dry_run: bool }`
     variant to `crates/argo-cli/src/cli.rs` (existing
     `DemoCmd` enum — keep narrow scope, do not refactor).
   - New handler at `crates/argo-cli/src/handlers/demo_seed.rs`
     that calls `argo_demo_seed::seed_workers` (brief #1) then
     `argo_demo_seed::drive_agent_turns` (brief #2).
   - `--tenant` defaults to a deterministic demo tenant ID printed
     to stdout if unset (so re-runs target the same tenant).
   - `--dry-run` runs brief #1's `seed_workers` only (no LLM
     spend), useful for sanity-checking the workspace before the
     real run.
   - `argo demo seed` honours `ARGO_DEMO_SEED_PROVIDER_KEY` env
     var and fails closed (with the same error from brief #2) if
     unset and `--dry-run` is false.
2. Runbook update (`docs/skycot/demos/v1-mission-mvp-runbook.md`):
   - Add to the "Pre-flight checklist (T-1 day)" section:
     - `[ ] export ARGO_DEMO_SEED_PROVIDER_KEY=<sandbox-key>`
     - `[ ] argo demo seed --tenant <demo-tenant-id>` exits 0 +
       prints `SeedReport` summary.
     - `[ ] curl http://127.0.0.1:7400/audit/by-kind` shows
       non-zero counts for `worker_duty.fired`,
       `workflow.step.executed`, `ApprovalAsked`.
   - Add to the "If a scene breaks mid-take" section:
     - "Re-run `argo demo seed` with `--tenant <fresh-id>` if the
       workspace state needs reset."
3. Dry-run script alignment
   (`scripts/demo-dry-run-fixtures/dry-run.mjs`):
   - Fix the onboarding-button regex so it matches actual button
     labels: connect-provider matches `/next|connect|validate|skip/i`,
     pick-vertical matches `/next|continue|select|pick/i`. OR add
     `data-testid="onboarding-forward"` to the actual buttons in
     `web/app/(setup)/onboarding/*/page.tsx` and update the script
     to query by test-id. Pick whichever is smaller — script-side
     is faster, test-id-side is more durable.
   - Update scene 6 (`6-approval-asked-in-audit`) assertion to hit
     the correct endpoint. If `fix-spa-audit-api-v1-proxy-routes`
     has landed by claim time, target the proxy
     (`/api/v1/audit/events`). If not, target the gateway native
     (`/audit/recent?limit=200`) and reshape the assertion to the
     gateway's `[{seq, kind, payload, ...}]` shape directly.
4. End-to-end verification: after running
   `argo demo seed --tenant <demo>` on a fresh workspace +
   gateway live + dry-run script, ALL 8 scenes pass GREEN. PR body
   includes the dry-run results JSON showing 0 failures.
5. Audit chain repair sub-check: `argo audit verify` against the
   seeded tenant prints `OK` with non-zero rows + zero breaks.
   (Scene 6's pass cue per runbook). If the seed produces breaks,
   either fix the seed's row-emission ordering OR flag a sibling
   fix-brief — do not ship with `breaks > 0`.
6. `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
7. `cargo test -p argo-cli -- demo_seed` passes (handler smoke
   test).
8. Every modified file < 500 LOC.
9. BUILD_STATUS.md updated with PR number, this brief's row
   flipped to IN_REVIEW.
10. Once MERGED, conductor flips `mission-demo-seed-data-bootstrap`
    parent row to MERGED-as-superseded with a note pointing at the
    3 child PR numbers.

## Parallel-surface declarations

- `DemoCmd` enum already exists in `crates/argo-cli/src/cli.rs` —
  this brief adds ONE variant. Grep'd for prior `Seed` variant —
  none exists. No parallel-surface collision.
- `argo demo seed` CLI invocation not registered anywhere else.
- Runbook is the operator-locked canonical at
  `docs/skycot/demos/v1-mission-mvp-runbook.md` (MERGED #2205).
  Older runbook copies (`docs/skycot/demos/recording-checklist.md`,
  `docs/skycot/runbook/v1-demo-recording.md`,
  `docs/skycot/operator/demo-runbook.md`) are historical and point
  at the canonical — do not edit them.

## May-touch

- `crates/argo-cli/src/cli.rs` (add `DemoCmd::Seed` variant only)
- `crates/argo-cli/src/handlers/mod.rs` (add `mod demo_seed;` line)
- `NEW: crates/argo-cli/src/handlers/demo_seed.rs`
- `crates/argo-cli/Cargo.toml` (add `argo-demo-seed` dep)
- `crates/argo-cli/tests/demo_seed_handler_smoke.rs` (NEW or extend
  existing test file)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-flight +
  break-recovery sections only)
- `scripts/demo-dry-run-fixtures/dry-run.mjs` (regex + scene 6
  assertion only)
- `web/app/(setup)/onboarding/{connect-provider,pick-vertical}/page.tsx`
  (ONLY if adding `data-testid="onboarding-forward"` is chosen
  over script-side regex fix)

## Files you must NOT touch

- `crates/argo-demo-seed/**` — briefs #1 and #2's territory.
- `crates/argo-identity/**`, `crates/argo-audit/**`,
  `crates/argo-agent-runner/**` — consume existing public APIs only.
- `crates/argo-store/src/migrations/**` — no migration in scope.
- The 3 historical runbook files (listed above) — leave them as
  pointers.

## Boundary

- Do NOT build the SPA `/api/v1/audit/*` proxy routes. That is a
  separate `fix-spa-audit-api-v1-proxy-routes` fix-brief (recommend
  authoring as a peer; the gateway-native scene-6 fallback in step
  3 makes this brief landable without the proxy).
- Do NOT add new DemoCmd subcommands beyond `Seed`.
- Do NOT refactor the runbook structure — surgical edits only.

## Sizing

~200-400 LOC across CLI + runbook + dry-run. Single-worker brief.
The "all 8 scenes GREEN" end-to-end verification is the
load-bearing acceptance — partial pass blocks this brief landing.

## Branch + PR

- Branch: `claude-mission-demo-seed-runbook-integration`
- PR title: `demo-seed: argo demo seed CLI + runbook + dry-run
  alignment (mission-demo-seed-runbook-integration)`
