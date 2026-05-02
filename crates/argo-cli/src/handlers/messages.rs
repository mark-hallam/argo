//! `argo messages …` — workspace-wide message rollups. Mirrors the
//! gateway's `/messages/*` family. Currently dispatches `by-day`
//! (mirrors #350-era /messages/by-day) and `by-role` (mirrors
//! #611's /messages/by-role). Lives as its own top-level group
//! parallel to `argo audit` so the messages-table surface has a
//! natural CLI home.
use crate::cli::MessagesCmd;
use argo_types::ArgoResult;

pub async fn run(cmd: MessagesCmd) -> ArgoResult<()> {
    match cmd {
        MessagesCmd::ByDay { days, json } => {
            super::messages_by_day::run(days, json).await
        }
        MessagesCmd::ByRole {
            since_minutes,
            json,
        } => super::messages_by_role::run(since_minutes, json).await,
        MessagesCmd::BySession {
            limit,
            since_minutes,
            json,
        } => super::messages_by_session::run(limit, since_minutes, json).await,
    }
}
