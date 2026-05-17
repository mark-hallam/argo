# codex-split-7-over500-files — drop all source files below the 500-LOC ceiling (P0 audit-finding-1b)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high — judgment on cut-lines per file.
**Depends on**: none.
**Authored against**: origin/main `6b432ec4` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-split-7-over500-files`.
**Source**: 2026-05-17 read-only audit finding #1b (audit lines 40-45). Plus 1 NEW violator since audit: `cli_command_enum.rs` at 503 (audit had it at 498 pre-#1778).

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — `loc-budget` is a CI invariant. Currently RED on main; admin-merges bypass it.
- **Repeatability** (#3) — smaller files are debugged + replaced faster.

## Scope

P0. Split each of these 7 files into thinner sibling modules, dropping every one below the 500-LOC ceiling. Goal: every file ≤ 450 LOC (leaves ~50 LOC of headroom for future additions).

### File targets

| File | Current | Target | Strategy |
|---|---:|---:|---|
| `crates/argo-cli/src/handlers/connector_authorize.rs` | 819 | ≤450 | Extract per-provider OAuth flows (oauth_google, oauth_slack, oauth_stripe) into sibling files |
| `crates/argo-cli/src/handlers/doctor_fix.rs` | 619 | ≤450 | Extract per-check fix actions into `doctor_fix_<check>.rs` siblings |
| `crates/argo-cli/src/handlers/worker_actions_auto_approval_tests.rs` | 547 | ≤450 | Split test module by scenario group (regression / unit / integration) |
| `crates/argo-cli/src/handlers/gateway/routes_orchestrator.rs` | 533 | ≤450 | Extract per-route handlers into sibling files (handoffs, slas already split — find the next group) |
| `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs` | 512 | ≤450 | Extract row-render helpers into `routes_audit_detail_render.rs` |
| `crates/argo-cli/src/handlers/gateway/routes_worker_overview_tests.rs` | 507 | ≤450 | Split test module by KPI group (was already split once; second pass needed) |
| `crates/argo-cli/src/cli_command_enum.rs` | 503 | ≤450 | Extract sub-command derives into per-noun files (audit `cli_<noun>_cmds.rs` pattern) |

### Discipline

- **No behavior change.** Pure mechanical extraction.
- **`pub(super)` visibility** for extracted helpers — sibling modules, not public API.
- **All extracted helpers ≤ 80 LOC** (function-LOC budget invariant from F-09).
- **Tests stay green without modification** — extracted helpers are private; existing tests still call the same public surface.

LOC: ~500-700 net (lines move between files; net additions for module declarations).

## Files you may touch

For each of the 7 files above: the file itself + 2-4 new sibling files. Plus the parent `mod.rs` for module registration. Per-file budget:

- 7 source files (existing) — net trim. ≤200 modified LOC each.
- ~15-20 new sibling files (NEW). ≤200 LOC each.
- 2-3 `mod.rs` updates — module declarations. ≤8 modified LOC each.
- Sibling test files — only if a test calls something that moved; mechanical import updates. ≤15 modified LOC each.

## Files you must NOT touch

- Behavior. Pure mechanical extraction.
- Public API surface — `pub fn` / `pub struct` declarations stay where they are (in the original file).
- The CI `loc-budget` workflow — consume only.
- Anything outside the 7 listed files + their sibling helpers.

## Acceptance criteria

- [ ] `find crates -name '*.rs' -exec wc -l {} \; | awk '$1 > 500 {print}'` returns empty.
- [ ] All 7 target files ≤ 450 LOC.
- [ ] All extracted helpers ≤ 80 LOC.
- [ ] No public API change (verify with `git diff --stat` filtered on `pub fn` / `pub struct` / `pub enum` lines).
- [ ] `cargo test --workspace` does not regress against pre-split baseline (compare test counts).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-split-7-over500-files`
- Title: `refactor: split 7 over-500-LOC files to close loc-budget gate (codex-split-7-over500-files)`
- Worker verification: crate-scoped clippy + tests + workspace clippy + `loc-budget` script per SWARM_PROTOCOL.md.

## Hidden constraints

- **`connector_authorize.rs` (819 LOC) is the hardest.** It's a 5-provider OAuth state machine; per-provider extraction is the natural cut.
- **Test files (`*_tests.rs`) split by scenario, not arbitrary 200-line chunks.** Group by what the tests are proving (regression vs unit vs integration).
- **`cli_command_enum.rs` is the clap subcommand enum.** Each variant moves to its own `cli_<noun>_cmds.rs` (the pattern already exists for many subcommands). The enum itself stays in `cli_command_enum.rs` — just re-export each variant from its sibling.
- **Don't introduce a new crate.** Each extraction is a sibling module in the same crate.
- **Order doesn't matter** (no cross-file deps in this list), but tackle the largest first so headroom is visible early.
- **Audit closes 1b.** When this merges, finding #1b is resolved on main.
