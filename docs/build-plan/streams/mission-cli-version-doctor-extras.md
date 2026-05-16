# mission-cli-version-doctor-extras — `--version` + `argo doctor` build info (post-v1 polish P2)

**Owner**: codex. **Depends on**: none (single-crate, mechanical).
**Authored against**: origin/main `223d3fab` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cli-version-doctor-extras`.
**Source**: post-v1 polish — operator UX. `argo --version` currently prints just the cargo-pkg version; insufficient for incident triage.

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — when an operator reports an issue, "I'm on version 1.0.0" is less useful than "I'm on 1.0.0+<git-sha>, built 2026-05-17, with feature otlp enabled, on aarch64-apple-darwin".
- **Reliability** (#1) — `argo doctor` is the operator's first port of call; surface ground-truth.

## Scope

P2. Three additions:

1. **`argo --version`** extends from `argo 1.0.0` to:
   ```
   argo 1.0.0 (a1b2c3d 2026-05-17)
     target: aarch64-apple-darwin
     rustc:  1.79.0 (stable)
     profile: release
     features: otlp,sentry
   ```
   - git SHA + commit date from `OUT_DIR/build_info.rs` populated by `build.rs`.
   - Target triple from `env!("TARGET")` (set by cargo).
   - Rustc version from `built` crate or hand-rolled `rustc_version_runtime`.
   - Active features from `cfg!(feature = "...")` introspection.

2. **`argo doctor`** prepends a Build Info block above the existing health checks. Same fields as `--version` plus:
   - Binary path (`std::env::current_exe()`).
   - Working directory + `argo.toml` path (if found).
   - Total uptime since launch (N/A for one-shot but useful for the long-running gateway → make `doctor` callable against running instance via `--remote http://localhost:8090` flag in a follow-up; out of scope for this brief).

3. **`argo doctor --json`** includes the same build fields under `build_info` key.

LOC: ~120.

## Files you may touch

- `crates/argo-cli/build.rs` (NEW or extend existing) — writes `build_info.rs` to `OUT_DIR` with constants for `GIT_SHA`, `GIT_DATE`, `TARGET`, `RUSTC_VERSION`, `PROFILE`, `FEATURES`. ≤60 LOC.
- `crates/argo-cli/Cargo.toml` — narrowly: ensure `[build-dependencies]` includes whatever you use to read git (e.g. `git2`, or simpler: `std::process::Command::new("git")`). ≤5 modified LOC.
- `crates/argo-cli/src/build_info.rs` (NEW) — wrapper that `include!`s `build_info.rs` from `OUT_DIR` + formats as text + serializable struct. ≤40 LOC.
- `crates/argo-cli/src/handlers/doctor.rs` — prepend Build Info section to output. Use `BuildInfo::serialize()` for JSON. ≤25 modified LOC.
- `crates/argo-cli/src/cli_command_enum.rs` — override clap version string with a runtime-computed value. Use `Command::version("...")` via a const-format trick or via `clap::Command::new(...).version(BuildInfo::VERSION_STRING_LONG)`. ≤8 modified LOC.
- `crates/argo-cli/src/handlers/doctor_tests.rs` — extend with two cases (text contains GIT_SHA stub; JSON has build_info object). ≤30 modified LOC.

## Files you must NOT touch

- Any non-CLI crate.
- The CI `cli-reference-drift.yml` — output format changes for `--version` are expected; this workflow re-generates the reference. (Worker should re-run `cargo run -- generate-cli-reference > docs/skycot/cli-reference.md` if drift workflow flags.)

## Acceptance criteria

- [ ] `argo --version` shows git SHA + date + target + rustc + features.
- [ ] `argo --version` works in tests (where `OUT_DIR` is set) and when run from `cargo run --` (where `OUT_DIR` is set during build).
- [ ] `argo --version` does NOT crash in unusual environments — falls back to `"unknown"` for any field that can't be determined.
- [ ] `argo doctor` shows a "Build Info" section above health checks.
- [ ] `argo doctor --json` includes `build_info: { version, git_sha, git_date, target, rustc, profile, features: [...] }`.
- [ ] Building without a git repo (e.g. tarball install) does not break the build — `build.rs` falls back gracefully.
- [ ] `cargo test -p argo-cli doctor` passes.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-cli-version-doctor-extras`
- Title: `feat(cli): build info + features in --version and argo doctor (mission-cli-version-doctor-extras)`
- Worker verification: crate-scoped tests + clippy + `cargo clippy --workspace` per SWARM_PROTOCOL.md.

## Hidden constraints

- **`build.rs` failing fails the build.** Wrap `Command::new("git")` calls in fallible logic; emit a `cargo:warning=...` line on failure and write `"unknown"` to the constant.
- **`OUT_DIR` is only available during cargo build.** Static `include!` works; runtime `env!("OUT_DIR")` doesn't.
- **No clap `command!()` macro** — the runtime version computation needs `.version(&str)` on the builder.
- **Cargo passes `TARGET` to build scripts**, not to the main crate. Persist it via the build script.
- **Active features** — `cfg!(feature = "otlp")` evaluates at compile time; emit a static list of enabled features into `build_info.rs`.
