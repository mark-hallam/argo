# claude-canonicalize-medium-duplicates — Resolve 4 medium-severity duplicate types

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-canonicalize-medium-duplicates`.

## Scope

Phase A audit identified 4 medium-severity duplicate type definitions. Resolve in one bundled brief.

**Current state** (verified by grep at HEAD):

### JudgeVerdict (3 definitions)
- `crates/argo-types/src/judge.rs:43` — `pub enum JudgeVerdict` (← **already canonical**)
- `crates/argo-genesis/src/genesis_judge.rs:45` — `pub enum JudgeVerdict` (duplicate)
- `crates/argo-vertical/src/eval_score_judge.rs:50` — `pub struct JudgeVerdict` (different shape — possibly a different concept)

### Role (2 definitions)
- `crates/argo-identity/src/types_role.rs:284` — `pub struct Role` (← **likely canonical**; identity owns workers/roles)
- `crates/argo-types/src/capability.rs:219` — `pub struct Role` (capability-system role; possibly different concept)
- Also `crates/argo-vertical/src/manifest.rs:116` defines `RoleSection` (manifest section, different concept — fine).

### Outcome (2 definitions)
- `crates/argo-executor-computer-use/src/executor_types.rs:65` — `pub enum Outcome` (CU executor outcome)
- `crates/argo-store/src/handoff_stats_query.rs:22` — `pub enum Outcome` (handoff stats outcome)

These are likely DIFFERENT concepts. Rename rather than merge.

### RestartDecision (2 definitions)
- `crates/argo-runtime/src/oom_restart.rs:190` — `pub enum RestartDecision` (OOM restart)
- `crates/argo-executor-computer-use/src/restart.rs:83` — `pub enum RestartDecision` (CU restart)

Likely different concepts; rename.

**Strategy**: per-type triage — read each duplicate, decide if same concept (consolidate) or different concept (rename for clarity). Keep changes minimal and behavior-preserving.

LOC: ~600 across all 4 types.

## Files you may touch

For each type, the relevant `.rs` files cited above + their immediate test siblings + call sites surfaced by `rg`. Cargo.toml dep edges may need updating if a non-canonical crate didn't previously depend on the canonical-home crate. ≤5 modified lines per Cargo.toml.

## Files you must NOT touch

- Anything outside the brief's may-touch set per CLAUDE.md.
- The audit chain hashing logic in `argo-audit` (separate concern).

## Acceptance criteria

- [ ] For each of {JudgeVerdict, Role, Outcome, RestartDecision}: either ONE canonical home with re-exports, OR clearly-named distinct types (e.g., `CuOutcome` vs `HandoffOutcome`).
- [ ] `rg 'pub (struct|enum) (JudgeVerdict|Role|Outcome|RestartDecision)\b' crates/` returns minimum-necessary matches (1 per concept).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] PR description documents the per-type decision (consolidated vs renamed-as-distinct) for review.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-canonicalize-medium-duplicates`
- PR title: `refactor: canonicalize 4 medium-severity duplicate types (claude-canonicalize-medium-duplicates)`

## Notes for the implementer

- Mission-claim alignment: production hardening (type clarity across substrate).
- Per-type triage process:
  1. Read all definitions.
  2. Compare variants/fields. Same → consolidate. Different → rename.
  3. Document the call: "JudgeVerdict: consolidate to argo-types::JudgeVerdict (genesis duplicate removed); vertical's was a different shape, renamed to `EvalJudgeScore`."
- Drift-prevention rule applied: brief removes parallel surfaces, doesn't add any.
