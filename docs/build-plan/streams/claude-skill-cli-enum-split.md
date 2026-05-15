# claude-skill-cli-enum-split — Split `cli_subcommands_skill.rs` to free LOC headroom (unblocks 2+ skill briefs)

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `03573bda` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-skill-cli-enum-split`.

## Scope

Phase D BLOCKED triage (`docs/skycot/audit/2026-05-15-phase-D-blocked-triage.md` §3) identified 2+ BLOCKED briefs whose acceptance would push the skill CLI enum file over the 500-LOC ceiling. Current state:
- `crates/argo-cli/src/cli_subcommands_skill.rs` is 485 LOC (near-limit).
- Any of `mission-skill-publish-verified-ingest`, `mission-skill-explanation-provenance` (and others queued) need to add 1-3 new subcommand variants → pushes file over 500.

Split strategy: extract per-domain sub-enum groups into siblings. Suggested:
- `cli_subcommands_skill.rs` — top-level enum + routing glue (target ~200 LOC)
- `cli_subcommands_skill_lifecycle.rs` — propose/review/publish/promote/unpublish (~150 LOC)
- `cli_subcommands_skill_audit.rs` — explain/provenance/honesty (~100 LOC)
- `cli_subcommands_skill_inspect.rs` — show/list/diff/catalog (~150 LOC)

**Unblocks the downstream briefs** because each now has a narrow sibling to extend, not a single 500-LOC monolith.

LOC: ~200 modified (mostly relocations; behavior-preserving).

## Files you may touch

- `crates/argo-cli/src/cli_subcommands_skill.rs` — split as above. ≤200 modified lines.
- NEW `crates/argo-cli/src/cli_subcommands_skill_lifecycle.rs`
- NEW `crates/argo-cli/src/cli_subcommands_skill_audit.rs`
- NEW `crates/argo-cli/src/cli_subcommands_skill_inspect.rs`
- `crates/argo-cli/src/cli_command_enum.rs` (or wherever the parent enum lives) — narrowly: import the new sub-enums. ≤20 modified lines.
- `crates/argo-cli/src/lib.rs` — narrowly: `pub mod cli_subcommands_skill_*;` declarations. ≤10 modified lines.

## Files you must NOT touch

- Skill handler code in `crates/argo-cli/src/handlers/skill*.rs` — separate concern; this is the **enum dispatch layer**, not the handlers.
- argo-skill production code.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `cli_subcommands_skill.rs` < 250 LOC post-split.
- [ ] Each new sibling < 200 LOC.
- [ ] No subcommand behavior change (pure refactor; relocated arms).
- [ ] `cargo test -p argo-cli skill` passes (focused subset).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] All existing CLI subcommands still discoverable via `argo skill --help`.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-skill-cli-enum-split`
- PR title: `refactor(cli): split cli_subcommands_skill.rs into domain-scoped siblings (claude-skill-cli-enum-split)`

## Notes for the implementer

- Mission-claim alignment: production hardening (unblocks 2+ downstream skill CLI briefs in BLOCKED state).
- High-leverage: this single brief unlocks 2-3 BLOCKED rows.
- Pure refactor; behavior must be byte-identical post-split.
- Drift-prevention: adds NEW per-domain siblings; no parallel surface — extends existing enum dispatch.


## Brief redesign 2026-05-15 evening — SUPERSEDES original split strategy

Codex correctly identified that the original brief's "split SkillCmd enum variants across sibling modules" is internally inconsistent — Rust enum variants cannot be split across modules while preserving the centralized match dispatch.

**Revised approach: per-handler-fn split, not per-enum-variant split.**

Keep `SkillCmd` enum + match dispatch single-file. Split the HANDLER FUNCTIONS that the match arms call into per-domain siblings:

- `crates/argo-cli/src/cli_subcommands_skill.rs` (485 LOC) — keep enum + match dispatch; each match arm calls into a sibling handler. Should drop to ~250 LOC.
- NEW `crates/argo-cli/src/handlers/skill_lifecycle.rs` — propose/review/publish/promote/unpublish handler bodies. ≤200 LOC.
- NEW `crates/argo-cli/src/handlers/skill_audit.rs` — explain/provenance/honesty handler bodies. ≤150 LOC.
- NEW `crates/argo-cli/src/handlers/skill_inspect.rs` — show/list/diff/catalog handler bodies. ≤200 LOC.
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 new `pub mod` declarations. ≤10 modified lines.

This achieves the LOC headroom the original brief targeted (frees cli_subcommands_skill.rs from being the bottleneck) without trying to fragment the enum itself.
