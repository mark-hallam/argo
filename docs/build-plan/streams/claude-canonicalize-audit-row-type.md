# claude-canonicalize-audit-row-type — Make argo-store::AuditRow canonical; eliminate 3 duplicates

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-canonicalize-audit-row-type`.

## Scope

Phase A audit found `AuditRow` defined in **4 crates** with potentially divergent fields. Data-integrity risk: if a writer crate's `AuditRow` and a reader crate's `AuditRow` differ silently, audit data drops fields or mis-maps them.

**Current definitions** (verified by grep at HEAD):
- `crates/argo-store/src/models.rs:136` — `pub struct AuditRow` (← **canonical**; argo-store owns audit persistence)
- `crates/argo-cli/src/handlers/worker_show.rs:44` — `pub struct AuditRow` (duplicate)
- `crates/argo-executor-computer-use/src/audit.rs:17` — `pub struct AuditRow` (duplicate)
- `crates/argo-skill/src/audit_reader.rs:29` — `pub struct AuditRow` (duplicate)

**Pick canonical = argo-store** because: it owns the SQLite schema for audit rows; all readers ultimately deserialize from its table; field set is authoritative.

**Goal**: one canonical `AuditRow` in `argo-store::models`. Other 3 crates either:
(a) import `argo_store::AuditRow` directly, OR
(b) define a narrower projection (e.g., `WorkerShowAuditRow`) with a clear suffix indicating it's a view, not a copy.

LOC: ~500 (mostly imports + field-equivalence verification + tests).

## Files you may touch

**Canonical (no change to definition; verify field set is the superset):**
- `crates/argo-store/src/models.rs` — confirm `AuditRow` field set covers all 4 use cases. If missing fields exist in any duplicate, add them here. ≤30 modified lines.

**Re-route to canonical:**
- `crates/argo-cli/src/handlers/worker_show.rs` — replace local `AuditRow` with `use argo_store::AuditRow`. If the local version was a projection (subset), rename to `WorkerShowAuditView` and make it `From<argo_store::AuditRow>`. ≤50 modified lines.
- `crates/argo-executor-computer-use/src/audit.rs` — same pattern. The local version likely captured CU-specific fields (screenshot path, action_id) — if so, RENAME to `CuAuditEvent` (clearly distinct concept, not a duplicate). ≤50 modified lines.
- `crates/argo-skill/src/audit_reader.rs` — same pattern. Likely a reader projection — rename to `SkillAuditView` or replace with `argo_store::AuditRow`. ≤50 modified lines.

**Cargo.toml deps:**
- `crates/argo-cli/Cargo.toml` — confirm `argo-store` is already a dep (it is). No change expected.
- `crates/argo-executor-computer-use/Cargo.toml` — add `argo-store = { workspace = true }` if not present. ≤3 modified lines.
- `crates/argo-skill/Cargo.toml` — same. ≤3 modified lines.

**Tests:**
- Any test that constructs an `AuditRow` literal — update field set if argo-store version is the superset.

## Files you must NOT touch

- argo-store's audit-table schema migrations — additive only if a field needs adding (no destructive changes).
- The audit-chain hashing logic in `argo-audit` — separate crate, separate concern.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `rg 'pub struct AuditRow' crates/` returns exactly ONE match (argo-store).
- [ ] No silent field drops: argo-store::AuditRow's field set is the superset of all 4 original definitions, OR the renamed projections are clearly narrower views with `From` conversions.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] No new pub structs introduced outside argo-store (drift-prevention rule).
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-canonicalize-audit-row-type`
- PR title: `refactor: canonicalize AuditRow to argo-store; remove 3 duplicates (claude-canonicalize-audit-row-type)`

## Notes for the implementer

- Mission-claim alignment: production hardening (data integrity for the audit chain; supports Claim 5 evidence chain + Claim 1 teammate visibility).
- **Verify field equivalence first.** Read all 4 current `AuditRow` definitions; document the field set delta in the PR description before refactoring. If a duplicate has a field argo-store lacks, the choice is: add to canonical OR rename projection.
- **Be especially careful with CU's AuditRow** — it likely carries CU-specific fields (screenshot path, action_id, mouse_x/y). These are CU-specific events, not generic audit rows; the right answer is probably RENAME to `CuAuditEvent` rather than collapse into argo-store::AuditRow.
- Drift-prevention rule applied: this brief removes parallel surfaces, doesn't add any.
