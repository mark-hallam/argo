//! Per-domain dispatch sub-fns for the `Session / Handoff / Subtask /
//! Action` Command variants and the session-lifecycle slice of
//! `RuntimeCmd` (`New / Resume / Chat / Steer / Trace /
//! Recover / RecoverCache`). Each fn is the body of one top-level
//! `match Command::*` arm extracted out of `main.rs`
//! (`claude-argo-cli-main-rs-split`); semantics are byte-identical
//! to the pre-split dispatch.

use crate::action_replay;
use crate::cli;
use crate::handlers;
use argo_types::ArgoResult;

pub(crate) async fn session(cmd: cli::SessionCmd) -> ArgoResult<()> {
    handlers::session::run(cmd).await
}

pub(crate) async fn handoff(cmd: cli::HandoffCmd) -> ArgoResult<()> {
    handlers::session::run_handoff(cmd).await
}

pub(crate) async fn subtask(cmd: cli::SubtaskCmd) -> ArgoResult<()> {
    match cmd {
        cli::SubtaskCmd::List {
            session,
            worker,
            parent_goal,
            state,
            limit,
            json,
        } => {
            handlers::subtask_list::run(
                session.as_deref(),
                worker.as_deref(),
                parent_goal.as_deref(),
                state.as_deref(),
                limit,
                json,
            )
            .await
        }
        cli::SubtaskCmd::Show { id, json } => handlers::subtask_show::run(&id, json).await,
        cli::SubtaskCmd::Tree { session, json } => {
            handlers::subtask_tree::run(&session, json).await
        }
        cli::SubtaskCmd::Budget { root_id, json } => {
            handlers::subtask_budget::run(&root_id, json).await
        }
    }
}

pub(crate) async fn action(cmd: cli::ActionCmd) -> ArgoResult<()> {
    match cmd {
        cli::ActionCmd::Replay {
            action_id,
            executor,
            dry_run,
            bundle_dir,
        } => {
            action_replay::run(cli::ActionCmd::Replay {
                action_id,
                executor,
                dry_run,
                bundle_dir,
            })
            .await
        }
        cli::ActionCmd::History {
            worker_id,
            since,
            limit,
            include_skill,
            json,
        } => {
            handlers::action_history::run(
                &worker_id,
                since.as_deref(),
                limit,
                include_skill,
                json,
            )
            .await
        }
        cli::ActionCmd::Timeline {
            worker_id,
            since,
            bucket_mins,
            json,
        } => handlers::action_timeline::run(&worker_id, &since, bucket_mins, json).await,
        cli::ActionCmd::Recent {
            tenant,
            worker,
            limit,
            json,
        } => handlers::action_recent::run(tenant.as_deref(), worker.as_deref(), limit, json).await,
    }
}

pub(crate) async fn runtime_new(
    provider: String,
    model: String,
    title: Option<String>,
    vertical: Option<String>,
) -> ArgoResult<()> {
    handlers::new::run(&provider, &model, title.as_deref(), vertical.as_deref()).await
}

pub(crate) async fn runtime_resume(id: String) -> ArgoResult<()> {
    handlers::resume::run(&id).await
}

pub(crate) async fn runtime_chat(
    id: String,
    user: Option<String>,
    stream: bool,
    interactive: bool,
    max_tool_rounds: Option<usize>,
) -> ArgoResult<()> {
    handlers::chat::run(&id, user.as_deref(), stream, interactive, max_tool_rounds).await
}

pub(crate) async fn runtime_steer(session_id: String, text: String) -> ArgoResult<()> {
    handlers::steer::run(&session_id, &text).await
}

pub(crate) fn runtime_trace_explore(
    session: String,
    format: String,
    path: Option<String>,
) -> ArgoResult<()> {
    let fmt = handlers::trace_explore::TraceFormat::parse(&format)?;
    let path_buf = path.map(std::path::PathBuf::from);
    handlers::trace_explore::run(&session, fmt, path_buf.as_deref())
}

pub(crate) async fn runtime_trace(
    turn: Option<String>,
    session: Option<String>,
) -> ArgoResult<()> {
    handlers::trace::run(turn, session).await
}

pub(crate) async fn runtime_recover(session_id: String, json: bool) -> ArgoResult<()> {
    handlers::recover::run(&session_id, json).await
}

pub(crate) async fn runtime_recover_cache(cmd: cli::cli_recover_cmds::RecoverCacheCmd) -> ArgoResult<()> {
    handlers::recover_cache::run(cmd).await
}
