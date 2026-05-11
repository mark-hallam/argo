//! Session-boot hint: emit `tracing::info!` per skill that has a
//! pending `skill_version_migration` row
//! (`mission-skill-version-migration-path`).
//!
//! Loaded via `#[path = "turn_skills_migration.rs"] mod
//! turn_skills_migration;` from `turn.rs` (mirrors the
//! `turn_skills_runtime.rs` sibling pattern). Keeps the lookup loop
//! out of `turn.rs` so that file stays under the 500-LOC HARD RULE
//! 6 ceiling.
//!
//! Intentionally side-effect-free aside from the tracing event:
//! actual auto-upgrade execution at session boot is out of scope
//! for this brief — `argo skill upgrade` is the operator-driven
//! application path. Lookup errors are warned-and-swallowed; this
//! must never fail the turn.
use argo_skill::{lookup_migration, Skill};
use argo_store::SessionStore;

/// For each skill in `active`, look up a pending migration row and
/// emit one `tracing::info!` per match. Errors are logged at `warn`
/// and SWALLOWED — matching the brief's "no automatic side
/// effects" + "side channel that must never fail the turn"
/// guidance.
pub(crate) async fn emit_pending_migration_hints(store: &SessionStore, active: &[Skill]) {
    for s in active {
        match lookup_migration(store, s.id, &s.frontmatter.version).await {
            Ok(Some(m)) => tracing::info!(
                skill = s.frontmatter.name.as_str(),
                from = m.from_version.as_str(),
                to = m.to_version.as_str(),
                auto = m.auto_upgrade,
                "skill_migration_available"
            ),
            Ok(None) => {}
            Err(e) => tracing::warn!(
                skill = s.frontmatter.name.as_str(),
                "skill migration lookup failed (turn unaffected): {e}"
            ),
        }
    }
}
