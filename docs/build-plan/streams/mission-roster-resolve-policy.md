# Mission load-bearing wire — roster lookup before resolve_effective_policy

**Owner**: claude. **Depends on**: `argo-identity::policy::resolve_effective_policy` exists (it does — `crates/argo-identity/src/policy.rs`); roster store exists.
**Authored against**: origin/main `0742cab0` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-roster-resolve-policy`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 2 item #53 ("Wire roster lookup before `resolve_effective_policy`"). Today the auto-approval gate calls `resolve_effective_policy(worker, action)` directly, which reads the worker's manifest-level policy. The roster — the per-tenant mapping of worker → assigned role/team/policy override — is consulted nowhere. Result: roster-level overrides (e.g. "all workers in the after-hours roster auto-approve up to $50") are silently inert. Mission Claim 2 ("delegation as a unit") implies the roster is a real authority surface; the wire closes the gap.

## Scope

`worker_actions_auto_approval.rs` (line ~250 region — the entry point that resolves the effective approval policy for an action attempt) currently does:

```rust
let policy = resolve_effective_policy(worker_id, &action_name, ...).await?;
```

This brief inserts a roster-lookup step before the policy resolution: read the worker's roster assignment via `RosterStore::lookup_for_worker(worker_id)`. If a roster is found AND the roster carries a `[policy_override]` block, merge that override into the policy resolution (override-on-top, manifest-as-floor). If no roster is found, behavior is unchanged.

The merge is additive: roster overrides can RAISE auto-approval thresholds (more permissive) or LOWER them (more restrictive), but cannot remove channels or risk classes from the manifest's declared set. This invariant prevents a roster from accidentally widening the audit surface. Implementations: extend `EffectivePolicy` (in `argo-identity::policy`) with a `pub(crate) fn merge_roster_override(self, override: RosterPolicyOverride) -> Self` method.

This is NOT in scope: defining a UI for editing roster overrides; supporting per-action overrides (only per-worker / per-tenant); modifying the auto-approval decision tree itself; building a roster CLI command.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` — insert roster lookup before `resolve_effective_policy` call; pass the override (if any) to the merge step.
- `crates/argo-identity/src/policy.rs` — narrowly scoped: add `pub fn merge_roster_override(self, override_: RosterPolicyOverride) -> Self` method on `EffectivePolicy`. NO other edits to existing functions/structs in this file.
- `crates/argo-identity/src/policy_tests.rs` — narrowly scoped: append 4 new tests beneath the existing test module: roster-raises-threshold; roster-lowers-threshold; roster-cannot-add-new-channel; roster-cannot-add-new-risk-class.
- `crates/argo-identity/src/lib.rs` — narrowly scoped: ONE re-export line if `RosterPolicyOverride` is added as a NEW public type. No other edits.
- `crates/argo-identity/src/roster_policy_override.rs` (NEW) — the `RosterPolicyOverride` struct (fields: `auto_approve_max_amount: Option<u64>`, `additional_quiet_hours: Option<QuietHours>`, etc.) with `Deserialize` + `Default`. Pure data type; no logic.
- `crates/argo-cli/tests/roster_policy_override_smoke.rs` (NEW) — integration smoke: seed a worker + roster with override, attempt action, assert the override applies.

## Files you must NOT touch

- `crates/argo-store/migrations/**` — the brief assumes `RosterStore::lookup_for_worker` already returns the override field. If it does NOT (verify in implementer's first 10 minutes), STOP and mark BLOCKED with reason `roster store missing override field; needs upstream brief`. Do not silently add a migration.
- `crates/argo-channels/**` — channel selection is decided downstream; this brief does not touch channel routing.
- Any worker-action handler beyond `worker_actions_auto_approval.rs`.

## Acceptance criteria

- [ ] `RosterPolicyOverride` struct exists in `argo-identity` and is publicly exported
- [ ] `EffectivePolicy::merge_roster_override` exists and merges fields per spec (override CAN raise/lower thresholds; CANNOT add new channels or risk classes)
- [ ] `worker_actions_auto_approval.rs` calls `RosterStore::lookup_for_worker(worker_id)` before `resolve_effective_policy` and threads the override into the merge call
- [ ] When roster lookup returns None, behavior is byte-identical to today (no roster, no override, original policy)
- [ ] When roster override conflicts with manifest invariant (tries to add a channel manifest didn't declare), the merge silently drops that field — no panic, no error returned to caller
- [ ] 4 unit tests in `policy_tests.rs` cover: raise-threshold; lower-threshold; cannot-add-new-channel; cannot-add-new-risk-class
- [ ] 1 integration smoke in `tests/roster_policy_override_smoke.rs` exercises end-to-end: seed worker + roster with override; trigger auto-approval; assert override applies
- [ ] If roster store does NOT carry the override field today, the brief is marked BLOCKED with the specific store-extension brief needed (do not invent the schema in this brief)
- [ ] `cargo build -p argo-cli -p argo-identity` clean
- [ ] `cargo clippy -p argo-identity --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-identity` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-roster-resolve-policy`
- PR title: `identity+cli: roster lookup merges policy override before resolve_effective_policy (mission-roster-resolve-policy)`

## Notes for the implementer

- First 10 minutes: confirm `RosterStore::lookup_for_worker` exists and returns a row carrying override-shaped fields. `rg -n 'lookup_for_worker|RosterStore' crates/argo-identity crates/argo-store`. If absent, STOP and BLOCK; do not invent the schema here.
- The merge invariant (override cannot add new channels or risk classes) is enforced inside `merge_roster_override` itself. Easiest implementation: intersect the override's declared sets with the manifest's, and use `max(manifest.threshold, override.threshold)` style merges per field. Document the invariant in a doc-comment on the function.
- `policy_tests.rs` already has fixture builders for `EffectivePolicy`; reuse them. Don't hand-roll new fixtures.
- The `additional_quiet_hours` override field, if implemented, should UNION with the manifest's quiet hours (more restrictive wins). Spec: a quiet hour declared in either manifest OR override is quiet. Test that explicitly.
- The roster CLI / UI is out of scope — there's no need to add a `roster set-override` command in this brief. Operators today edit the roster store row directly; that's adequate for the wire-up.
