//! High-level façade over `Sandbox` + `ops` — used by the
//! filesystem `Executor` impl in argo-cli. Bundles "open the
//! sandbox", "parse the action's JSON payload", and "run the op"
//! into one type so the call site stays small.
//!
//! Three pieces fit together:
//!   * `FsConfig` — sandbox root + caps. Construct with
//!     `with_defaults(&root)` for a sane default; build a
//!     deterministic per-worker root via `default_root_for(id)`.
//!   * `FsAction` — typed result of parsing
//!     `action.payload`. Carries the path + payload bytes ready
//!     to feed into the ops layer.
//!   * `FsExecutor` — owns a `Sandbox`, runs an `FsAction`,
//!     returns a `FsOpOutcome` the caller can serialise into the
//!     audit ledger.
use crate::error::{FsExecutorError, FsResult};
use crate::ops::{self, DirEntry, FileMeta};
use crate::sandbox::Sandbox;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default per-worker root prefix. Per-worker scoping keeps two
/// workers from stepping on each other's files even if both are
/// running on the same machine.
pub const DEFAULT_FS_ROOT_PREFIX: &str = "/tmp/argo-fs";

/// Configuration handed to `FsExecutor::new`. Carries the absolute
/// sandbox root + caps; the executor opens a `Sandbox` lazily so a
/// missing `/tmp/argo-fs/<worker_id>/` is fine until first use.
#[derive(Debug, Clone)]
pub struct FsConfig {
    root: PathBuf,
}

impl FsConfig {
    /// Build a config rooted at `root` with the default caps from
    /// `crate::sandbox` (16 MiB per-op, 1 GiB per-worker total).
    /// Caps still pick up env overrides via the `Sandbox::open`
    /// path.
    #[must_use]
    pub fn with_defaults(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// Return the deterministic per-worker root we'd hand a worker
    /// when no explicit `--fs-root` was provided at spawn time.
    #[must_use]
    pub fn default_root_for(worker_id: &str) -> PathBuf {
        PathBuf::from(DEFAULT_FS_ROOT_PREFIX).join(worker_id)
    }

    /// Borrow the configured root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Parsed action ready to dispatch. Mirrors `FsOpKind` but carries
/// the per-op arguments so the caller can run it straight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FsAction {
    ReadFile { path: String },
    WriteFile { path: String, body_utf8: String },
    ListDir { path: String },
    Mkdir { path: String },
    Remove { path: String },
    Stat { path: String },
}

/// What an `FsAction` produced. Serialisable so the audit ledger
/// can ingest it via `serde_json::to_value`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FsOpOutcome {
    ReadFile {
        path: String,
        size: u64,
        body_utf8: String,
    },
    WriteFile {
        path: String,
        abs_path: String,
        size: u64,
    },
    ListDir {
        path: String,
        entries: Vec<DirEntry>,
    },
    Mkdir {
        path: String,
        abs_path: String,
    },
    Remove {
        path: String,
    },
    Stat {
        path: String,
        meta: FileMeta,
    },
}

/// Higher-level façade over `Sandbox` + `ops`. Holds an opened
/// sandbox and dispatches one `FsAction` per `run` call.
pub struct FsExecutor {
    sandbox: Sandbox,
}

impl FsExecutor {
    /// Open the sandbox at `config.root` (creating it if missing)
    /// and wrap it in an executor. Sandbox-open failures (canonical
    /// path, mkdir) surface as `ArgoError::Provider` so the call
    /// site can `?`-propagate without conversion boilerplate.
    pub fn new(config: FsConfig) -> ArgoResult<Self> {
        let sandbox = Sandbox::open(config.root()).map_err(to_argo)?;
        Ok(Self { sandbox })
    }

    /// Borrow the underlying sandbox (e.g. for tests that want to
    /// poke at the on-disk layout without going through `run`).
    #[must_use]
    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    /// Parse a JSON payload (typically `action.payload` from the
    /// worker action ledger) into a typed `FsAction`. The wire
    /// shape mirrors `FsAction`'s serde derive — a `{op, path,
    /// body_utf8?}` envelope.
    pub fn parse_action(value: &serde_json::Value) -> ArgoResult<FsAction> {
        serde_json::from_value::<FsAction>(value.clone()).map_err(|e| {
            ArgoError::Provider(format!(
                "filesystem action payload: {e}; expected {{op, path, body_utf8?}}"
            ))
        })
    }

    /// Run one `FsAction` against the open sandbox and return its
    /// outcome. All sandbox-violation / quota / I/O errors surface
    /// as `ArgoError::Provider` so the audit ledger sees the
    /// underlying message.
    pub fn run(&self, action: FsAction) -> ArgoResult<FsOpOutcome> {
        let sb = &self.sandbox;
        let result: FsResult<FsOpOutcome> = match action {
            FsAction::ReadFile { path } => {
                let bytes = ops::read_file(sb, &path)?;
                Ok(FsOpOutcome::ReadFile {
                    size: bytes.len() as u64,
                    body_utf8: String::from_utf8_lossy(&bytes).into_owned(),
                    path,
                })
            }
            FsAction::WriteFile { path, body_utf8 } => {
                let abs = ops::write_file(sb, &path, body_utf8.as_bytes())?;
                Ok(FsOpOutcome::WriteFile {
                    size: body_utf8.len() as u64,
                    abs_path: abs.display().to_string(),
                    path,
                })
            }
            FsAction::ListDir { path } => {
                let entries = ops::list_dir(sb, &path)?;
                Ok(FsOpOutcome::ListDir { path, entries })
            }
            FsAction::Mkdir { path } => {
                let abs = ops::mkdir(sb, &path)?;
                Ok(FsOpOutcome::Mkdir {
                    abs_path: abs.display().to_string(),
                    path,
                })
            }
            FsAction::Remove { path } => {
                ops::remove(sb, &path)?;
                Ok(FsOpOutcome::Remove { path })
            }
            FsAction::Stat { path } => {
                let meta = ops::stat(sb, &path)?;
                Ok(FsOpOutcome::Stat { path, meta })
            }
        };
        result.map_err(to_argo)
    }
}

fn to_argo(e: FsExecutorError) -> ArgoError {
    ArgoError::Provider(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn default_root_for_uses_per_worker_prefix() {
        let p = FsConfig::default_root_for("w-123");
        assert!(p.starts_with(DEFAULT_FS_ROOT_PREFIX));
        assert!(p.ends_with("w-123"));
    }

    #[test]
    fn parse_action_round_trips_write_file_envelope() {
        let payload = json!({
            "op": "write_file",
            "path": "notes/today.md",
            "body_utf8": "hi"
        });
        let action = FsExecutor::parse_action(&payload).unwrap();
        assert_eq!(
            action,
            FsAction::WriteFile {
                path: "notes/today.md".into(),
                body_utf8: "hi".into()
            }
        );
    }

    #[test]
    fn parse_action_rejects_garbage_payload() {
        let payload = json!({"op": "nuke_universe", "path": "x"});
        let err = FsExecutor::parse_action(&payload).unwrap_err();
        assert!(err.to_string().contains("filesystem action payload"));
    }

    #[test]
    fn run_write_then_read_round_trips_via_executor() {
        let dir = tempdir().unwrap();
        let exec = FsExecutor::new(FsConfig::with_defaults(dir.path())).unwrap();
        let outcome = exec
            .run(FsAction::WriteFile {
                path: "hello.txt".into(),
                body_utf8: "hi there".into(),
            })
            .unwrap();
        assert!(matches!(outcome, FsOpOutcome::WriteFile { size: 8, .. }));
        let outcome = exec
            .run(FsAction::ReadFile {
                path: "hello.txt".into(),
            })
            .unwrap();
        match outcome {
            FsOpOutcome::ReadFile { body_utf8, .. } => assert_eq!(body_utf8, "hi there"),
            other => panic!("expected ReadFile, got {other:?}"),
        }
    }

    #[test]
    fn run_propagates_path_traversal_as_argo_error() {
        let dir = tempdir().unwrap();
        let exec = FsExecutor::new(FsConfig::with_defaults(dir.path())).unwrap();
        let err = exec
            .run(FsAction::ReadFile {
                path: "../escape".into(),
            })
            .unwrap_err();
        assert!(err.to_string().contains("path escapes sandbox root"));
    }

    #[test]
    fn run_stat_reports_missing_path_without_error() {
        let dir = tempdir().unwrap();
        let exec = FsExecutor::new(FsConfig::with_defaults(dir.path())).unwrap();
        let outcome = exec
            .run(FsAction::Stat {
                path: "ghost.txt".into(),
            })
            .unwrap();
        match outcome {
            FsOpOutcome::Stat { meta, .. } => assert!(!meta.exists),
            other => panic!("expected Stat, got {other:?}"),
        }
    }

    #[test]
    fn run_list_dir_returns_sorted_entries() {
        let dir = tempdir().unwrap();
        let exec = FsExecutor::new(FsConfig::with_defaults(dir.path())).unwrap();
        exec.run(FsAction::WriteFile {
            path: "b.txt".into(),
            body_utf8: "b".into(),
        })
        .unwrap();
        exec.run(FsAction::WriteFile {
            path: "a.txt".into(),
            body_utf8: "a".into(),
        })
        .unwrap();
        let outcome = exec
            .run(FsAction::ListDir {
                path: ".".into(),
            })
            .unwrap();
        match outcome {
            FsOpOutcome::ListDir { entries, .. } => {
                let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
                assert_eq!(names, vec!["a.txt", "b.txt"]);
            }
            other => panic!("expected ListDir, got {other:?}"),
        }
    }
}
