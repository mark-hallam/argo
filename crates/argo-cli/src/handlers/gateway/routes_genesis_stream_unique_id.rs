//! Per-generation unique `vertical_id` stamping for `routes_genesis_stream`
//! (Genesis non-determinism robustness).
//!
//! Extracted from `routes_genesis_stream.rs` to keep that file under the
//! per-file LOC ceiling. See the module-level "Non-determinism robustness"
//! section there for the full rationale.

use argo_genesis::GenesisPlan;

/// Stamp a unique, loader-legal suffix onto this generation's `vertical_id` so
/// a regeneration for the same role writes a NEW
/// `.genesis/<tenant>/<vertical_id>/` dir instead of clobbering a manifest the
/// operator may already be reviewing.
///
/// Rewrites BOTH `plan.vertical_id` AND `plan.manifest.vertical.id` in
/// lock-step — they MUST stay equal, because `write_vertical_with_overlap`
/// round-trips through `argo_vertical::load_dir` and the handler then looks the
/// vertical up by `plan.vertical_id` (`registry.get(...)`); a mismatch fails
/// that round-trip. `WorkerConfig` + the eval-seed file derive from
/// `plan.vertical_id` at write time (after this call), so they inherit the
/// suffix automatically.
///
/// Suffix = a v7 UUID's leading 8 hex (timestamp-high → siblings roughly sort
/// by creation for a future GC sweep) + trailing 8 hex (random `rand_b` → ~32
/// bits of entropy disambiguating two generations in the SAME millisecond,
/// which the timestamp prefix alone does NOT). All lowercase `[0-9a-f]`, so the
/// id stays loader-legal. The suffix is NOT part of the role-contract digest
/// (which covers only the capability projection — see
/// [`argo_genesis::RoleContract::digest`]), so identical-capability
/// regenerations still confirm identically.
pub(super) fn apply_unique_vertical_suffix(plan: &mut GenesisPlan) {
    let suffixed = format!("{}-{}", plan.vertical_id, unique_suffix());
    plan.manifest.vertical.id = suffixed.clone();
    plan.vertical_id = suffixed;
}

/// A short, loader-legal, collision-resistant suffix: a v7 UUID's leading 8 hex
/// (timestamp-high) + trailing 8 hex (random tail).
fn unique_suffix() -> String {
    let hex = uuid::Uuid::now_v7().as_simple().to_string(); // 32 lowercase hex chars
    let prefix: String = hex.chars().take(8).collect();
    let random_tail: String = hex.chars().rev().take(8).collect();
    format!("{prefix}{random_tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_is_loader_legal_and_distinct_within_the_same_millisecond() {
        // Two suffixes minted back-to-back (same millisecond) must DIFFER — the
        // v7 timestamp prefix alone collides at sub-millisecond cadence, so the
        // random tail is what guarantees a regeneration never clobbers the
        // reviewed manifest. (This is exactly the bug the first implementation
        // hit: a timestamp-only suffix produced identical ids for rapid calls.)
        let a = unique_suffix();
        let b = unique_suffix();
        assert_ne!(a, b, "back-to-back suffixes must be distinct");
        for s in [&a, &b] {
            assert_eq!(s.len(), 16, "8 (ts) + 8 (random) hex chars");
            assert!(
                s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "loader-legal lowercase hex only: {s}"
            );
        }
    }

    #[test]
    fn stamp_keeps_plan_id_and_manifest_id_in_lockstep() {
        let mut plan =
            argo_genesis::plan("be my marketing intern, run weekly campaigns", "maya").unwrap();
        let base = plan.vertical_id.clone();
        apply_unique_vertical_suffix(&mut plan);
        assert_ne!(plan.vertical_id, base, "id must be suffixed");
        assert!(
            plan.vertical_id.starts_with(&format!("{base}-")),
            "suffix appends to the stable base: {}",
            plan.vertical_id
        );
        assert_eq!(
            plan.vertical_id, plan.manifest.vertical.id,
            "plan id and manifest id MUST stay equal (load_dir round-trip)"
        );
    }
}
