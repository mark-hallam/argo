# Mission load-bearing wire — per-worker persona evolution (audit-logged + revertible)

**Owner**: claude. **Depends on**: `Persona` struct SHIPPED (verified at `crates/argo-identity/src/types_role.rs:31`); `argo-audit` chain SHIPPED.
**Authored against**: origin/main `06e67801` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-persona-evolution-audit-revert`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 1 item #46 ("Add per-worker persona evolution (audit-logged + revertible)"). Today `Persona` is mutated in-place via setters/CLI (`claude-persona-edit-cli` SHIPPED); each write overwrites the previous value with no history and no revert path. Mission Claim 1 ("real teammate") needs persona to evolve over time WITH operator-visible diff history and one-step revert.

## Scope

Add a `persona_revision` table + thin store API:

```rust
pub struct PersonaRevision { rev_id: Uuid, worker_id: WorkerId, revised_at: DateTime<Utc>,
    field: String,        // e.g. "voice_profile", "values", "communication_style"
    old_value: String,    // serialized old field value (JSON)
    new_value: String,
    actor: String,        // CLI user / system / forge
    audit_chain_hash: String, }
```

- `record_persona_change(worker_id, field, old, new, actor)` — append-only; computes audit-chain hash linking to previous revision for tamper-evidence.
- `list_persona_revisions(worker_id, limit)` — newest-first; default limit 50.
- `revert_persona_to(rev_id)` — applies `old_value` of the cited revision as the current persona; recursively records this as a NEW revision (so revert itself is in history).

CLI: `argo persona history <worker_id>` lists revisions; `argo persona revert <rev_id>` performs revert with confirmation prompt.

This is NOT in scope: cross-worker persona diffs; auto-revert on regression; persona forks/branches; UI page (gateway dashboard work is separate).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__persona_revision.sql` (NEW) — `CREATE TABLE persona_revision (rev_id BLOB PRIMARY KEY, worker_id BLOB NOT NULL, revised_at INTEGER NOT NULL, field TEXT NOT NULL, old_value TEXT NOT NULL, new_value TEXT NOT NULL, actor TEXT NOT NULL, audit_chain_hash TEXT NOT NULL)` + matching versioned migration in `migrations/src/`. **Implementer MUST verify next free slot via `grep -E "v4[8-9]|v5[0-9]" docs/build-plan/BUILD_STATUS.md` to avoid collision with in-flight briefs** (mission-escalation-on-no-reply uses v50/51).
- `crates/argo-store/src/persona_revision.rs` (NEW, ≤ 300 LOC) — `record_persona_change`, `list_persona_revisions`, `revert_persona_to`. Audit-chain hash computed via existing `argo-audit::chain_hash` utility.
- `crates/argo-store/src/persona_revision_tests.rs` (NEW, sibling) — 8 unit tests: record-appends-row; list-returns-newest-first; revert-applies-old-value; revert-records-itself; audit-chain-prev-link; tamper-detected; cross-worker-isolation; concurrent-writes-no-collision.
- `crates/argo-cli/src/handlers/persona.rs` (or wherever persona CLI lives — verify in first 5min) — narrowly scoped: ADD `history` and `revert` subcommands; reuse existing `argo persona edit` handler shape.
- `crates/argo-cli/tests/persona_history_smoke.rs` (NEW) — integration smoke: edit persona twice, list shows 2 revisions, revert to first, current persona matches first.

## Files you must NOT touch

- `crates/argo-identity/src/types_role.rs` — `Persona` struct itself is upstream; reuse only
- `crates/argo-audit/**` — audit-chain hash utility is upstream; reuse only
- Any tool, any non-store/non-cli crate

## Acceptance criteria

- [ ] `persona_revision` table exists with all required columns
- [ ] `record_persona_change` appends row + computes audit-chain hash linked to previous revision for that worker
- [ ] `list_persona_revisions(worker_id, limit)` returns newest-first
- [ ] `revert_persona_to(rev_id)` applies `old_value` and records the revert as a NEW revision
- [ ] Audit-chain tamper detection works (modifying a row hash invalidates subsequent links)
- [ ] `argo persona history <worker_id>` CLI works, JSON output for list/show
- [ ] `argo persona revert <rev_id>` CLI works with confirmation prompt
- [ ] 8 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-cli` clean
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-persona-evolution-audit-revert`
- PR title: `identity: persona_revision table + history/revert CLI (mission-persona-evolution-audit-revert)`

## Notes for the implementer

- The audit-chain link is load-bearing: each new revision's `audit_chain_hash = sha256(prev_hash || rev_id || field || new_value)`. If `prev_hash` row is missing for a worker, seed with the empty-string hash. This mirrors `argo-audit::chain_hash` exactly — reuse the utility.
- Revert is a NEW write, not a delete. The revision history is forever-append-only.
- The CLI `revert` should print a diff (old → current → target) before the confirmation prompt — operators are about to lose recent work and need to see what's changing.
- Don't track every Persona field individually; the `field` column is just a string label. If an operator edits 3 fields in one CLI invocation, that's 3 revisions (one per field) — simpler than batch-revert semantics.
