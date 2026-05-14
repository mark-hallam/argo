# claude-ci-lint-parallel-pub-types — CI lint to prevent new duplicate pub types across crates

**Owner**: claude. **Depends on**: nothing (but most useful after the 4 canonicalization briefs land).
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-ci-lint-parallel-pub-types`.

## Scope

`scripts/lint-brief.sh` already detects parallel-name briefs and parallel-surface declarations *in brief text*. Phase A audit showed 11+ duplicate `pub struct/enum` definitions snuck into committed code despite the brief-time check — because workers can violate may-touch and add new pub types without the brief saying so.

Extend the lint to scan **committed code**, not just briefs:

1. Walk `crates/*/src/**/*.rs`.
2. Find every `pub struct <Name>` / `pub enum <Name>` / `pub type <Name>`.
3. Group by name. Any name with ≥2 crates → flag.
4. Maintain an **allowlist** for legitimate cases (e.g., trait impls, intentional re-exports under same name).
5. Wire into CI as a non-blocking warning initially (WARN mode); flip to enforce after a 2-week stabilization window.

LOC: ~400 (one Python script + CI workflow snippet + initial allowlist).

## Files you may touch

- NEW `scripts/lint-parallel-pub-types.py` — the scanner. Reads crates/, emits findings to stdout, exits 0 in WARN mode and 1 in --enforce mode.
- NEW `scripts/lint-parallel-pub-types.allowlist` — list of `<TypeName> <crate1> <crate2>` triples that are intentional duplicates (re-exports, trait method name overlap, etc.).
- `.github/workflows/lint.yml` (or equivalent existing workflow) — add `python3 scripts/lint-parallel-pub-types.py` as a non-blocking step. ≤15 modified lines.
- `scripts/lint-brief.sh` — narrowly: add a one-liner that runs the new script with `--check-changed-files` mode for staged files in pre-commit hook. ≤10 modified lines.
- `docs/skycot/DRIFT_PREVENTION.md` — narrowly: add a §3.5 "CI-side parallel-pub-types lint" with usage notes. ≤30 modified lines.

## Files you must NOT touch

- Existing rust source files (the lint is read-only over them).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `python3 scripts/lint-parallel-pub-types.py` runs successfully on current `crates/`.
- [ ] Output matches the audit findings: AuditRow (4x), Decision (2x), ApprovalDecision (2x), JudgeVerdict (3x), Role (2x), Outcome (2x), RestartDecision (2x) — UNLESS the canonicalization briefs have already landed.
- [ ] Allowlist file is documented + has at least 1 example entry showing legitimate intentional duplication.
- [ ] CI workflow added in WARN mode (exits 0; logs findings).
- [ ] `scripts/lint-parallel-pub-types.py` has `--enforce` flag for future strict mode.
- [ ] `bash scripts/lint-brief.sh` self-test passes (the script integration didn't break brief-time lint).
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-ci-lint-parallel-pub-types`
- PR title: `tooling: CI lint for parallel pub types across crates (claude-ci-lint-parallel-pub-types)`

## Notes for the implementer

- Mission-claim alignment: production hardening (prevents the class of drift that the canonicalization briefs are cleaning up).
- The script's job is just detection + reporting. It doesn't enforce a fix; it gives the operator a signal.
- Default WARN mode prevents the lint from accidentally breaking active worker PRs during the canonicalization rollout.
- Flip to `--enforce` mode by config (project-level config flag, similar to `git config skycot.brief-lint enforce`) after canonicalization briefs (audit-row, decision, approval-decision, medium-duplicates) all land.
- Drift-prevention rule applied: brief adds NEW pub surfaces only in tooling scripts (no Rust pub structs added).
