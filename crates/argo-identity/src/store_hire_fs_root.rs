//! Default `worker.fs_root` resolution for hires + the
//! [`IdentityStore::create_worker_from_draft`] commit method (moved here
//! from [`crate::store`] for LOC headroom — same sibling-impl pattern as
//! [`crate::store_role`] / [`crate::store_skills`]).
//!
//! Day-1 unblock (2026-06-11): a Genesis-minted teammate was hired with
//! `fs_root = NULL`, so even APPROVED file work jailed to the gateway
//! process cwd instead of the user's folder. The root rule lives here:
//! a Genesis hire with no explicit `fs_root` defaults to
//! `<home>/SkycotTeammates/<persona-slug>/`. This crate is store-layer —
//! it only COMPUTES and PERSISTS the path (a string column); the
//! `mkdir` side effect belongs to the gateway hire route
//! (`routes_workers_hire.rs`), which creates the directory fail-soft
//! after a successful hire.

use crate::store::IdentityStore;
use crate::store_hire::{assemble_worker_from_draft, validate_hire_draft, HireDraft};
use crate::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::path::{Path, PathBuf};

/// Directory under the user's home that collects per-teammate default
/// filesystem roots: `<home>/SkycotTeammates/<persona-slug>/`.
pub const TEAMMATES_DIR_NAME: &str = "SkycotTeammates";

/// Default filesystem jail root for a Genesis-hired teammate:
/// `<home>/SkycotTeammates/<persona-slug>/`, where the persona slug is
/// the sanitized lowercase `suggested_name` (alphanumerics kept,
/// spaces/dashes/underscores collapse to `-`, everything else dropped;
/// `"worker"` when nothing survives).
///
/// ```
/// use argo_identity::default_genesis_fs_root;
/// use std::path::Path;
/// let root = default_genesis_fs_root(Path::new("/home/sam"), "Ava");
/// assert_eq!(root, Path::new("/home/sam/SkycotTeammates/ava"));
/// ```
#[must_use]
pub fn default_genesis_fs_root(home: &Path, suggested_name: &str) -> PathBuf {
    home.join(TEAMMATES_DIR_NAME).join(persona_slug(suggested_name))
}

/// Resolve the effective `fs_root` for a hire draft.
///
/// Precedence:
/// 1. An explicit non-empty `draft.fs_root` always wins.
/// 2. A Genesis draft (non-empty `genesis_plan_id`) with a known `home`
///    defaults to [`default_genesis_fs_root`].
/// 3. Otherwise `None` — a non-Genesis hire keeps today's behaviour
///    (`worker.fs_root` stays `NULL`).
///
/// `home` is a parameter (rather than read from the environment) so the
/// resolution is pure and deterministic; production callers pass the
/// `HOME` env var via [`resolved_fs_root_from_env`].
///
/// ```
/// use argo_identity::{resolved_fs_root, ApprovalPolicyChoice, HireDraft, HireDutyDraft};
/// use std::path::Path;
/// let mut draft = HireDraft {
///     suggested_name: "Ava".into(),
///     charter: "Read invoice files and draft reminders.".into(),
///     suggested_vertical: "genesis-ava-1".into(),
///     approval_policy: ApprovalPolicyChoice { level: "balanced".into() },
///     duties: vec![HireDutyDraft {
///         cron: "0 9 * * 1-5".into(),
///         entry_action: "follow_up".into(),
///         description: "Follow up.".into(),
///         default_enabled: true,
///     }],
///     voice_profile: None,
///     tagline: None,
///     genesis_plan_id: Some("genesis-ava-1".into()),
///     fs_root: None,
/// };
/// let home = Path::new("/home/sam");
/// assert_eq!(
///     resolved_fs_root(&draft, Some(home)),
///     Some(Path::new("/home/sam/SkycotTeammates/ava").to_path_buf()),
/// );
/// draft.fs_root = Some("/data/inbox".into());
/// assert_eq!(
///     resolved_fs_root(&draft, Some(home)),
///     Some(Path::new("/data/inbox").to_path_buf()),
/// );
/// draft.fs_root = None;
/// draft.genesis_plan_id = None;
/// assert_eq!(resolved_fs_root(&draft, Some(home)), None);
/// ```
#[must_use]
pub fn resolved_fs_root(draft: &HireDraft, home: Option<&Path>) -> Option<PathBuf> {
    if let Some(explicit) = draft.fs_root.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(explicit));
    }
    let is_genesis = draft
        .genesis_plan_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());
    if !is_genesis {
        return None;
    }
    home.map(|h| default_genesis_fs_root(h, &draft.suggested_name))
}

/// [`resolved_fs_root`] with `home` read from the `HOME` environment
/// variable — the production seam used by both the store commit path
/// and the gateway hire route's fail-soft `mkdir`. Never hardcodes a
/// platform prefix; when `HOME` is unset the Genesis default is simply
/// skipped (explicit `draft.fs_root` still wins).
///
/// ```
/// use argo_identity::{resolved_fs_root_from_env, ApprovalPolicyChoice, HireDraft};
/// let draft = HireDraft {
///     suggested_name: "Ava".into(),
///     charter: "Read invoice files and draft reminders.".into(),
///     suggested_vertical: "support".into(),
///     approval_policy: ApprovalPolicyChoice { level: "balanced".into() },
///     duties: vec![],
///     voice_profile: None,
///     tagline: None,
///     genesis_plan_id: None,
///     fs_root: Some("/data/inbox".into()),
/// };
/// assert_eq!(
///     resolved_fs_root_from_env(&draft).as_deref(),
///     Some(std::path::Path::new("/data/inbox")),
/// );
/// ```
#[must_use]
pub fn resolved_fs_root_from_env(draft: &HireDraft) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    resolved_fs_root(draft, home.as_deref())
}

/// Sanitize a display name into a lowercase persona slug: alphanumerics
/// kept (lowercased), spaces / dashes / underscores collapse to `-`,
/// everything else dropped. Mirrors `argo_genesis::name_to_persona_slug`
/// by value (`argo-identity` does not depend on `argo-genesis`).
fn persona_slug(name: &str) -> String {
    let mut out = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '_' {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("worker");
    }
    out
}

impl IdentityStore<'_> {
    /// Commit a worker from a hire draft (Stream J5).
    ///
    /// Assembles a [`crate::Worker`] from `draft`, the caller's
    /// `tenant_id`, and the `principal_id` of the operator placing the
    /// hire (stored as `persona.builder_attribution`), forwards to the
    /// existing [`Self::create_worker`] transactional path, then stamps
    /// the resolved `worker.fs_root` ([`resolved_fs_root_from_env`]) —
    /// the explicit draft value, or the Genesis default
    /// `<home>/SkycotTeammates/<persona-slug>/`, or `NULL`. Validation
    /// errors surface as [`ArgoError::Schema`] so the HTTP layer can map
    /// them to 400; SQL errors continue to surface as
    /// [`ArgoError::Provider`].
    ///
    /// ```
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// use argo_identity::{ApprovalPolicyChoice, HireDraft, HireDutyDraft, IdentityStore};
    /// use argo_store::SessionStore;
    /// use argo_types::TenantId;
    /// let store = SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// let draft = HireDraft {
    ///     suggested_name: "Maya".into(),
    ///     charter: "Own outbound follow-up end-to-end.".into(),
    ///     suggested_vertical: "marketplace-seller".into(),
    ///     approval_policy: ApprovalPolicyChoice { level: "balanced".into() },
    ///     duties: vec![HireDutyDraft {
    ///         cron: "0 9 * * 1-5".into(),
    ///         entry_action: "follow_up".into(),
    ///         description: "Send approved follow-up.".into(),
    ///         default_enabled: true,
    ///     }],
    ///     voice_profile: None,
    ///     tagline: None,
    ///     genesis_plan_id: None,
    ///     fs_root: None,
    /// };
    /// let id = IdentityStore::new(&store)
    ///     .create_worker_from_draft(&draft, TenantId::new(), "human:op")
    ///     .await?;
    /// assert!(!id.to_string().is_empty());
    /// # Ok(()) }
    /// ```
    pub async fn create_worker_from_draft(
        &self,
        draft: &HireDraft,
        tenant_id: TenantId,
        principal_id: &str,
    ) -> ArgoResult<WorkerId> {
        validate_hire_draft(draft)?;
        let worker = assemble_worker_from_draft(draft, tenant_id, principal_id);
        self.create_worker(&worker).await?;
        if let Some(root) = resolved_fs_root_from_env(draft) {
            self.persist_fs_root(tenant_id, worker.id, &root).await?;
        }
        Ok(worker.id)
    }

    /// Stamp `worker.fs_root` for a freshly-committed hire. Kept as a
    /// plain UPDATE (not part of the `create_worker` INSERT) so the
    /// shared transactional path — used by CLI onboarding and tests
    /// that build [`crate::Worker`] values directly — is untouched.
    async fn persist_fs_root(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        root: &Path,
    ) -> ArgoResult<()> {
        sqlx::query("UPDATE worker SET fs_root = ? WHERE id = ? AND tenant_id = ?")
            .bind(root.display().to_string())
            .bind(worker_id.to_string())
            .bind(tenant_id.to_string())
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("update worker fs_root: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "store_hire_fs_root_tests.rs"]
mod tests;
