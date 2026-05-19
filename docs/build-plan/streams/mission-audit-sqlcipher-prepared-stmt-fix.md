# Stream Audit-P1-1 — fix prepared-statement lint failure on PRAGMA key

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-sqlcipher-prepared-stmt-fix`.
**Source**: 2026-05-19 codex audit P1-5 (sub-issue 1), validated by claude `VALIDATION.md` (CONFIRMED — `check-prepared-statements.sh` fails on `store_open.rs:114`, the `audit-allow:` comment is at line 142, 27 lines too far from the `format!` to satisfy the 3-line rule).

## Scope

`scripts/check-prepared-statements.sh` is failing on one site:

```
FAIL: undocumented SQL interpolation via format!() (1 site(s)):
  crates/argo-store/src/store_open.rs:114: PRAGMA key = "x'{}'"
```

The interpolation IS validated — `encryption::validate_hex_key(hex)?` runs immediately before — but the `audit-allow:` comment that whitelists this pattern is 27 lines below at line 142, outside the lint's 3-line check window. The fix is purely mechanical: move the comment, or restructure the `format!` call so the comment + format are adjacent.

This is the smallest brief in the batch. Mechanical fix; no behavior change. Restores a green safety gate that should be green.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/store_open.rs` — restructure the `PRAGMA key` block so an `// audit-allow: SQLCipher PRAGMA literal, value comes from encryption::validate_hex_key()` comment sits within 3 lines above the `format!("PRAGMA key = \"x'{}'\"", ...)` call at line 114. Two options the implementer may choose between: (a) move the existing comment up; (b) extract the `format!` into a small named helper `build_pragma_key(hex: &str) -> ArgoResult<String>` co-located with the comment. Pick whichever is cleaner; document the choice in the PR body.
- `crates/argo-store/src/store_open_tests.rs` (if it exists, otherwise create as a sibling) — a single test confirming `build_pragma_key` (or the post-refactor equivalent) produces the expected string for a known-good hex key. Optional but encouraged.

## Files you must NOT touch

- `scripts/check-prepared-statements.sh` — the lint is correct; the code is wrong. Do NOT loosen the lint.
- `crates/argo-store/src/encryption.rs` — `validate_hex_key` is the right gate, untouched.
- Any callers of `store_open` — signature stays the same.

## Acceptance criteria

- [ ] `bash scripts/check-prepared-statements.sh` exits 0 (green) on the resulting tree.
- [ ] No behavior change: same PRAGMA string is generated for any given hex key.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-store` passes.
- [ ] Modified file < 500 LOC (the existing file is small; this should not approach the limit).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-audit-sqlcipher-prepared-stmt-fix`
- PR title: `store: move audit-allow comment to satisfy prepared-statement lint (mission-audit-sqlcipher-prepared-stmt-fix)`

## Notes for the implementer

- The `audit-allow:` comment text should be specific. Recommended: `// audit-allow: SQLCipher PRAGMA literal, hex value pre-validated by encryption::validate_hex_key()`. Avoid generic "trust me" comments.
- If choosing the helper-function approach, place `build_pragma_key` immediately above the call-site; do not move it to a separate module — locality is what makes the lint readable.
- After landing, `scripts/check-prepared-statements.sh` should be part of every conductor verification step (already in PROTOCOL_LOOP_PROMPT.md if I'm reading the swarm protocol right).

## Out of scope (follow-up briefs)

- The broader SQLCipher enterprise-default question (default `key_source=none`, no fail-closed enterprise mode) — that's `mission-audit-sqlcipher-enterprise-default`.
- A reusable PRAGMA-builder helper for future encrypted-key rotation flows — premature; come back if a second call-site emerges.
