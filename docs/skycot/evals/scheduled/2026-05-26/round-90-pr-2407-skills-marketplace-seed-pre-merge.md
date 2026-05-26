# Round 90 — PR #2407 skills-marketplace demo seed — PRE-MERGE re-verification on head 8e47bfaba

- **Date**: 2026-05-26 (UTC) — wall clock at session start 16:10:00Z
- **Round**: 90
- **Pillar / Claim**: Claim 4 (Skills shared + learned) — demo-MP4 Scene 4 marketplace flow; **4-of-4 demo-MP4 P0**
- **Verdict**: 🟢 PRE-MERGE PASS with **MERGE-ORDER CAVEAT (P1)**
- **Severity**: N/A (PASS) · merge-order caveat tracked as P1

## Setup

- Worktree `/tmp/eval-r90` at `origin/main` (top commit `01e220756`).
- `git fetch origin pull/2407/head:pr-2407` → checkout PR head `8e47bfaba`.
- `git merge-base origin/main pr-2407` = `f2b37c86e` (`status: claim fix-skills-marketplace-perpetual-loading`). True PR diff is +365 / −0 across 10 files (matches GitHub).
- `CARGO_TARGET_DIR=/tmp/eval-r90-cargo-target cargo test -p argo-demo-seed --lib marketplace` — cold build 4m59s, test runtime 0.42s.
- Source-equivalence reads of the new files: `crates/argo-demo-seed/src/marketplace.rs` (+176), `crates/argo-cli/src/handlers/gateway/demo_seed_skills.rs` (+101), boot wiring at `crates/argo-cli/src/handlers/gateway/boot_serve.rs:91`.

## Observed

**Cargo test result** — 4/4 PASS:

```
running 4 tests
test marketplace::tests::includes_pricing_competitor_scrape ... ok
test marketplace::tests::rows_are_tenant_scoped ... ok
test marketplace::tests::second_run_is_noop ... ok
test marketplace::tests::seeds_full_catalog_for_tenant ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.42s
```

**Production wire chain** (Scene 4 demo path):
- Boot side (`boot_serve.rs:91`): `super::demo_seed_skills::seed_marketplace_if_demo_mode(s).await;` runs unconditionally on every gateway boot; guard is internal `if !super::demo_mode::is_enabled()` short-circuit, idempotent.
- Workspace resolution (`demo_seed_skills.rs:39-54`): `store.ensure_workspace(workspace_name, &root)` returns the canonical tenant_id under the gateway's CWD-bound workspace — the SAME tenant the `/api/v1/skills` route will read from.
- Seed call (`demo_seed_skills.rs:55`): `argo_demo_seed::seed_demo_marketplace_skills(store, tenant_id)`.
- Insert path (`marketplace.rs:78-94`): iterates 7 catalog rows → `SkillStore::find_active_by_name(row.name)` skip-if-present → `SkillStore::install_for_tenant(&raw, &tenant_str, None)` real DB write. Provenance `published:verified:argo-seed` matches the gateway's `source LIKE 'published:%'` catalog filter at `routes_skills_list_get.rs` (round 87 prior verification).

**LOC budget on gateway/mod.rs** — ⚠️ MERGE-ORDER CAVEAT:
- Current `origin/main`: `crates/argo-cli/src/handlers/gateway/mod.rs` = **503 LOC** (already 3 over the 500 ceiling — pre-existing overage from prior commits).
- After PR #2407 merge: 503 → 504 LOC (PR adds `pub mod demo_seed_skills;` declaration at line ~).
- Fix path: PR #2422 `fix(gateway): extract store helpers from mod.rs to land under 500 LOC` MERGEABLE/OPEN at head `dbcecfc3c` — extracts 6 leaf helpers, brings mod.rs 503 → 424 LOC. Round 89 PRE-MERGE PASS already.
- **Merge ordering**: #2422 MUST land before #2407. Conductor authored this split brief at T14-12 (commit `a3b4cc54a`) for exactly this reason.

## Anti-theater check

Three of the four tests query the raw `skill` SQLite table via `sqlx::query("SELECT COUNT(*) FROM skill WHERE …")` and bind the test's own `tenant_id`. The production write goes through `SkillStore::install_for_tenant` → migration 0xxx skill table → same row the SQL `SELECT COUNT(*)` reads. The test cannot be made green by rewriting an assertion against a function return value — it reads what the production code wrote, by raw SQL, in a fresh `:memory:` DB.

`seeds_full_catalog_for_tenant` further pins `assert!(inserted >= 6, "brief requires >= 6 marketplace skills")` — a direct enforcement of the brief's success criterion at the test layer.

The Scene-4 narration anchor row `pricing.competitor_scrape` is explicitly tested by name (`includes_pricing_competitor_scrape`). If a future refactor changes the catalog order or the headline row's name, this test fails.

`second_run_is_noop` asserts second call returns `0` inserts — confirms the `find_active_by_name` idempotency gate is real, not just a code path that runs once at boot.

The boot-wiring is verified by reading `boot_serve.rs:91` directly (not by a mock test that simulates boot) — confirms the seed actually runs on real gateway boot when `ARGO_DEMO_MODE=1`.

Operator NO-THEATER rule honoured: Option B (real backend seed via `install_for_tenant` → real `skill` table writes) is the design, not Option A (mock-bypass / fixture file). Re-confirmed.

## Gap / follow-up brief candidates

- **P0 MERGE ORDER**: merge PR #2422 FIRST (mod.rs split) → THEN PR #2407 (this PR). Without #2422 first, #2407 trips the LOC-budget gate (504 > 500). Both are evaluator-pre-verified for `--skip-clippy` admin-merge.
- No new brief candidates — PR is internally complete + anti-theater dispositive.
- Closes the 4-of-4 demo-MP4 P0 set. Remaining gates after merge are operator-only (macOS perms grant + recording).

## Token-budget note

Cold cargo build for the `marketplace` test set was 4m59s wall (incremental hit `argo-demo-seed` at the leaf — workspace-wide rebuild from `argo-audit` upward). Test runtime 0.42s — the fastest part of the verification. Total round budget approx 28K tokens used by end of S1.
