# Mission load-bearing wire — operator onboarding wizard (`argo wizard onboard`)

**Owner**: claude. **Depends on**: existing `argo setup` / `argo doctor` / `argo new` CLI surface (verify in first 5min via `grep -nE 'Setup|Doctor|New' crates/argo-cli/src/cli_runtime_cmds.rs`); existing config primitives in `argo-config`.
**Authored against**: origin/main `da6415b0` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-operator-onboarding-wizard`.

> SOURCE: MISSION_BACKLOG.md Section 11 #170 — "Build operator onboarding wizard end-to-end." Today a fresh operator runs `argo setup`, `argo doctor`, edits `argo.toml` by hand, copies their provider key, then `argo new` to create their first worker — five separate commands with hand-edited TOML between. Mission promise: `argo wizard onboard` is a single interactive command that walks through the same 5-step setup with prompts and validation, ending with a working first worker.

## Scope

1. **NEW CLI** `argo wizard onboard [--non-interactive] [--config <path>]`:
   - Step 1: detect platform / dependencies (calls existing `doctor` checks under the hood).
   - Step 2: prompt for `ANTHROPIC_API_KEY` (or other provider); validate via a quick test call.
   - Step 3: prompt for operator name + email; write to `argo.toml [operator]`.
   - Step 4: prompt for first vertical (from existing `argo vertical list` output, defaults to "marketplace-seller-browser").
   - Step 5: prompt for first worker name + persona summary; run existing `argo new` flow internally.
   - End: print summary + next-step commands ("try `argo chat <worker>`, `argo handoff list`, …").

2. **Non-interactive mode** `--non-interactive --answers <path-to-toml>`:
   - Reads pre-populated answers from a TOML file. Useful for CI and "copy-paste this config from your team's onboarding doc."
   - Skips prompts; runs all steps with the file's answers. Validates inputs, errors out clearly on bad data.

3. **Idempotency**: if `argo.toml` already exists and has `[operator]`, prompt "already configured — overwrite [y/N]?". Default No.

4. **Validation per step**: each step's prompt validates before moving on (e.g. API key gets a real test call; vertical name must be in `argo vertical list`; worker name must match `/^[a-z][a-z0-9_-]{2,30}$/`).

5. **NEW doc**: `docs/skycot/runbook/wizard-onboard.md` (≤ 100 LOC) — walks through what each step does, what the prompts mean, how to find answers.

6. **Telemetry-free**: NO usage data sent during onboarding. Operator might be airgapped. Document explicitly.

This is NOT in scope: GUI/TUI version (CLI prompts are fine for v1); multi-provider setup (one provider in step 2; operator adds others via existing config later); auto-detect existing `~/.argo/` from another machine; tenant/team setup (single-operator v1).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/wizard_onboard.rs` (NEW, ≤ 400 LOC) — the wizard handler.
- `crates/argo-cli/src/handlers/wizard_onboard_tests.rs` (NEW, sibling, ≤ 250 LOC) — 8 unit tests: each step validation; non-interactive happy path; non-interactive bad input; existing-config-prompt; existing-config-default-no; pid-test-passes-known-good-key; api-key-test-rejects-bad-key.
- `crates/argo-cli/src/cli_command_enum.rs` (verify path) — narrowly scoped: ADD `Wizard { cmd: WizardCmd }` parent + `Onboard` variant.
- `crates/argo-cli/src/cli_wizard_cmds.rs` (NEW, ≤ 80 LOC) — subcommand enum.
- `crates/argo-cli/tests/wizard_onboard_smoke.rs` (NEW, ≤ 200 LOC) — 1 integration smoke: non-interactive mode with a known-good answers file produces a working argo.toml + first worker.
- `docs/skycot/runbook/wizard-onboard.md` (NEW, ≤ 100 LOC) — operator-facing walkthrough.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/setup.rs` / `doctor.rs` / `new.rs` — reuse only
- `crates/argo-config/**` — reuse only; no schema change
- `crates/argo-store/migrations/**` — no schema change
- Any non-cli crate

## Acceptance criteria

- [ ] `argo wizard onboard` runs interactively end-to-end on a clean machine
- [ ] `argo wizard onboard --non-interactive --answers <file>` works deterministically
- [ ] Each step validates input before moving on
- [ ] Existing-config prompt defaults to No (preserve operator's data)
- [ ] API-key test call is real (verified against a mock provider in unit tests)
- [ ] Worker name regex enforced
- [ ] On success, prints next-step commands
- [ ] 8 unit tests + 1 integration smoke
- [ ] Runbook doc exists
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-operator-onboarding-wizard`
- PR title: `cli: argo wizard onboard end-to-end operator setup (mission-operator-onboarding-wizard)`

## Notes for the implementer

- For interactive prompts: use `dialoguer` if it's already a workspace dep; otherwise stdin-readline is fine. Document choice in PR body.
- The validation step for API key: do a minimal real call (`messages.create` with 1 token max) — short circuit any retries. Document the cost (~$0.001 per validation).
- For the non-interactive mode answers file: keep the schema simple — flat TOML keys mirror the prompt order.
- DON'T try to embed Anthropic-account creation in the wizard. The operator must arrive with a key in hand.
- The wizard MUST be safe to re-run — running it after partial setup picks up where it left off (or prompts to overwrite). This is the "idempotent" requirement.
