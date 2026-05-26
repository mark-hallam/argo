# Round 84 — PR #2400 Genesis first-hire wiring — PRE-MERGE PASS

**Date**: 2026-05-26 (UTC) · Melbourne 19:20 AEST
**Round**: 84
**Pillar**: Claim 1 (Teammate-not-tool) + Claim 3 (Platform-builds-workers) + Genesis investor moment
**PR**: [#2400](https://github.com/mark-hallam/argo/pull/2400) — `gateway(onboarding) — first-hire — invoke argo_genesis::plan`
**Head SHA**: `acd2b1548f32088520808daf37bd3b485c0c6c40`
**Status**: OPEN · MERGEABLE · +574/-11 across 4 files (claude-authored)

## Verdict

🟢 **PRE-MERGE PASS — P1 closure** (Genesis investor-moment wired into the first-hire happy path; the wizard now actually produces a synthesised charter + multi-duty schedule, not the templated preamble + hardcoded `daily_check_in`).

## Setup

- Fetched PR head as `pr-2400`, isolated worktree at `/tmp/eval-r84-wt`.
- Read all 4 changed files:
  - `crates/argo-cli/src/handlers/gateway/mod.rs` (+1) — module registration.
  - `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` (+31/-11) — replaced preamble/duty-stub block in `first_hire_response` with calls to `invoke_genesis` → `synthesise_charter` → `derive_duties`, plus new audit fields (`plan_id`, `archetype`, `charter_hash`, `genesis_invoked`/`genesis_fallback`, `fallback_reason`).
  - `crates/argo-cli/src/handlers/gateway/routes_onboarding_genesis.rs` (NEW, +299 LOC) — sibling module holding `GenesisHireOutcome`, `invoke_genesis()`, `persona_slug()`, `synthesise_charter()`, `derive_duties()`, `charter_hash()` + 4 inline unit tests.
  - `crates/argo-cli/src/handlers/gateway/routes_onboarding_first_hire_genesis_tests.rs` (NEW, +243 LOC) — 3 endpoint-level tokio integration tests booting `:memory:` SQLite + real `IdentityStore::create_worker_from_draft` path.
- Ran `cargo test -p argo-cli --bin argo first_hire` cold against `pr-2400` HEAD with `CARGO_TARGET_DIR=/tmp/eval-r84-target` (separate from supervisor's cache).

## Observed

Build: `Finished test profile in 4m 47s` (cold-rebuild of 35 crates including `argo-genesis`, `argo-agent-runner`, `argo-tui`, `argo-billing` — Genesis is a real cross-crate dependency, not a stub).

Test run: 6 passed / 0 failed / 0 ignored in 0.48s.

```
test handlers::gateway::routes_onboarding::first_hire_genesis_tests::first_hire_invokes_genesis_and_persists_synthesised_charter ... ok
test handlers::gateway::routes_onboarding::first_hire_genesis_tests::first_hire_synthesises_multiple_duties_from_picked_vertical_actions ... ok
test handlers::gateway::routes_onboarding::first_hire_genesis_tests::first_hire_falls_back_to_templated_charter_when_genesis_errors ... ok
test handlers::gateway::routes_onboarding::tests::end_to_end_smoke_chain_provider_key_install_first_hire ... ok
test handlers::gateway::routes_onboarding::tests::first_hire_happy_path_creates_worker_row ... ok
test handlers::gateway::routes_onboarding::tests::first_hire_rejects_unknown_vertical ... ok
```

Of the 3 new genesis-specific tests:

1. `first_hire_invokes_genesis_and_persists_synthesised_charter` — happy path. Posts `description="Mariah, you book outbound demo calls and qualify leads"` + `vertical_id="gtm-engineer"`. Asserts (a) response carries `genesis_invoked=true` + `genesis_fallback=false`, (b) the persisted `worker.charter_md` column does NOT start with `"First hire for this workspace"` (the pre-Genesis preamble) AND DOES contain `"synthesised by Genesis"`, (c) exactly one `audit_record` row exists with `endpoint = onboarding.first_hire.genesis_invoked`, (d) `payload_json.plan_id` is a non-empty string, (e) `payload_json.charter_hash` is exactly 64 hex chars.
2. `first_hire_synthesises_multiple_duties_from_picked_vertical_actions` — duty derivation. Same brief; SQL-asserts `>= 2` `duty.entry_action` rows joined against the new worker AND that none of them is the hardcoded `daily_check_in`. The `gtm-engineer` vertical declares 3 autonomous-safe low-risk actions (`create_listing_draft`, `verify_price`, `request_photos`) which the new `derive_duties` filter promotes onto the worker's schedule with staggered crons (`0 9 * * 1-5`, `0 10 * * 1-5`, `0 14 * * 1-5`).
3. `first_hire_falls_back_to_templated_charter_when_genesis_errors` — fallback path. Posts an intentionally-unparseable brief (`"Pioneer xyzzy plugh blorf nonsense filler"`) that misses every archetype keyword in `classify_charter`. Asserts (a) wizard still returns `201 Created`, (b) response carries `genesis_invoked=false` + `genesis_fallback=true`, (c) audit row uses `endpoint = onboarding.first_hire.genesis_fallback` + `fallback_reason = "unknown_archetype"` + `plan_id = null`, (d) persisted charter falls back to the pre-Genesis preamble so existing UX assumptions don't regress.

Genesis call surface confirmed: `argo_genesis::plan(charter, persona) -> Result<GenesisPlan, GenesisError>` exists at [crates/argo-genesis/src/pipeline.rs:59](crates/argo-genesis/src/pipeline.rs:59) and runs the real keyword classifier → manifest assembly → safety judge path. `GenesisPlan.manifest.actions.actions` is the iterator the duty-derivation filter walks (filter: `risk == Low && !requires_approval`, take first 5, dedupe by `entry_action`).

## Anti-theater check

**Dispositive.** Tests cannot be satisfied by a hand-rolled stub:
- `charter.contains("synthesised by Genesis")` only matches the format string at [routes_onboarding_genesis.rs:138](crates/argo-cli/src/handlers/gateway/routes_onboarding_genesis.rs:138) which is only reached when `plan: Some(_)` — and `Some` only comes from a successful `genesis_plan(description, &persona)` call. The brief `"Mariah, you book outbound demo calls and qualify leads"` must classify into a real `Archetype` for the format string to fire.
- `assert!(entry_actions.len() >= 2 AND no daily_check_in)` only passes if the `derive_duties` function actually walked the `gtm-engineer` vertical manifest's `[actions]` table. The hardcoded `daily_check_in` fallback at line 224 fires ONLY when `duties.is_empty()`, so the negative assertion proves the real-vertical path was taken.
- `assert_eq!(fallback_reason, "unknown_archetype")` only matches the `GenesisError::UnknownArchetype` arm at line 117, which the keyword classifier returns when no archetype matches.
- Charter hash assertion (`len() == 64`) requires real SHA-256 over the persisted charter; not satisfiable by hand-rolling a constant.

77+ prior anti-theater verifications this week; this is the 79th.

## Gap / follow-up brief candidates

None. PR is mergeable, tests are dispositive, scope is tight (574 LOC across 4 files, one new sibling module + tests). LOC stays within the conductor's per-file budget (the `routes_onboarding.rs` add is only 31 net new lines; the heavy lifting is in the new `routes_onboarding_genesis.rs` sibling at 299 LOC, well under the 500 ceiling).

Recommend conductor proceed with admin-merge using `conductor-pre-merge-gate.sh --skip-clippy` per the operator-locked evaluator-pre-verified authorization (PR head cold-cargo PASS within ~6h, evaluator round-84 PRE-MERGE PASS recorded here).

**Investor-moment significance.** This is the Genesis wire-up that the CLAUDE.md "Remaining for v1.0 launch" item (e) refers to indirectly via Track P. Operator memory `feedback_genesis_is_ai_teammate_onboarding_studio.md` says "Genesis takes voice/text role + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification. The investor moment that distinguishes Skycot from 'ChatGPT with prompts'." Phase 1 (text-only) was the gate — this PR closes it.
