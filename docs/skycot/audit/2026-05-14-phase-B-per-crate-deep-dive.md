# SKYCOT Phase B Audit — Per-Crate Deep Dive (Top 20 by LOC)

**Authored**: 2026-05-14 by conductor via Explore agent (aca6db95248b00261).
**Scope**: Per-crate deep dive on the 20 heaviest crates from Phase A — orphan pub surfaces, production unwraps, near-limit files, TODO/FIXME accumulations, internal parallel surfaces.
**HEAD at run time**: `66758890`.

## Headline finding

**Codebase is healthier than expected.**
- **0 CLAUDE.md invariant violations** (no production unwrap/expect outside test code; all unsafe blocks are properly-marked FFI shims).
- **1 confirmed orphan** (`FallbackChain::with_operator_preferred()` in argo-channels).
- **30 near-limit files** (400-499 LOC) but only 1 justifies refactoring (`argo-store/migrations/registry.rs`).
- **10 TODO/FIXME comments**, all aspirational/low-priority; none v1-blocking.

## Per-crate summary (highlights)

- **argo-store** (33,073 LOC): 6 files at 400-461 LOC. `migrations/registry.rs` (461 LOC) is the only split candidate. Zero production unwraps. Healthy.
- **argo-skill** (14,206 LOC): `marketplace.rs` 471 LOC; `ir/types_tests.rs` 488 LOC (test file). One TODO at `marketplace_oci.rs:9` about pagination beyond 100 items. Low priority.
- **argo-channels** (8,422 LOC): `fallback.rs:88` has the one confirmed orphan — `FallbackChain::with_operator_preferred()` pub method never called in workspace.
- **argo-genesis** (7,809 LOC): `broad_role.rs` 473 LOC, `pipeline.rs` 452 LOC. Both load-bearing. No orphans.
- **argo-vertical** (5,736 LOC): `manifest.rs` 473 LOC. Core validation; keep intact.
- **argo-runtime** (5,971 LOC): 3 documentation-anchor TODOs. No gaps.
- **argo-executor-computer-use** (6,217 LOC): 2 guarded `unsafe` blocks for test-env mutation (marked SAFETY). One profile-pinned tokens-per-dollar TODO; low priority.

## Top orphan pub surfaces

| Rank | Crate/File:Line | Name | Action |
|---|---|---|---|
| 1 | `argo-channels/fallback.rs:88` | `FallbackChain::with_operator_preferred()` | Remove OR complete operator-preferences wiring |

**Rank 2-10: NONE** — all other pub surfaces across the top 20 crates are actively consumed.

## Production unwrap violations

**ZERO.** All detected unwraps fall into safe categories:
- Test code (`#[test]`, `_tests.rs`)
- Doc examples in `///` comments
- Guarded patterns (`unwrap_or_else`, `.ok()?`)

Example: `argo-marketplace/oci_auth.rs:64` uses `std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))` — safe fallback.

## Top 10 near-limit files (400-499 LOC)

| LOC | File | Recommendation |
|---|---|---|
| 488 | `argo-skill/ir/types_tests.rs` | Test file; no split |
| 473 | `argo-vertical/manifest.rs` | Core validation; keep |
| 473 | `argo-genesis/broad_role.rs` | Cohesive archetype synthesis; keep |
| 471 | `argo-skill/marketplace.rs` | Load-bearing integration; keep |
| 468 | `argo-identity/types.rs` | Type consolidation; keep |
| **461** | **`argo-store/migrations/registry.rs`** | **SPLIT** — extract per-domain migrations |
| 460 | `argo-store/approvals.rs` | Cohesive; keep |
| 458 | `argo-store/lib_tests.rs` | Test file; no split |
| 455 | `argo-store/approvals_tests.rs` | Test file; no split |
| 452 | `argo-genesis/pipeline.rs` | Cohesive; keep |

## Top 5 load-bearing TODOs

1. `argo-skill/marketplace_oci.rs:9` — OCI catalog pagination beyond 100 items TODO. P1, deferred to v1.1.
2. `argo-channels/fallback.rs:88` (orphan) — operator-preferences integration incomplete. P0 or P1.1 depending on cleanup choice.
3. `argo-executor-computer-use/executor.rs:190` — profile-pinned cost ratio hard-coded. P2.
4. `argo-runtime/oom_restart.rs:200` — OOM claim transition semantics. P2.
5. `argo-marketplace/marketplace_oci.rs:10` — multi-tag selection semantics. P2.

## Recommended briefs

| Brief | Scope | LOC | Priority |
|---|---|---|---|
| `claude-channels-fallback-orphan-cleanup` | Remove `with_operator_preferred()` OR complete preferences wiring | 50-150 | P0 (orphan cleanup) |
| `claude-store-migrations-registry-split` | Pre-emptive 461→<500 split | 200 | P1 |
| `mission-skill-oci-pagination` | Pagination for OCI catalog beyond 100 items | 300-400 | P1 (post-v1) |
| `mission-forge-dynamic-cost-registry` | Profile-pinned cost ratio → dynamic price registry | 250 | P2 (post-v1) |
| `mission-marketplace-multi-tag-semantics` | Clarify multi-tag selection in marketplace | 100 | P2 (post-v1) |

## Verdict

**Green for v1.0.** Codebase is production-ready with no invariant violations. Only one orphan to clean up + one pre-emptive split. All other findings are post-v1.1 backlog candidates.
