# Stream fix-cli-skill-search-indirect-placeholder-fallback-error-message — clearer error when `argo skill search` indirect path falls back without a session

**Owner**: codex or claude (CLI; ~30 LOC; fast-mode candidate).
**Depends on**: none.
**Authored against**: origin/main `234ffd9f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cli-skill-search-indirect-placeholder-fallback-error-message`.

🟢 **P3 UX fix.** Source: round-52 evaluator finding (k) carried in r53 SHARED.md handoff.

## Scope

`crates/argo-store/src/skill_query.rs:57` `search_indirect` path is invoked when `argo skill search` runs without an explicit `--tenant` flag AND no session exists for the requested vertical. The current behavior surfaces an opaque error like:

```
Error: skill_search indirect: invalid UUID: invalid character: found 'X' at position N
```

This happens because the indirect path tries to bind a session's `tenant_id` (BLOB / UUID-typed) but no session row exists, so the fallback hits the binding code with a placeholder or empty string, which trips `Uuid::parse_str` with a parse-error message that's meaningless to operators.

The fix is to detect the no-session condition BEFORE the UUID parse + surface a clear message:

```
Error: no sessions found for vertical 'X'. Pass --tenant <tenant_id> explicitly,
       or create a session for this vertical first via `argo session create -v X`.
```

This is purely a UX-layer fix — no behavior change, no security implication (the rejection still happens, just with a better error message). Not a security hole — just an operator-papercut.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`crates/argo-store/src/skill_query.rs`]

## Files you may touch

- `crates/argo-store/src/skill_query.rs:100-120` — `search_indirect` function. Before the bind, check whether any session row exists for `vertical_id`; if not, return a typed error variant (e.g. `ArgoError::Configuration { hint: "no sessions for vertical 'X' — pass --tenant or create a session" }`).
- `crates/argo-cli/src/handlers/skill_search.rs` — adjust the error-to-user mapping if needed to surface the new variant clearly.
- `crates/argo-store/tests/skill_query_tenant_bind_pin.rs` (added in #2260) — add 1 test case: empty session table + indirect query → assert the error is the new typed variant, not a UUID parse error.

## Tests

- 1 unit test (~20 LOC) for the new error path.
- Manual smoke: `cargo run -p argo-cli -- skill search foo` against an empty workspace.

## Definition of done

- `argo skill search <name>` on an empty workspace returns the helpful message, not the UUID parse error.
- `cargo test -p argo-store --test skill_query_tenant_bind_pin` green.
- PR body includes a before/after CLI output excerpt.

Notes: pure mechanical + a typed error variant. Fast-mode appropriate. Don't expand scope to other CLI error paths in this brief.
