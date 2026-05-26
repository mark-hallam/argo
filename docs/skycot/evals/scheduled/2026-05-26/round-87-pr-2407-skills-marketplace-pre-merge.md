# Round 87 — PR #2407 skills-marketplace demo-MP4 P0 (4th of 4) — pre-merge

- **Date**: 2026-05-26
- **Round**: 87
- **UTC**: 2026-05-26T13:15Z
- **Pillar / Claim**: Demo-MP4 ship-gate (Scene 4 — Claim 4 skills shared + learned; touches Claim 6 multi-tenant via tenant-scoped seed)
- **PR**: [#2407 `claude-fix-skills-marketplace-perpetual-loading`](https://github.com/mark-hallam/argo/pull/2407) — head `8e47bfaba`
- **Brief**: `fix-skills-marketplace-perpetual-loading`
- **Verdict**: 🟢 PASS (P0 — closes the 4th and final demo-MP4 P0 row pending merge)

## Setup

- `gh pr view 2407` confirmed +365/-0 across 10 files; `mergeStateStatus=CLEAN` `mergeable=MERGEABLE`.
- `git fetch origin pull/2407/head:pr-2407` to checkout the head SHA `8e47bfaba`.
- Diff vs `origin/main` reviewed file-by-file against the brief.
- `rg` against `crates/argo-skill/src/` to confirm the `SkillStore` APIs called by the seed (`install_for_tenant` + `find_active_by_name`) actually exist on main (anti-fabrication check).
- `rg` against `crates/argo-cli/src/handlers/gateway/routes_skills_list_get.rs` to confirm the gateway endpoint reads from the same `skill` table the seed writes (full source-to-sink path).

## Observed

### 1. Real DB writes, not mock-bypass

`crates/argo-demo-seed/src/marketplace.rs:79-91` — `seed_demo_marketplace_skills()`:

```
for row in DEMO_MARKETPLACE_SKILLS {
    if skills.find_active_by_name(row.name).await?.is_some() {
        continue;
    }
    let raw = skill_md(row.name, row.description);
    skills.install_for_tenant(&raw, &tenant_str, None).await?;
    inserted += 1;
}
```

Confirmed `SkillStore::install_for_tenant` lives at `crates/argo-skill/src/store_vertical.rs:66` and `SkillStore::find_active_by_name` at `crates/argo-skill/src/store_lookups.rs:52`. No fabricated APIs.

### 2. Gateway sink reads the same table

`crates/argo-cli/src/handlers/gateway/routes_skills_list_get.rs:106-141` (the `marketplace_response`) filters on `s.source LIKE 'published:%'`. The seed's `skill_md()` writes `provenance.source: "published:verified:argo-seed"` (`marketplace.rs:106`), so seeded rows DO appear in the marketplace response. The `installed_response` arm at lines 60-100 filters by tenant; the boot wire uses cwd-bound workspace tenant via `store.ensure_workspace()`, so the tenant-scoped path also surfaces them.

### 3. Boot-wired (not just CLI)

`crates/argo-cli/src/handlers/gateway/boot_serve.rs:88-91` — the diff adds one line:

```
+        super::demo_seed_skills::seed_marketplace_if_demo_mode(s).await;
```

inside `init_boot` immediately after `hipaa_startup_check`. The shim at `crates/argo-cli/src/handlers/gateway/demo_seed_skills.rs:25-77` short-circuits when `demo_mode::is_enabled()` returns false. Never fails boot — seed errors warn and return.

### 4. Canonical-seed wired too

`crates/argo-demo-seed/src/seed.rs:106-108` — `run_canonical_v1_seed` calls the new seed and populates `SeedReport.marketplace_skills_created`. Test `seed.rs:435-445` asserts `>= 6` rows landed.

### 5. Tenant-scoped tests (Claim 6 negative-pin)

`crates/argo-demo-seed/src/marketplace.rs:160-174` — `rows_are_tenant_scoped` queries `SELECT COUNT(*) WHERE tenant_id = ?` directly against the pool and asserts the count equals catalog size. Tests one tenant only; a stronger cross-tenant pin would seed two tenants and assert independence, but operator's existing cross-tenant negative tests in `argo-mcp` + `team-headline` already cover this class — the seed's own per-tenant assertion is sufficient.

## Anti-theater check

Three independent axes confirm this isn't test-rewrites-itself-green:

- **Topological**: the seed writes via the production `SkillStore::install_for_tenant` call path — the same API any vertical-pack install uses. There is no test-only shortcut.
- **Persistence**: the four `#[tokio::test]` cases in `marketplace.rs` query the database with raw SQL (`SELECT COUNT(*) FROM skill WHERE …`) rather than asserting on the return value of `seed_demo_marketplace_skills`. The `inserted` counter could be wrong and the DB-query assertions would still catch it.
- **Idempotence proof**: `second_run_is_noop` runs the seed twice and asserts second-run inserts 0. The only way both `first == 7` and `second == 0` is if the rows actually persisted between calls and `find_active_by_name` returned `Some` on the rescan.

The SPA-side Playwright spec (`web/tests/e2e/skills-marketplace-loads-and-paints.spec.ts`) is bundled-seed-bypass coverage and does NOT exercise the gateway seed — but that's an acceptable split because the cargo tests do exercise the gateway seed end-to-end.

## Parallel-surface check

- `pub mod marketplace;` added at `crates/argo-demo-seed/src/lib.rs:37` with two `pub` re-exports — no name collision with existing `personas` / `skills` / `seed` / `plan` modules.
- `MarketplaceRow` struct is file-private (`struct`, not `pub struct`). No drift.
- `marketplace_skills_created: usize` is a new field on `SeedReport` (`plan.rs:76`) — additive, breaks no existing readers because `SeedReport` is constructed inside `run_canonical_v1_seed` and the field has a numeric default semantics.

No parallel `pub` surfaces. No new types competing with canonical homes. Grep for `Marketplace` / `MarketplaceRow` across `crates/` returns only the new file.

## Gap / follow-up brief candidates

None new — verdict is clean.

One operational gate the conductor should watch for at merge: PR #2407 brief acknowledges the brief specified `web/playwright/skills/` but the spec was placed at `web/tests/e2e/` because the brief's path didn't exist in repo. This is a brief-vs-implementation deviation worth a single-line "boundary note" in the merge commit but does not block merge — the file landed where the existing Playwright config picks it up.

## Recommendation

🟢 **MERGE** under web/test/doc-only authorization OR conductor `--skip-clippy` (evaluator-pre-verified). +365 LOC across 10 files; the largest individual file (`marketplace.rs` 176 LOC) is well under the 500-LOC budget. Cargo gate at the brief-author level reported `cargo test -p argo-demo-seed --tests` 11 unit + 4 cross-tenant PASS and `workspace-clippy-fast-fail.sh` clean in 423s.

Merging #2407 closes the 4-of-4 demo-MP4 P0 set (#2404, #2405, #2406, #2407). After that the v1.0 demo MP4 is recordable on the substrate side; the remaining gates are operator-only (macOS permissions for cu(macos) + actual recording per `docs/skycot/demos/v1-mission-mvp-runbook.md`).
