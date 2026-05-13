# claude-roster-store-policy-override-column — precursor: add policy_override column + lookup_for_worker helper

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `6676b391` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-roster-store-policy-override-column`.

> **PRECURSOR 2026-05-13 by conductor (CEO-authorized Cluster D #2).** Unblocks `mission-roster-resolve-policy` v2. The roster store today has `rosters_for_worker(tenant, worker_id) -> Vec<Roster>` but no per-roster `policy_override` column and no `lookup_for_worker` single-row helper. The downstream brief needs both.

## Scope

1. **Migration**: NEW slot (≥ next free) adds nullable `policy_override_json TEXT` column to existing `roster` table. Nullable so existing rows stay valid; new field carries serialized `RosterPolicyOverride` JSON when set.

2. **Single-row helper**: NEW `RosterStore::lookup_for_worker(worker_id: WorkerId) -> StoreResult<Option<RosterAssignment>>` — returns the single most-relevant roster (priority order: explicit tenant assignment → default tenant roster → None). Different shape from `rosters_for_worker` which returns `Vec<Roster>` (workers can belong to N rosters); the downstream `resolve_effective_policy` flow needs the "which roster's override applies right now" decision.

3. **Override type**: NEW `RosterPolicyOverride` struct in `crates/argo-identity/src/roster_policy_override.rs` — fields per the v1 brief (auto_approve_max_amount: Option<u64>, additional_quiet_hours: Option<QuietHours>, etc). Pure data type; `Deserialize` + `Default`.

**Not in scope**: actually merging the override into the policy (downstream brief); UI for editing overrides; per-action override semantics.

## Files you may touch

- `<NEW: crates/argo-store/migrations/<slot>__roster_policy_override.sql>` (≤ 30 LOC) — `ALTER TABLE roster ADD COLUMN policy_override_json TEXT`.
- `crates/argo-store/src/migrations/registry.rs` — narrowly scoped: register the new slot.
- `crates/argo-store/src/roster.rs` (or wherever `RosterStore` lives — `rg -l 'impl RosterStore' crates/argo-store/`) — narrowly scoped: add `lookup_for_worker` helper. Expose the new `policy_override_json` field on the row struct.
- `<NEW: crates/argo-identity/src/roster_policy_override.rs>` (≤ 80 LOC) — pure data type.
- `crates/argo-identity/src/lib.rs` — narrowly scoped: re-export `RosterPolicyOverride`.

## Files you must NOT touch

- The existing `rosters_for_worker(...)` API — locked.
- `argo-identity::policy::resolve_effective_policy` — downstream brief handles the merge.
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` — downstream brief wires the call.

## Acceptance criteria

- [ ] Migration is additive only (ALTER TABLE ADD COLUMN; no drops/renames).
- [ ] `lookup_for_worker` returns `Some(RosterAssignment)` for workers with a roster + `None` otherwise.
- [ ] Round-trip: write a roster row with `policy_override_json = Some(...)`, read back via `lookup_for_worker`, deserialize → matches the original `RosterPolicyOverride`.
- [ ] `RosterPolicyOverride::default()` is the no-op override (all `None`).
- [ ] 5 unit tests covering: no-roster lookup; roster-without-override; roster-with-override round-trip; multi-roster priority ordering; default vs explicit override.
- [ ] `cargo clippy -p argo-store -p argo-identity --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-roster-store-policy-override-column`
- PR title: `store: roster policy_override column + lookup_for_worker helper (claude-roster-store-policy-override-column)`

## Notes for the implementer

- Single-row priority ordering: if a worker has multiple roster assignments, pick the one with the most specific scope (explicit-tenant > default-tenant). Document the ordering in a code comment.
- Migration slot: verify next free at claim time. Slot ≥ what `#1450`'s v61 took.
- Once this lands, `mission-roster-resolve-policy` is reauthored as `mission-roster-resolve-policy-v2` targeting argo-identity + argo-cli only.
