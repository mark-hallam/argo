# Mission load-bearing wire — SkillStore::available_for_worker lookup (M6 precursor 2/3)

**Owner**: claude. **Depends on**: `mission-skill-min-trust-grade-column` MERGED (precursor 1/3 — adds the column this method filters on); `Worker.trust_grade` SHIPPED in #1273; `SkillStore` SHIPPED.
**Authored against**: origin/main `5e1ef545` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-store-available-for-worker`.

> SOURCE: precursor authored 2026-05-11 by conductor to unblock `mission-skill-propagation-loop` (Tier M6). The M6 brief was BLOCKED because it assumed a `SkillStore::available_for_worker` lookup that doesn't exist. This brief adds it; the next brief (mission-skill-session-boot-wire) wires it into agent boot; the rewritten M6 demonstrates propagation end-to-end.

## Scope

Add `pub async fn available_for_worker(&self, worker_id: &WorkerId) -> ArgoResult<Vec<Skill>>` to `SkillStore`:

1. Reads the worker's `trust_grade` from `IdentityStore`.
2. Queries skills where `(min_trust_grade IS NULL) OR (worker_trust_grade >= min_trust_grade)`.
3. Returns the filtered Vec<Skill>.

```rust
impl SkillStore {
    pub async fn available_for_worker(
        &self,
        worker_id: &WorkerId,
    ) -> ArgoResult<Vec<Skill>>;
}
```

**Wait, scope concern**: this fn needs to read both `skill` AND `worker` rows, but `argo-skill` doesn't (and shouldn't) depend on `argo-identity` directly. Solution: take an `IdentityStore` reference as a parameter:

```rust
impl SkillStore {
    pub async fn available_for_worker(
        &self,
        identity: &IdentityStore,
        worker_id: &WorkerId,
    ) -> ArgoResult<Vec<Skill>>;
}
```

OR: define a narrow `WorkerTrustReader` trait in `argo-skill` with `fn trust_grade(&self, id: &WorkerId) -> ArgoResult<Option<TrustGrade>>` and impl it on `IdentityStore` in `argo-cli`. Implementer chooses based on existing crate-boundary patterns. **Document the choice in the PR body.**

If worker isn't found, return error. If worker has no trust_grade column populated (legacy), treat as the lowest grade (most restrictive).

This is NOT in scope: cross-tenant filtering (per-tenant scope only); workspace_id filtering (orthogonal); skill version selection (latest only).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-skill/src/store.rs` — narrowly scoped: ADD `available_for_worker` method. Verify file currently <450 LOC; if at-risk-of-ceiling, place in NEW sibling `store_available.rs`.
- `crates/argo-skill/src/store_available_tests.rs` (NEW, sibling) — 6 unit tests: no-restriction-skills-returned; restricted-skill-filtered-when-worker-grade-too-low; restricted-skill-returned-when-worker-grade-equal; restricted-skill-returned-when-worker-grade-higher; legacy-null-worker-grade-treated-as-lowest; nonexistent-worker-errors.
- (IF trait approach chosen) `crates/argo-skill/src/trust_reader.rs` (NEW, ≤ 50 LOC) — `WorkerTrustReader` trait.
- (IF trait approach chosen) `crates/argo-cli/src/handlers/...` (one wiring file) — impl `WorkerTrustReader for IdentityStore`.

## Files you must NOT touch

- `crates/argo-identity/**` — identity store stable; reuse only (don't add dependencies on argo-skill or vice versa beyond the trait)
- `crates/argo-skill/src/trust.rs` — TrustGrade type stable
- `crates/argo-store/migrations/**` — no schema change (column added by precursor 1/3)
- Any non-skill / non-identity-wiring crate

## Acceptance criteria

- [ ] `available_for_worker(worker_id)` exists on `SkillStore`, returns `Vec<Skill>`
- [ ] Skills with `min_trust_grade IS NULL` are always included
- [ ] Skills with `min_trust_grade` set are included only when worker's trust_grade ≥ that value
- [ ] Worker not found → explicit error
- [ ] Legacy worker (no trust_grade) treated as most restrictive (lowest grade)
- [ ] Crate-boundary choice documented in PR body (trait vs. direct ref param)
- [ ] 6 unit tests
- [ ] No new public API outside `available_for_worker` + (optionally) `WorkerTrustReader`
- [ ] `cargo build -p argo-skill -p argo-identity -p argo-cli` clean
- [ ] `cargo clippy -p argo-skill -p argo-identity -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-skill -p argo-identity -p argo-cli` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-store-available-for-worker`
- PR title: `skill: SkillStore::available_for_worker trust-gated lookup (mission-skill-store-available-for-worker)`

## Notes for the implementer

- The crate-boundary question is real: argo-skill currently does NOT depend on argo-identity. Adding a dep would be a one-way arrow; argo-identity already depends on argo-store. The narrow `WorkerTrustReader` trait is the cleaner answer. Verify in first 5min: check `crates/argo-skill/Cargo.toml` for argo-identity; if absent, use the trait approach.
- The "legacy worker without trust_grade" case will be common until the trust-grade rollout is widespread. Don't error — treat as lowest grade and let the result be a smaller list.
- Performance: this is called on session boot (precursor 3/3). Hit count is low per session. No caching needed for v1.
- Precursor 3/3 (mission-skill-session-boot-wire) will use this method. Don't try to do the wiring here.
