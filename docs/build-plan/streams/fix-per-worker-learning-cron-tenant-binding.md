# fix-per-worker-learning-cron-tenant-binding

**Owner**: claude or codex — WS1 tenant-isolation class → conductor adversarial review before merge.
**Depends on**: dec042-p1a-distilled-skill-tenant-binding MERGED (#2836, `cd9eea87b` — provides the `*_for_tenant` propose chain to reuse).
**Authored against**: origin/main `cd9eea87b` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-per-worker-learning-cron-tenant-binding

## Problem (same class as p1a; found by the #2836 adversarial review)

`crates/argo-skill/src/per_worker_learning_cron.rs:110` proposes overlay
skills via the legacy `propose_for_review_judged(...)` — which lands the row
under `PLACEHOLDER_TENANT_ID` — even though the cron's READ side is
tenant-scoped. Same wrong-tenant binding p1a fixed for the distillation
path, on the per-worker overlay path. (p1a deliberately scoped this out and
pinned the legacy shim's placeholder behavior; this brief is the named
follow-up.)

Secondary, same review: `find_active_by_name`
(`crates/argo-skill/src/store_lookups.rs:52-59`) is GLOBAL — one tenant's
active skill name blocks another tenant's proposal (minor cross-tenant
denial / name-probing channel). Fix it here if the lookup grows a
tenant-aware variant cleanly; otherwise note it for its own brief in the PR
body.

## Fix

Reuse the p1a `*_for_tenant` chain (NO new parallel propose surface —
grep-before-write): thread the worker's tenant from the per-worker cron's
read scope into the proposal. Mirror p1a's test pattern
(`pending_review_tenants`-style pin + cross-tenant negative). Check other
remaining production callers of the legacy shims
(`crates/argo-executor-computer-use/src/learn.rs:248`,
`crates/argo-cli/src/handlers/skill_review_candidates.rs:140`) and either
fix in-scope if the tenant is available at those call sites, or list them
in the PR body as explicitly-out-of-scope with the reason.

## May touch

- `crates/argo-skill/src/per_worker_learning_cron.rs` (+ tests)
- `crates/argo-skill/src/store_lookups.rs` (+ tests, only for the
  tenant-aware lookup variant)
- the two legacy caller files above IF fixed in-scope

## Acceptance / verifier

1. Per-worker overlay proposal lands under the worker's tenant (test would
   FAIL on current main).
2. Cross-tenant negative pin (tenant B never sees A's overlay proposal).
3. `cargo test -p argo-skill` green; clippy clean; no new pub types.

~40–90 LOC. **🟠 P2 — WS1 class, prerequisite-adjacent to p1b/p1c wiring
real producers onto this loop.**
