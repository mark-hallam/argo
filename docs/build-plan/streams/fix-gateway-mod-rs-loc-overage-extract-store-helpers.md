# fix-gateway-mod-rs-loc-overage-extract-store-helpers

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `8c65a63d4` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-mod-rs-loc-overage-extract-store-helpers`.

🔴 **P0 BLOCKS DEMO-MP4 P0 PR #2407.** `crates/argo-cli/src/handlers/gateway/mod.rs` is at **503 LOC** on `origin/main` — 3 LOC over the 500-LOC hard ceiling enforced by gate 3 of `conductor-pre-merge-gate.sh`. PR #2407 (`fix: seed demo marketplace skill catalog on gateway boot` — the 4th of 4 demo-MP4 P0s, evaluator-pre-verified round 87) needs to add a single `mod demo_seed_skills;` declaration, which tips the file to 504. The PR cannot merge until `mod.rs` is back under 500.

The fast extract path: move the 4 `pub(super)` async store-helper fns at the bottom of `mod.rs` (lines ~400–500: `load_tenant_rate_limit_overrides`, `session_tool_call_count`, `session_spend_micros`, `spend_per_session`) plus the two tiny helpers `store_unavailable` + `body_from_request` into a new sibling file `crates/argo-cli/src/handlers/gateway/store_helpers.rs`. Add `mod store_helpers;` + `pub(super) use store_helpers::{...};` re-exports in `mod.rs`. Net delta: `mod.rs` shrinks by ~120 LOC; new file `store_helpers.rs` is ~120 LOC; both well under 500.

## Scope

- Create `crates/argo-cli/src/handlers/gateway/store_helpers.rs` containing the 6 extracted fns (4 async store fns + 2 tiny non-async helpers).
- Edit `crates/argo-cli/src/handlers/gateway/mod.rs`:
  - Remove the 6 fn definitions.
  - Add `mod store_helpers;` to the module-declaration block (alphabetical position).
  - Add `pub(super) use store_helpers::{body_from_request, load_tenant_rate_limit_overrides, session_spend_micros, session_tool_call_count, spend_per_session, store_unavailable};`
- Keep all imports each fn needs (e.g. `argo_store`, `argo_types`, `sqlx`) inside `store_helpers.rs` — do not leak them at the parent module scope.
- The `#[cfg(test)] mod bind_flag_tests { ... }` block at the bottom of `mod.rs` STAYS in `mod.rs` — it tests `resolve_non_loopback_override` which is defined in `mod.rs` itself, not extracted.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/mod.rs`
- `crates/argo-cli/src/handlers/gateway/store_helpers.rs` (new)
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Any callers of the extracted fns — `pub(super) use` re-exports preserve the call sites verbatim (they reference `crate::handlers::gateway::session_tool_call_count` etc., which still resolve).
- The `#[cfg(test)] mod bind_flag_tests` block in `mod.rs`.
- Any other crate — this is a strictly local refactor.

## Acceptance criteria

- [ ] `crates/argo-cli/src/handlers/gateway/mod.rs` is **≤ 500 LOC** after the extract.
- [ ] `crates/argo-cli/src/handlers/gateway/store_helpers.rs` is **≤ 500 LOC**.
- [ ] `cargo check -p argo-cli` passes.
- [ ] `cargo test -p argo-cli --lib gateway::` passes (the bind-flag tests still find their target).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` exits 0 against the PR branch.
- [ ] BUILD_STATUS.md row flipped to MERGED with PR number; SHARED.md handoff line written so PR #2407 can be unblocked next conductor tick.

## Why this is mechanical, not architectural

The 6 fns are leaf helpers — they don't call each other, they don't take parent-module state, and every call site already references them through their canonical paths. Moving them is rename-and-recompile, not redesign. Operator-locked CLAUDE.md "anti-parallel-surface" rule does NOT apply (no new pub type, no widening visibility — `pub(super)` is preserved as-is). Operator-locked "wire or defer" rule does NOT apply (no new public surface added).

## Why P0

This is the only thing blocking PR #2407, which is the **4th of 4 demo-MP4 P0s** the evaluator (round 87, `8e47bfaba`) cleared as anti-theater verified. After this brief lands + #2407 rebases-and-merges, the **agent-doable remaining work for the v1.0 demo MP4 is complete** and the demo recording is unblocked end-to-end (apart from operator-only macOS perms grant + recording itself).
