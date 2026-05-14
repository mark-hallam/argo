# claude-store-migrations-registry-split — Pre-emptive split for `argo-store/src/migrations/registry.rs` (461 LOC)

**Owner**: claude. **Depends on**: nothing (but pairs well with `claude-migration-slot-dedup-and-validator`).
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-store-migrations-registry-split`.

## Scope

Phase B audit found `crates/argo-store/src/migrations/registry.rs` at 461 LOC, the only near-limit file from the top-20 deep dive that justifies a refactor split (the others are either tests or load-bearing cohesive domain logic).

Pre-emptive split: extract per-domain migration registration into sibling files. The registry continues to manage runner orchestration; per-domain files own their migration lists.

Suggested split:
- `migrations/registry.rs` — coordinator + runner glue (~200 LOC)
- `migrations/registry_workers.rs` — worker/identity migrations
- `migrations/registry_skills.rs` — skill-related migrations
- `migrations/registry_workflow.rs` — workflow/orchestrator migrations
- `migrations/registry_compliance.rs` — audit/compliance/HIPAA/SOC2/GDPR migrations
- Each sibling: ≤150 LOC.

LOC: ~200 modified + redistributed.

## Files you may touch

- `crates/argo-store/src/migrations/registry.rs` — split as above. ≤200 modified lines (mostly relocating items, not changing them).
- NEW `crates/argo-store/src/migrations/registry_workers.rs`
- NEW `crates/argo-store/src/migrations/registry_skills.rs`
- NEW `crates/argo-store/src/migrations/registry_workflow.rs`
- NEW `crates/argo-store/src/migrations/registry_compliance.rs`
- `crates/argo-store/src/migrations/mod.rs` (or equivalent module file) — narrowly: add `pub mod registry_*;` declarations. ≤10 modified lines.

## Files you must NOT touch

- ANY `.sql` migration file (this is a Rust-code split only).
- The sqlx migration runner internals (consume existing API).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `registry.rs` < 300 LOC after split.
- [ ] Each new sibling file < 200 LOC.
- [ ] Migration order is preserved across the split (runner sees identical sequence).
- [ ] `cargo test -p argo-store migrations` passes.
- [ ] `cargo build --workspace` clean.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Smoke test: a fresh `argo-store` database initializes cleanly post-split (same as pre-split).
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-store-migrations-registry-split`
- PR title: `refactor: split argo-store migrations/registry.rs into domain-scoped siblings (claude-store-migrations-registry-split)`

## Notes for the implementer

- Mission-claim alignment: production hardening (file-LOC-ceiling discipline).
- Phase B audit report: `docs/skycot/audit/2026-05-14-phase-B-per-crate-deep-dive.md` §4.
- **Coordinate with `claude-migration-slot-dedup-and-validator`** — both touch the same file. Land that brief first if both are in flight.
- Pure refactor; behavior must be byte-identical post-split. Migration ordering is the key invariant.
- Drift-prevention: brief adds NEW siblings to an existing module; no parallel surfaces.
