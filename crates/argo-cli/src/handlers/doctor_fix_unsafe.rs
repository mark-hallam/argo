//! Unsafe `argo doctor fix` actions — surface remediation hints only.

use super::doctor_fix::{FixContext, FixReport, POLICY_RELPATH};
use crate::home::db_path;

pub(super) fn run_unsafe(ctx: &FixContext, id: &'static str) -> FixReport {
    match id {
        "database_corruption" => FixReport::unsafe_issue(
            id,
            format!(
                "database at {} requires manual recovery",
                db_path(&ctx.home).display()
            ),
            "restore sessions.db from backup, or move it aside and re-run setup",
        ),
        "missing_api_keys" => FixReport::unsafe_issue(
            id,
            "missing API keys require operator-supplied secrets",
            "set the missing keys in the operator environment or credential store",
        ),
        "workspace_policy_bare" => FixReport::unsafe_issue(
            id,
            "workspace policy bare-mode enabled",
            "disable enabled=true or add per-action/daily spend caps",
        )
        .with_path(&ctx.cwd.join(POLICY_RELPATH)),
        _ => FixReport::blocked(
            id,
            "unsafe",
            "unknown unsafe fix",
            "choose a known issue id",
        ),
    }
}
