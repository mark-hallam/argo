# mission-workplace-knowledge-audit-kinds — add 3 `AuditKind` variants for wiki doc lifecycle

**Owner**: codex or claude (purely mechanical clone — codex preferred).
**Depends on**: `mission-workplace-knowledge-substrate` MERGED (the table its variants describe).
**Authored against**: origin/main `c613d922` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-workplace-knowledge-audit-kinds`.
**Priority**: P1 — Wave A brief 2 of 6 from `docs/skycot/design/workplace-onboarding-architecture.md`.
**Sizing**: ~200 LOC across 2 files (`event.rs` + `event_tests.rs`).

Suggest fast-mode in the worker header — pure mechanical clone-and-extend with no architectural decisions.

## Why this brief exists

Read design doc §3.3 (12 new AuditKinds total — this brief lands the 3 wiki-doc ones; the other 9 are Wave-B-E briefs) and §7 Pillar 1 acceptance ("Audit row per edit — `SELECT kind, payload FROM audit_record WHERE kind = 'workplace_knowledge.doc.updated'` returns rows with diff in payload after every edit").

The wiki substrate brief (#1) lands the tables. The gateway brief (#3) lands the endpoints. Between them, the endpoints need to emit named audit rows for every doc lifecycle event so the audit feed shows `workplace_knowledge.doc.updated` rather than the generic `HttpRequest` opaque row.

This is the exact 4-site recipe used by **`web-audit-kinds-for-journey-events`** (sibling brief — different domain, identical structure) and by every `AuditKind` variant landed since #1865 introduced `ServiceTokenBindingCreated` (look there for the canonical example).

## The 3 variants (verbatim from design doc §3.3)

| Variant | Kind label |
|---------|------------|
| `WorkplaceKnowledgeDocCreated` | `workplace_knowledge.doc.created` |
| `WorkplaceKnowledgeDocUpdated` | `workplace_knowledge.doc.updated` |
| `WorkplaceKnowledgeDocDeleted` | `workplace_knowledge.doc.deleted` |

## Approach (concrete, mirrors `AuditKind::ServiceTokenBindingExpired` from #1911)

### Site 1 — enum definition

**MODIFY** `crates/argo-audit/src/event.rs` — add 3 variants to `pub enum AuditKind`. Insert alphabetically (or in whatever convention the existing enum follows; read the surrounding ~5 variants first to confirm).

```rust
WorkplaceKnowledgeDocCreated,
WorkplaceKnowledgeDocUpdated,
WorkplaceKnowledgeDocDeleted,
```

### Site 2 — `as_kind_str` match arm

**MODIFY** same file. Add 3 arms to `impl AuditKind { pub fn as_kind_str(&self) -> &'static str { match self { ... } } }`:

```rust
Self::WorkplaceKnowledgeDocCreated => "workplace_knowledge.doc.created",
Self::WorkplaceKnowledgeDocUpdated => "workplace_knowledge.doc.updated",
Self::WorkplaceKnowledgeDocDeleted => "workplace_knowledge.doc.deleted",
```

### Site 3 — `from_kind_str` match arm

**MODIFY** same file. Add 3 arms to `impl AuditKind { pub fn from_kind_str(s: &str) -> Self { match s { ... } } }`:

```rust
"workplace_knowledge.doc.created" => Self::WorkplaceKnowledgeDocCreated,
"workplace_knowledge.doc.updated" => Self::WorkplaceKnowledgeDocUpdated,
"workplace_knowledge.doc.deleted" => Self::WorkplaceKnowledgeDocDeleted,
```

### Site 4 — `ALL_KINDS` slice / round-trip iterator

**MODIFY** same file. Grep for `ALL_KINDS` or similar — there's typically a `pub const ALL_KINDS: &[AuditKind] = &[ ... ];` array used by `event_tests.rs` for exhaustive round-trip coverage:

```rust
Self::WorkplaceKnowledgeDocCreated,
Self::WorkplaceKnowledgeDocUpdated,
Self::WorkplaceKnowledgeDocDeleted,
```

### Test update

**MODIFY** `crates/argo-audit/src/event_tests.rs` — bump the variant-count assertion. Grep for the assertion pattern (e.g. `assert_eq!(ALL_KINDS.len(), N)` or similar). Bump `N → N+3`. Add round-trip coverage for the 3 new kinds if the test file does per-variant assertions.

### Optional convenience wrappers

If the existing pattern includes wrapper helpers (e.g. `pub fn emit_approval_expired(...) -> AuditRow`), add 3 wrappers:

```rust
// Likely in crates/argo-audit/src/event.rs or a sibling helpers module —
// check what the existing variants do. If no convenience helpers exist
// for ServiceTokenBindingExpired, skip this step.
pub fn workplace_knowledge_doc_created_payload(slug: &str, version: i64, created_by: &str) -> serde_json::Value {
    serde_json::json!({ "slug": slug, "version": version, "created_by": created_by })
}
// Plus *_updated_payload(...) and *_deleted_payload(...).
```

These are **opt-in** — only add them if the established pattern has them for adjacent variants.

## Files you may touch (verified to exist on `c613d922`)

- **MODIFY** `crates/argo-audit/src/event.rs` — 4-site recipe above. ~15 LOC delta total.
- **MODIFY** `crates/argo-audit/src/event_tests.rs` — variant-count bump + optional round-trip extension. ~10 LOC delta.

That's it. Two files. Total delta ~25 LOC of changes (the brief's ~200 LOC budget reflects authoring overhead + clippy reruns; net code change is small).

## Files you must NOT touch

- Anything outside `crates/argo-audit/src/`.
- `crates/argo-audit/src/sink_chain.rs` — chain-emit logic is correct.
- `crates/argo-audit/src/verify.rs` — chain-verify logic is correct.
- `crates/argo-cli/src/handlers/gateway/*` — the gateway emits the variants but that wiring is **brief 3**'s job, not this one. Stay scoped.
- `crates/argo-workplace-knowledge/*` — the substrate is **brief 1**'s domain. This brief doesn't read/write docs, it just adds the audit kinds.

## Pre-flight verification

```bash
# Confirm the canonical reference site (recent equivalent brief).
rg 'ServiceTokenBindingExpired' crates/argo-audit/src/event.rs | head -5
# Expect 4 matches (the 4-site recipe).

# Confirm ALL_KINDS slice exists.
rg 'pub const ALL_KINDS|fn from_kind_str|fn as_kind_str' crates/argo-audit/src/event.rs

# Confirm current variant count from the test file.
grep -E 'ALL_KINDS.len\(\)|len\(\), [0-9]+' crates/argo-audit/src/event_tests.rs | head -3

# Confirm the substrate brief landed (this brief depends on it).
ls crates/argo-workplace-knowledge/src/lib.rs
# If this returns "no such file", DO NOT CLAIM — wait for brief 1 to merge.
```

## Acceptance criteria

- [ ] All 3 new variants exist on the enum.
- [ ] `AuditKind::WorkplaceKnowledgeDocCreated.as_kind_str() == "workplace_knowledge.doc.created"` (and same for Updated / Deleted).
- [ ] `AuditKind::from_kind_str("workplace_knowledge.doc.created") == AuditKind::WorkplaceKnowledgeDocCreated` (round-trip).
- [ ] All 3 variants appear in `ALL_KINDS`.
- [ ] Variant-count assertion in `event_tests.rs` bumped by 3.
- [ ] `cargo test -p argo-audit --lib` green.
- [ ] `cargo clippy -p argo-audit --all-targets -- -D warnings` clean.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] Every modified file < 500 LOC.
- [ ] No `unwrap` / `expect` outside `#[cfg(test)]`.
- [ ] BUILD_STATUS.md row flipped to IN_REVIEW with PR number on push.

## Branch + PR

- Branch: `codex-mission-workplace-knowledge-audit-kinds`
- PR title: `argo-audit: add 3 WorkplaceKnowledgeDoc* AuditKind variants (mission-workplace-knowledge-audit-kinds)`

## Gotchas

- **Hash-chain stability** (design doc §7 Pillar 5 acceptance): adding new AuditKind variants MUST NOT break `argo-audit::verify::verify_chain`. Pre-existing chain tests must continue to pass. The hash includes the kind label string, so as long as the strings are STABLE (don't rename a variant after merge), the chain is preserved. **Do not rename or remove any of the 3 strings once this brief lands.**
- **Alphabetical insertion**: read the surrounding variants in `event.rs` and insert in whatever order the rest of the enum uses. `Workplace*` should land near the end alphabetically — verify by `grep -nE 'pub enum AuditKind' crates/argo-audit/src/event.rs` then scrolling.
- **Don't emit the variants here**: emission is brief 3's responsibility. This brief makes the variants EXIST; the gateway brief makes them GET EMITTED.

## References

- Design doc: `docs/skycot/design/workplace-onboarding-architecture.md` §3.3 (full 12-variant table; this brief lands 3 of 12), §7 Pillar 1.
- Sibling reference PR: `#1911` (`codex-service-token-binding-stale-sweeper`) added `ServiceTokenBindingExpired` using the exact same 4-site recipe — read the merged diff for the literal pattern.
- Sibling brief: `mission-onboarding-audit-variants` (currently IN_REVIEW from tick 13) adds `ProviderKeyConfigured` + `VerticalInstalled` using the same recipe — could be a parallel reference, but use the merged #1911 pattern as the source of truth.
- Design doc §7 Pillar 5 — hash-chain integrity acceptance.

## Why this is a codex-sized brief

- Single architectural pattern (proven 4-site AuditKind recipe).
- 2 files touched, both narrow.
- No new pub types beyond 3 trivial enum variants, no new crate, no migration.
- Worker can complete in 30-60 min including the test bump.
