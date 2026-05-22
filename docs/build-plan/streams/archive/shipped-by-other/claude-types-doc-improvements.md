# claude-types-doc-improvements — usage examples + doctests on top-20 argo-types

**Owner**: claude. **Depends on**: nothing. **Phase B doc-quality narrow.**

## Why

`argo-types` is the foundation crate — every other workspace crate imports from it. The public types (TenantId, SessionId, WorkerId, ProviderModel, RoutingDecision, ActionDeclaration, ActionOutcome, ExecutorKind, etc.) carry **thin one-line doc comments** that describe what the type is but not how to construct, compare, or serialize it. New contributors and skill-consumers (the Claude SDK, gateway clients, marketplace tooling) repeatedly ask the same questions: "what's the canonical TenantId format", "is SessionId a UUID", "how do I round-trip ProviderModel through serde". Each answer lives only in tribal memory or by reading the impl.

Doctests double as compile-checked examples. Thin docs + zero examples means every new caller learns by failed attempt.

## Approach (concrete)

**Pure documentation.** Zero behaviour change. Every type's API surface stays identical. Add ≥ 1 doctest per top-20 type, with the doctest demonstrating the most common construction + a round-trip through serde where applicable.

Top-20 types to cover (verify exact names against `crates/argo-types/src/lib.rs`):

1. `TenantId` — construction, `as_str`, parse, serde round-trip
2. `SessionId` — construction (UUID-backed), comparison, serde
3. `WorkerId` — construction, parse, serde
4. `ProviderModel` — construction from `(provider, model)`, display, parse round-trip
5. `RoutingDecision` — construction, the variants, when each applies
6. `ActionDeclaration` — minimal viable construction + serialize
7. `ActionOutcome` — `Completed` vs `Pending` vs error path (note: no `Failed` variant — use `Result::Err`)
8. `ExecutorKind` — every variant + parse from string
9. `SkillId` — construction, namespacing rules
10. `BudgetCents` — construction, arithmetic, no-negative invariant
11. `CostCents` — same as BudgetCents but separate type
12. `Timestamp` (or `Instant` wrapper) — UTC discipline, serde
13. `MessageRole` — `User` / `Assistant` / `System` / `Tool` round-trip
14. `Message` — minimal text message
15. `ToolCall` — JSON-arg shape
16. `ToolResult` — success + error variants
17. `Vertical` (or `VerticalManifest`) — minimal manifest
18. `RoutingPolicy` — the variants
19. `ChannelId` (push channel) — construction
20. `EventId` (or `AuditId`) — construction, monotonicity

For each: ≤ 10-line doctest. Bias toward demonstrating the canonical happy path + the non-obvious thing (e.g. that `TenantId` strings are case-sensitive, that `SessionId::new()` allocates a fresh UUID, that `ProviderModel` parses `provider:model` syntax).

## Files you may touch

- `crates/argo-types/src/lib.rs` — narrowly: add doc-comments + doctests above each of the top-20 types
- `crates/argo-types/src/*.rs` — if any of the top-20 live in submodules, narrowly extend their docs
- NO new files unless an existing module exceeds 500 LOC after additions (in which case extract to a sibling)

## Files you must NOT touch

- Any non-types crate
- Any test directory other than the inline `///` doctests
- The Cargo.toml (no new deps)

## Acceptance criteria

- [ ] Top-20 types each have a triple-slash usage example block
- [ ] `cargo test -p argo-types --doc` runs every new doctest green
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] No public API change (rustdoc diff is purely additive)
- [ ] Every modified file remains ≤ 500 LOC (split if needed)
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-types-doc-improvements`
- PR title: `types: usage examples + doctests on top-20 argo-types (claude-types-doc-improvements)`

## Notes for the implementer

- Open `crates/argo-types/src/lib.rs` and grep for `pub struct` / `pub enum`. The top-20 are the most-imported across the workspace; if uncertain, run `rg --type rust 'use argo_types::' | sort | uniq -c | sort -rn | head -30`.
- Doctests must compile — use `# use argo_types::*;` hidden setup lines liberally to keep examples short.
- For UUID-backed types, prefer `SessionId::new()` over hard-coded UUIDs in doctests so the output isn't asserted against a specific value.
- For serde round-trips, use `serde_json::to_string` + `serde_json::from_str` (serde_json is already a dev-dep via the workspace).
- This brief is **scoped narrowly to docs**. If you find a missing constructor or an inconsistency, file a follow-up brief — DO NOT widen scope.
