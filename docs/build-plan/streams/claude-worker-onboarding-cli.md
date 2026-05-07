# Worker onboarding from a single TOML spec (`argo worker onboard`)

**Owner**: claude. **Depends on**: —

## Why

Today, creating a fully-formed Worker takes 5+ separate CLI calls
(persona create, worker draft, channels add, role-set, memory init).
Founders + operators want one command: "here's a TOML, give me a
teammate." This is the on-ramp the demo rests on.

## Scope

`argo worker onboard --persona-spec <path/to/spec.toml>`. The TOML carries
persona name + voice + signature, role description, vertical_id, optional
channels (array), optional initial duties (cron strings), and optional
memory_namespace seed.

The command:
1. Parses the spec, validates required fields.
2. Creates the Worker (Persona + Role + WorkerId + memory namespace).
3. Persists via existing worker repo.
4. Adds requested channels.
5. Schedules requested duties (using existing `argo worker duty` machinery
   IF it's already wired — do not re-implement).
6. Prints the newly-issued WorkerId + a markdown summary.

Idempotency: if `--id <existing-id>` passed and worker exists, fail
loudly. No update-on-collision.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/worker_onboard.rs` (~280 LOC):
   - `pub struct OnboardSpec` (serde-deserialized from TOML)
   - `pub fn validate(spec: &OnboardSpec) -> ArgoResult<()>` — required
     fields, regex-checks on vertical_id, charter size cap, channel count
     cap (≤ 8)
   - `pub async fn run(args, store, ...) -> ArgoResult<WorkerId>`
2. **NEW** `crates/argo-cli/src/handlers/worker_onboard_tests.rs` (~200 LOC)
   covering: happy path, missing required field, oversize charter,
   too many channels, invalid vertical_id, channel-add failure mid-flight
   (must rollback or surface partial state cleanly).
3. **Modify** `crates/argo-cli/src/cli.rs` — narrowly: 1 new
   `Worker::Onboard { persona_spec: PathBuf, id: Option<WorkerId> }`
   variant. EXHAUSTIVE-MATCH note: handlers/mod.rs dispatch arm.
4. **NEW** `crates/argo-cli/tests/worker_onboard_smoke.rs` (~120 LOC)
   round-trip: write TOML to tempdir, run command, assert worker
   readable via `argo worker show`.
5. **NEW** `docs/build-plan/streams/fixtures/onboard_spec_example.toml`
   — a real spec for the v1 demo (Maya the GTM worker).

## Files you may touch

- `crates/argo-cli/src/handlers/worker_onboard.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/worker_onboard_tests.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (1 pub mod + 1 dispatch arm)
- `crates/argo-cli/src/cli.rs` (BLANKET CLI ALLOWLIST — 1 enum variant; cli.rs is 49 LOC post-#968 split, plenty of headroom)
- `crates/argo-cli/src/cli_command_enum.rs` (post-#968: this is where the `Command` enum body lives now — variant additions go here)
- `crates/argo-cli/src/cli_worker_cmds.rs` (post-cli-split: WorkerCmd variants live here — add `Onboard` variant)
- `crates/argo-cli/src/handlers/worker_actions.rs` (worker dispatch lives here — add 1 dispatch arm for the Onboard variant)
- `crates/argo-cli/tests/worker_onboard_smoke.rs` (NEW)
- `docs/build-plan/streams/fixtures/onboard_spec_example.toml` (NEW)

## Files you must NOT touch

- `crates/argo-identity/` — re-use existing `Worker::draft` etc.
- `crates/argo-store/` — re-use existing worker repo
- The duty scheduler — only invoke its existing public API
- The channel implementations — only invoke `worker channels add`'s existing entry point

## Acceptance criteria

- [ ] `argo worker onboard --persona-spec foo.toml` creates a worker and prints WorkerId
- [ ] `argo worker show <new-id>` finds the worker (depends on
      `claude-worker-introspection-cli` for ideal verification, but
      any existing introspection works)
- [ ] Missing `vertical_id` in TOML → clear error
- [ ] >8 channels in spec → rejected before any DB write
- [ ] Mid-flight channel-add failure → command exits non-zero with the
      offending channel named; worker may exist but stderr is clear
- [ ] Idempotency: `--id` of existing worker → clean failure, no mutation
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-onboarding-cli`
- PR title: `worker: one-command onboarding from TOML spec (claude-worker-onboarding-cli)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies.
- The spec format is part of the public surface — pin it with serde
  rename rules so we don't break it later.
- If the TOML doesn't include `duties` and the duty scheduler isn't
  yet wired, that's fine — skip that block. Don't add the wiring here.
- Lean on `toml = "0.8"` (already a workspace dep — verify before adding).
