//! `argo-delegate` — subagent delegation: budget tree + COW
//! workspace fork (§16, Amendment 7).
//!
//! Three primitives:
//!
//! - [`DelegateOptions`] — per-call knobs, including the
//!   fallback-chain inheritance rule (Amendment 7). The agent loop
//!   constructs one per `delegate_task` invocation.
//! - [`WorkspaceFork`] — eager-COW copy of the parent workspace into a
//!   sibling temp dir. Children operate on `child_root()`; on
//!   completion, [`WorkspaceFork::merge_back`] returns a
//!   [`MergeReport`] of applied / conflicting / deleted files.
//! - Budget-tree integration via [`argo_types::BudgetNode::fork`] —
//!   a property test (`budget_tree::total_spend_never_exceeds_root_budget`)
//!   covers the §16.4 acceptance.
//!
//! ```
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! # let dir = tempfile::tempdir().unwrap();
//! # let parent = dir.path().join("parent");
//! # tokio::fs::create_dir_all(&parent).await.unwrap();
//! # tokio::fs::write(parent.join("a.txt"), "v1").await.unwrap();
//! use argo_delegate::WorkspaceFork;
//! let fork = WorkspaceFork::fork(&parent).await?;
//! tokio::fs::write(fork.child_root().join("a.txt"), "v2").await.unwrap();
//! let report = fork.merge_back().await?;
//! assert!(report.conflicts.is_empty());
//! fork.cleanup().await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod options;
mod workspace;

#[cfg(test)]
mod budget_tree;

pub use options::DelegateOptions;
pub use workspace::{MergeReport, WorkspaceFork};
