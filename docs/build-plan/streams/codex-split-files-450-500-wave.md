# codex-split-files-450-500-wave — drop top-10 at-risk files below 420 LOC (P1 audit-finding)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high — same pattern as #1808 but multi-file.
**Depends on**: none.
**Authored against**: origin/main `fa849bbb` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-split-files-450-500-wave`.
**Source**: 2026-05-17 claude conductor audit finding P1-1. Post #1808 (split-7-over500-files), 45 files sit in the 450-500 LOC at-risk band. One file (`cli_subcommands_skill.rs`) is exactly at 500. Any new mount-line tripping these would re-red the loc-budget gate.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — loc-budget invariant is GREEN today; this brief keeps it green for the next wave of feature work.
- **Repeatability** (#3) — same pattern as #1808; mechanical extraction.

## Scope

P1. Split top-10 at-risk files (450-500 LOC band) down to ≤420 LOC each. Leaves ~80 LOC of headroom per file for future additions.

### Top-10 target list (as of `fa849bbb`)

| File | Current LOC | Target | Strategy |
|---|---:|---:|---|
| `crates/argo-cli/src/cli_subcommands_skill.rs` | 500 | ≤420 | Extract per-subcommand (rate / ratings / publish / install) into siblings |
| `crates/argo-executor-computer-use-macos/src/executor.rs` | 495 | ≤420 | Extract preflight + permission check into `executor_preflight.rs` |
| `crates/argo-mcp/src/pool.rs` | 494 | ≤420 | Extract `BucketRegistry` (added by #1806) into `rate_limit.rs` sibling |
| `crates/argo-orchestrator/src/persist.rs` | 492 | ≤420 | Extract per-table CRUD into `persist_<table>.rs` siblings |
| `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` | 490 | ≤420 | Extract `apply_effective_gates` + `quiet_hours_check` into siblings |
| `crates/argo-cli/src/handlers/gateway/sso_oidc.rs` | 490 | ≤420 | Extract OIDC discovery + JWKS handling into sibling |
| `crates/argo-skill/src/ir/types_tests.rs` | 488 | ≤420 | Test-side split by scenario group |
| `crates/argo-audit/src/sink.rs` | 488 | ≤420 | Extract broadcast fan-out + chain-hash helpers |
| `crates/argo-skill-sdk/src/builders.rs` | 487 | ≤420 | Extract per-builder into `builders_<kind>.rs` siblings |
| `crates/argo-cli/src/handlers/worker_actions.rs` | 486 | ≤420 | Extract action-specific dispatch helpers |

### Discipline

- **No behavior change.** Pure mechanical extraction.
- **`pub(super)` or `pub(crate)` for extracted helpers** — never expand public surface.
- **All extracted helpers ≤80 LOC** (fn-LOC budget).
- **Tests stay green without modification** (or with mechanical import-path updates only).

LOC: ~600 net (lines move between files; net additions for module declarations).

## Files you may touch

For each of the 10 target files: the file itself + 2-4 new sibling files. Plus parent `mod.rs` for module registration.

**Files always touched (per target):**
- The target `.rs` file (existing) — net trim. ≤150 modified LOC each.
- 1-3 new sibling `.rs` files (NEW). ≤200 LOC each.
- Parent `mod.rs` — module declarations. ≤5 modified LOC each.
- Sibling test files — import-path updates only if helpers moved. ≤10 modified LOC each.

## Files you must NOT touch

- Behavior. Pure mechanical extraction.
- Public API surface — `pub fn` / `pub struct` declarations stay where they are.
- The CI loc-budget workflow — consume only.
- Files outside the top-10 list.

## Acceptance criteria

- [ ] All 10 target files are ≤420 LOC after split.
- [ ] `find crates -name '*.rs' -exec wc -l {} \; | awk '$1 >= 450 && $1 <= 500 {print}'` count drops by ≥10.
- [ ] All extracted helpers ≤80 LOC.
- [ ] Public API surface unchanged (verify with `git diff --stat` filtered on `pub fn`/`pub struct`/`pub enum`).
- [ ] No behavior change asserted by existing tests passing without functional modification.
- [ ] `cargo test --workspace` does not regress.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-split-files-450-500-wave`
- Title: `refactor: split top-10 at-risk files (450-500 LOC) below 420 (codex-split-files-450-500-wave)`
- Worker verification: crate-scoped clippy + tests + workspace clippy + loc-budget script per SWARM_PROTOCOL.md.

## Hidden constraints

- **Order doesn't matter (no cross-file deps in this list)** but tackle the closest-to-500 first (cli_subcommands_skill.rs at exactly 500) so the safety margin is visible early.
- **Don't introduce new crates.** Each extraction is a sibling module in the same crate.
- **`cli_subcommands_skill.rs` is the clap subcommand enum module.** Each variant moves to its own `cli_skill_<subcommand>_cmds.rs` (existing pattern).
- **`pool.rs` extraction** — the `BucketRegistry` (added by #1806 rate-limit work) is the obvious extract; `PoolGuard` + `PoolKey` + `PoolStats` stay in pool.rs.
- **Closes audit finding P1-1.** Headroom regained for next wave of feature work.
