//! `GET /api/v1/workers` - canonical collection alias for the
//! tenant-scoped worker roster.
//!
//! The historical list endpoint is `/api/v1/team`; this route keeps
//! the same payload contract while letting API consumers address the
//! resource with the same noun used by singleton worker routes.

use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    super::super::routes_team_list::list_response(req, store).await
}
