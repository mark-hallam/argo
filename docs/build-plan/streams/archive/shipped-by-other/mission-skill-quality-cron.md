# Mission load-bearing wire — `argo skill quality-recompute` cron tick

**Owner**: claude. **Depends on**: `mission-skill-quality-from-audit` MERGED (provides `QualityInputs::from_audit_history`).
**Authored against**: origin/main `d53a8f71` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-quality-cron`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 item #78 ("Build cron tick `argo skill quality-recompute --all`"). Pairs with #77 (`mission-skill-quality-from-audit`): once that brief lands the offline constructor, this brief delivers the operator-runnable + cron-suitable command that exercises it. Mission Claim 4 ("skills shared and learned") needs a periodic recompute to detect quality drift; without this command, recompute is theoretical.

## Scope

Add `argo skill quality-recompute [--skill <id> | --all] [--since <ts>] [--dry-run]` CLI subcommand under the existing `skill` family. For each in-scope skill:

1. Call `QualityInputs::from_audit_history(skill_id, &audit_store, since)` (from #77)
2. Call `compute_score(&inputs)` (existing primitive)
3. Persist the resulting `QualityScore` via the existing skill-quality persistence path (re-use, don't redesign)
4. Print one line per skill: `<id> <old_score> -> <new_score> (<delta>)`

`--all` walks every skill in the local marketplace; `--skill <id>` targets one. `--since <ts>` is forwarded to `from_audit_history` for incremental recompute. `--dry-run` runs steps 1-2 + prints, but does NOT persist.

This is NOT in scope: building the actual cron / systemd-timer wrapper (operator concern); hooking into the forge auto-promote loop (separate brief #69 already authored as `mission-wire-judge-on-forge-promote`); persisting recompute history; cross-tenant aggregation.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/skill_quality_recompute.rs` (NEW, ≤ 250 LOC) — handler analogous to `skill_distill_now.rs` (template at `crates/argo-cli/src/handlers/skill_distill_now.rs`); reads args, walks skills, calls `from_audit_history` + `compute_score`, prints output, persists unless `--dry-run`.
- `crates/argo-cli/src/cli_subcommands_skill.rs` — narrowly scoped: ADD a `QualityRecompute { skill: Option<SkillId>, all: bool, since: Option<DateTime<Utc>>, dry_run: bool }` variant to the existing `SkillCmd` enum (verify enum location in first 5 minutes; may live elsewhere).
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: ONE `pub(crate) mod skill_quality_recompute;` line.
- `crates/argo-cli/src/main.rs` — narrowly scoped: ONE arm in the skill-dispatch match for `QualityRecompute` calling the new handler.
- `crates/argo-cli/src/handlers/skill_quality_recompute_tests.rs` (NEW, sibling) — 5 unit tests: dry-run-does-not-persist; persist-flow-updates-score; --all-walks-marketplace; --skill-targets-single; --since-filter-passes-through.
- `crates/argo-cli/tests/skill_quality_recompute_smoke.rs` (NEW) — integration smoke: seed marketplace + audit log; invoke `argo skill quality-recompute --all`; assert score persisted for at least one skill.

## Files you must NOT touch

- `crates/argo-skill/src/quality.rs` — `compute_score`, `QualityInputs`, `from_audit_history` are upstream contracts; reuse only
- `crates/argo-skill/src/marketplace.rs` — marketplace iteration is upstream; reuse only
- `crates/argo-store/migrations/**` — no schema change
- Any forge-related handler (separate brief)

## Acceptance criteria

- [ ] `argo skill quality-recompute --all` walks every local skill and recomputes
- [ ] `argo skill quality-recompute --skill <id>` targets one skill; errors if id unknown
- [ ] `argo skill quality-recompute --skill <id> --since <iso8601>` forwards timestamp to `from_audit_history`
- [ ] `--dry-run` prints output but does NOT persist
- [ ] Output line format: `<skill_id> <old_score:u8> -> <new_score:u8> (<delta:i16>)` — one per skill processed
- [ ] When `--all` produces zero skills (empty marketplace), prints `no skills found` and exits 0 (NOT an error)
- [ ] Mutually-exclusive args: `--all` + `--skill` is a usage error (clap-level if possible)
- [ ] 5 new unit tests cover the matrix
- [ ] 1 integration smoke exercises end-to-end
- [ ] `cargo build -p argo-cli` clean
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-quality-cron`
- PR title: `cli: argo skill quality-recompute (--skill | --all) [--since] [--dry-run] (mission-skill-quality-cron)`

## Notes for the implementer

- **Wait for #77 (`mission-skill-quality-from-audit`) to merge first.** If you claim this brief while #77 is still in flight, mark BLOCKED with reason "depends on mission-skill-quality-from-audit MERGED".
- Use `skill_distill_now.rs` as the structural template — same handler shape, same async surface, same arg parsing pattern.
- The `compute_score` + persist path: find the existing `argo-skill` API that persists `QualityScore` (search for `pub async fn record_score` or `pub fn put_score`). If no such API exists, BLOCK with reason "skill quality persistence path not exposed; needs upstream brief".
- Output format: don't add JSON. The next brief that wants programmatic output can add `--json`. v1 is human-readable text.
- The cron / systemd wrapper: out of scope. Operators wire this via `crontab -e`. Document the suggested invocation in the handler's rustdoc as a comment, but don't add a `Cargo.toml` `[[bin]]` for it.
- If walking the marketplace requires a slow scan (e.g. parsing every TOML), print progress every 50 skills so an operator running it interactively gets feedback.
