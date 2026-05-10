//! Optional screenshot-persistence hook for the CU executor.
//!
//! The executor lives below `argo-cli`, so it cannot reach
//! `crates/argo-cli/src/handlers/cu_screenshots.rs::CuScreenshotStore`
//! directly without a cyclic dependency. This trait is the seam: a
//! caller (the CLI / gateway boot wiring) plumbs an
//! `Arc<dyn ScreenshotPersister>` through `with_screenshot_persister`
//! and the executor invokes it after each step's screenshot capture.
//!
//! Returning `BlobRef` (the lightweight pointer from
//! `argo-store::attached_blob`) lets the caller stamp the reference
//! into the trajectory log without dragging the PNG bytes back.

use argo_store::attached_blob::BlobRef;
use argo_types::{ArgoResult, SessionId};
use async_trait::async_trait;
use std::sync::Arc;

/// Persist a CU screenshot for `(tenant, session, action, step)`.
///
/// Implementations are expected to be cheap to clone (typical:
/// `Arc<Inner>`) and safe to call concurrently across worker tasks.
/// `put_screenshot` returning `Err` does not abort the CU loop —
/// the caller logs and continues so a flaky blob backend never
/// fails an in-progress action.
#[async_trait]
pub trait ScreenshotPersister: Send + Sync + std::fmt::Debug {
    /// Persist `png` and return the lightweight `BlobRef`.
    async fn put_screenshot(
        &self,
        tenant_id: &str,
        session_id: &str,
        action_id: &str,
        step_n: u32,
        png: &[u8],
    ) -> ArgoResult<BlobRef>;
}

/// Best-effort: push `png` through the optional persister, logging
/// and swallowing errors so a flaky blob backend never fails the
/// in-progress CU loop. No-op when persister or session is `None`.
pub async fn try_persist(
    persister: Option<&Arc<dyn ScreenshotPersister>>,
    tenant_id: &str,
    action_id: &str,
    session: Option<&SessionId>,
    step: u32,
    png: &[u8],
) {
    let Some(persister) = persister else { return };
    let Some(session) = session else { return };
    let session_str = session.to_string();
    if let Err(e) = persister
        .put_screenshot(tenant_id, &session_str, action_id, step, png)
        .await
    {
        tracing::warn!(
            target: "cu::screenshot_persist",
            action = %action_id,
            step,
            error = %e,
            "screenshot persist failed; continuing"
        );
    }
}
