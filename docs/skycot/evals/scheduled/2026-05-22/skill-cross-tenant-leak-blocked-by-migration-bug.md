# Eval: Skills — cross-tenant leak negative test (blocked by P0 migration regression)

**Date**: 2026-05-22
**Pillar**: Skills shared + learned
**Mission claim**: 4 — Skills shared and learned · **6 — Multi-tenant by construction**
**Verdict**: 🔴 **NOT-COVERED** — the test cannot run; **a separate P0 migration bug landed today blocks all fresh-DB cross-tenant tests**.

## Setup

The intended scenario:

1. tenant_a creates a learned skill (`tenant_private_price_check`) → installed in `argo_skill::SkillStore` under tenant_a.
2. tenant_b boots a separate `SkillStore` (different workspace, different library).
3. `LayeredLookup::with_library(workspace_b, library_b)` is asked to render the worker prompt for tenant_b.
4. **Assert** the rendered prompt does NOT contain `tenant_private_price_check` (the tenant_a skill name).

This exact test already exists at [crates/argo-skill/tests/end_to_end_claim4_loop.rs:54](crates/argo-skill/tests/end_to_end_claim4_loop.rs) — `claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak`. It's labelled as the mission-claim-4 closed-loop test. I ran it.

## Action

```bash
cargo test -p argo-skill --test end_to_end_claim4_loop \
  claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak \
  -- --nocapture
```

## Expected

Test passes. The assertion `assert!(!other_prompt.contains("tenant_private_price_check"))` proves cross-tenant isolation at the SkillStore level.

## Observed

```
running 1 test

thread 'claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak' (152576041)
panicked at crates/argo-skill/tests/end_to_end_claim4_loop.rs:192:27:
migrate store: Provider("v43 up: error returned from database:
                       (code: 1) near \"operator\": syntax error")

test claim4_learned_skill_publishes_and_reloads_without_cross_tenant_leak ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
```

The test panicked at line 192 — `let other_tenant_workspace = skill_store().await;` — before reaching any cross-tenant assertion. The migration to v43 itself fails on the fresh `:memory:` SQLite store.

## Anti-theater check + root cause

Direct file evidence — the SQL migration at `crates/argo-store/migrations/47__skill_canary.sql:6` (read by `crates/argo-store/src/migrations/v43.rs:21` via `include_str!`):

```sql
-- catalog by `skill_id`; operator-driven and not tenant-partitioned.
```

The `;` after `skill_id` is **inside an SQL line comment** (line starts with `--`). The migration wrapper at [v43.rs:25](crates/argo-store/src/migrations/v43.rs) splits the SQL into statements with a comment-blind splitter:

```rust
for stmt in V43_UP_SQL.split_terminator(';') {
    let sql = stmt.trim();
    if sql.is_empty() { continue; }
    sqlx::query(sql).execute(&mut **tx).await.map_err(...)?;
}
```

`split_terminator(';')` splits on the raw character `;` without recognising `--` line comments. The chunk after the first `;` therefore begins with ` operator-driven and not tenant-partitioned.` — which SQLite parses as raw SQL and reports `near "operator": syntax error`.

### Git blame

```bash
git log --oneline -S 'skill_id`;' -- crates/argo-store/migrations/47__skill_canary.sql
# → c92abfa7f audit — migration tenant-id coverage — SAFETY comments on 12 global tables (#1988)
```

[#1988](https://github.com/anthropics/argo/pull/1988) landed today (commit `c92abfa7f`). It added "SAFETY:" comment blocks to 12 migration files. **48 migration SQL files now have at least one semicolon inside a `-- ` comment**:

```
25  40  41  42  46  47  48  49  50  53  54  56  57  58  59  61  62  63  65  70  71
72  73  74  77  79  91  95  97  99  102 103 106 107 109 110 111 112 115 116 117 118
119 120 121 122 123 124 125 127
```

Only the 3 wrappers v43.rs (slot 47), v47.rs (slot 51), and v51.rs (slot 53) use the fragile `split_terminator(';')` pattern. v43 is the first one a fresh-from-zero migration hits, so it's the visible blast-radius. Beyond v43, other wrappers may share the same bug.

## Why the swarm hasn't seen this

1. **CI = `cargo build`, not `cargo test --test '*'`.** The SQL files are `include_str!`-baked at compile time but never *executed* until a test or boot runs migrations against a real SQLite engine. `cargo build` is silent.
2. **The running gateway (PID 28198) was started 92 hours ago at schema v70.** Its `~/.argo/sessions.db` already has v43 applied — the wrapper is skipped on every restart. The bug only fires for fresh-from-zero migrations.
3. **PR #1988 was a "documentation" change** ("SAFETY: comments on 12 global tables"). It looked harmless in review.

## Impact

🔴 **Mission claim 4 closed-loop test is FAIL on main.** The test that proves "skills don't leak across tenants" cannot run.

🔴 **Any new operator who installs argo today** and boots it against a fresh `~/.argo/sessions.db` will hit `v43 up: ... near "operator": syntax error` and the gateway will refuse to start.

🟡 **Other downstream tests likely broken** — e.g. any integration test that calls `SessionStore::open(":memory:").await; store.migrate().await` is exposed to v43+ slot bugs. The workflow_edit_api_smoke test (still running at time of writing) may hit a similar wall.

## Gap / follow-up brief candidates

**🔴 P0 — `claude-migration-comment-aware-splitter`**: Replace the `split_terminator(';')` pattern in `v43.rs`, `v47.rs`, `v51.rs` with a parser that:
- Strips `-- ...\n` line comments before splitting.
- Also handles `/* ... */` block comments (none exist today, but defensive).
- A unit test that pins a string containing a semicolon inside a `-- ` comment and asserts the splitter doesn't see it.

Alternatively (lower-LOC fix): pre-filter each line by `if line.trim_start().starts_with("--") { skip }` before concatenating into statement chunks.

Acceptance:
- `cargo test -p argo-skill --test end_to_end_claim4_loop` passes.
- `cargo test -p argo-cli --test workflow_edit_api_smoke` passes.
- New unit test in `crates/argo-store/src/migrations/` asserts a comment containing `;` does not break statement splitting.

**🟡 P1 — `claude-migration-runner-canonicalise`**: There are at least 3 wrappers (v43, v47, v51) that hand-roll a SQL splitter. Other wrappers use `sqlx::migrate!()` or direct embedded statements. Consolidate to one execution path so the fix above only needs to land once.

**🟡 P1 — `claude-migration-smoke-fresh-zero-to-head`**: A CI smoke that opens `:memory:`, runs `migrate()` from v0 → v76, asserts success. Would have caught #1988 the moment it landed.

**🟢 INFO**: The PR #1988 changed all 12 migration files in one go — but the comment-format defect (`text; more text`) only affects wrappers whose own `up()` uses the broken splitter. The conductor's PR review on #1988 should be updated to require migration files only be edited alongside their wrapper if the wrapper has the fragile splitter.

## What's true about cross-tenant skill isolation despite the test being blocked

Static reading of `crates/argo-skill/src/library.rs:LayeredLookup::with_library` shows the lookup chains workspace ↔ library by Arc, never cross-pollinates between two workspace instances, and renders prompts per-workspace. The architecture *looks* sound. I just cannot prove it end-to-end on `:memory:` until P0 above lands.

Reference for cross-tenant skill isolation work-done-on-the-real-gateway: PR #1702 ("cross-tenant negative test") merged 2026-05-15 and per the operator's memo `project_v1_ship_blocker_skill_library_load.md` is wired live at `crates/argo-agent-runner/src/turn_helpers.rs:65`. That live path is separate from this test, but the test was supposed to be the regression gate.
