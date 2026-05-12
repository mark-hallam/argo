//! OOM-kill detection + bounded exponential-backoff worker
//! restart (mission-oom-restart-handler / MISSION_BACKLOG §8
//! Claim 8 #237 — Tier P reliability).
//!
//! When a worker subprocess dies from an out-of-memory kill the
//! supervisor today sees a generic "process exited with status
//! X" and the brief is lost. This module is the supervisor-level
//! seam that fixes that:
//!
//! - [`classify_exit`] / [`classify_hung`] derive a typed
//!   [`WorkerExitReason`].
//! - [`OomRestartCoordinator`] + [`decide_restart`] enforce a
//!   rolling-window restart budget with exponential backoff.
//! - [`emit_audit_row`] appends a `worker.oom_restart` row to
//!   the audit chain for every non-clean exit.
//!
//! ## Platform matrix
//!
//! | Platform | OOM signal     | Code path                         |
//! | -------- | -------------- | --------------------------------- |
//! | Linux    | signal 9       | `ExitStatus::signal() == Some(9)` |
//! | Linux    | shell exit 137 | `ExitStatus::code() == Some(137)` |
//! | macOS    | signal 9       | `ExitStatus::signal() == Some(9)` |
//! | WSL2     | signal 9       | Linux path                        |
//! | Windows  | n/a            | Falls through to `OtherSignal`    |
//!
//! On Linux, the classifier also tries to read
//! `/sys/fs/cgroup/<cgroup>/memory.events` for an `oom_kill`
//! line — best-effort; absence does not flip the verdict.
//!
//! Per-action transient retries live in
//! `crates/argo-executor-computer-use/src/restart.rs`. This
//! module is the *whole-process death* path.

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::process::ExitStatus;
use std::time::Duration;

/// Stable `kind` label that supervisors pass to
/// `AuditSink::write` when persisting an OOM-restart row. Lives
/// alongside the [`AuditKind::WorkerOomRestart`] enum variant in
/// `argo-audit`; duplicated here as a literal so this crate
/// does not need to take a dependency on `argo-audit` (the
/// `argo-store → argo-runtime` edge in the workspace graph
/// would otherwise close into a cycle).
pub const AUDIT_KIND_LABEL: &str = "worker.oom_restart";

/// Shell-wrapped exit code for `SIGKILL` (128 + 9).
pub const EXIT_CODE_OOM_KILL: i32 = 137;
/// Unix `SIGKILL` signal number.
pub const SIGNAL_OOM_KILL: i32 = 9;
/// Default restart cap per [`OomRestartConfig::restart_window`].
pub const DEFAULT_MAX_RESTARTS: u32 = 3;
/// Default rolling-window length.
pub const DEFAULT_RESTART_WINDOW: Duration = Duration::from_secs(15 * 60);
/// Default first-attempt backoff.
pub const DEFAULT_BASE_BACKOFF: Duration = Duration::from_secs(2);
/// Default exponential-backoff ceiling.
pub const DEFAULT_MAX_BACKOFF: Duration = Duration::from_secs(120);

/// Typed classification of a worker subprocess exit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkerExitReason {
    /// Worker exited normally with a numeric return code.
    Clean {
        /// Process exit code (typically `0`).
        code: i32,
    },
    /// Killed by the kernel OOM killer / cgroup memory breach /
    /// `kill -9`. `cgroup_evidence` carries the `oom_kill <count>`
    /// line from `memory.events` when available.
    OomKilled {
        /// Reported signal number (typically `9`).
        signal: i32,
        /// Best-effort cgroup-v2 evidence string.
        cgroup_evidence: Option<String>,
    },
    /// Killed by a non-`SIGKILL` signal (`SIGTERM`, `SIGSEGV`, …).
    OtherSignal {
        /// Reported signal number.
        signal: i32,
    },
    /// Process alive but `last_heartbeat` exceeds the configured
    /// `hung_threshold`. Surfaces from [`classify_hung`].
    Hung {
        /// Last observed heartbeat timestamp.
        last_heartbeat: DateTime<Utc>,
    },
}

impl WorkerExitReason {
    /// `true` for exits that warrant supervisor reaction (audit
    /// row + restart-decision pass). Clean exits skip both.
    #[must_use]
    pub fn is_actionable(&self) -> bool {
        !matches!(self, Self::Clean { .. })
    }
}

/// Tunables for the rolling restart-budget window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OomRestartConfig {
    /// Restart attempts allowed within [`Self::restart_window`].
    pub max_restarts: u32,
    /// First-attempt backoff; doubles each attempt up to
    /// [`Self::max_backoff`].
    pub base_backoff: Duration,
    /// Ceiling on exponential backoff.
    pub max_backoff: Duration,
    /// Rolling-window length.
    pub restart_window: Duration,
}

impl Default for OomRestartConfig {
    fn default() -> Self {
        Self {
            max_restarts: DEFAULT_MAX_RESTARTS,
            base_backoff: DEFAULT_BASE_BACKOFF,
            max_backoff: DEFAULT_MAX_BACKOFF,
            restart_window: DEFAULT_RESTART_WINDOW,
        }
    }
}

/// Tracks restart attempts in a rolling window. Built once per
/// worker id by the supervisor.
#[derive(Debug, Clone)]
pub struct OomRestartCoordinator {
    config: OomRestartConfig,
    restart_history: VecDeque<(DateTime<Utc>, WorkerExitReason)>,
}

impl OomRestartCoordinator {
    /// Build a coordinator with explicit config.
    #[must_use]
    pub fn new(config: OomRestartConfig) -> Self {
        Self {
            config,
            restart_history: VecDeque::new(),
        }
    }

    /// Build with documented defaults (3 restarts / 15 min,
    /// 2s base / 2min ceiling).
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(OomRestartConfig::default())
    }

    /// Borrow the config for callers that need to clamp against
    /// the operator-configured budget.
    #[must_use]
    pub fn config(&self) -> &OomRestartConfig {
        &self.config
    }

    fn prune_window(&mut self, now: DateTime<Utc>) {
        let Ok(window) = ChronoDuration::from_std(self.config.restart_window) else {
            return;
        };
        let cutoff = now - window;
        while let Some((ts, _)) = self.restart_history.front() {
            if *ts < cutoff {
                self.restart_history.pop_front();
            } else {
                break;
            }
        }
    }

    /// Attempts still within the rolling window at time `now`.
    pub fn attempts_in_window(&mut self, now: DateTime<Utc>) -> u32 {
        self.prune_window(now);
        u32::try_from(self.restart_history.len()).unwrap_or(u32::MAX)
    }

    /// Record a restart attempt. Exposed so tests can pre-seed
    /// without mocking time.
    pub fn record_attempt(&mut self, now: DateTime<Utc>, reason: WorkerExitReason) {
        self.restart_history.push_back((now, reason));
    }
}

/// Outcome of [`decide_restart`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RestartDecision {
    /// Sleep `after_backoff`, then respawn with the same brief.
    Restart {
        /// Backoff before respawn (`base * 2^(attempt-1)`,
        /// clamped at [`OomRestartConfig::max_backoff`]).
        after_backoff: Duration,
        /// 1-based attempt counter within the rolling window.
        attempt: u32,
    },
    /// Budget exhausted. Supervisor releases the worker's brief
    /// claim (CLAIMED → TODO with the embedded `salvage_note`).
    GiveUp {
        /// Salvage note destined for the ledger row.
        salvage_note: String,
    },
    /// Worker exited cleanly — supervisor takes its existing
    /// clean-shutdown path.
    IgnoreCleanExit,
}

/// Derive a [`WorkerExitReason`] from a real
/// [`std::process::ExitStatus`].
///
/// `cgroup_path` is an optional cgroup-v2 directory (e.g.
/// `/sys/fs/cgroup/argo-workers/<id>`); when supplied the
/// classifier best-effort reads `memory.events` to populate
/// `cgroup_evidence`.
///
/// ```
/// # #[cfg(unix)]
/// # {
/// use std::os::unix::process::ExitStatusExt;
/// use argo_runtime::oom_restart::{classify_exit, WorkerExitReason};
/// let status = std::process::ExitStatus::from_raw(9);
/// assert!(matches!(
///     classify_exit(status, None),
///     WorkerExitReason::OomKilled { .. }
/// ));
/// # }
/// ```
#[must_use]
pub fn classify_exit(status: ExitStatus, cgroup_path: Option<&str>) -> WorkerExitReason {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            if signal == SIGNAL_OOM_KILL {
                return WorkerExitReason::OomKilled {
                    signal,
                    cgroup_evidence: cgroup_path.and_then(read_cgroup_oom_evidence),
                };
            }
            return WorkerExitReason::OtherSignal { signal };
        }
    }
    let code = status.code().unwrap_or(-1);
    if code == EXIT_CODE_OOM_KILL {
        return WorkerExitReason::OomKilled {
            signal: SIGNAL_OOM_KILL,
            cgroup_evidence: cgroup_path.and_then(read_cgroup_oom_evidence),
        };
    }
    WorkerExitReason::Clean { code }
}

/// Companion path for the alive-but-stuck case: returns
/// `Some(Hung)` when `last_heartbeat` is older than
/// `hung_threshold`. The supervisor drives this on its
/// heartbeat sweep; [`classify_exit`] is for post-mortem.
#[must_use]
pub fn classify_hung(
    last_heartbeat: Option<DateTime<Utc>>,
    hung_threshold: Duration,
    now: DateTime<Utc>,
) -> Option<WorkerExitReason> {
    let last = last_heartbeat?;
    let threshold = ChronoDuration::from_std(hung_threshold).ok()?;
    if now - last > threshold {
        Some(WorkerExitReason::Hung {
            last_heartbeat: last,
        })
    } else {
        None
    }
}

/// Apply the rolling-window restart-budget rules.
///
/// `now` is parameterised so tests can drive the arithmetic
/// without sleeping; production callers pass `Utc::now()`.
pub fn decide_restart(
    coord: &mut OomRestartCoordinator,
    exit: &WorkerExitReason,
    now: DateTime<Utc>,
) -> RestartDecision {
    if !exit.is_actionable() {
        return RestartDecision::IgnoreCleanExit;
    }
    let attempts_used = coord.attempts_in_window(now);
    if attempts_used >= coord.config.max_restarts {
        return RestartDecision::GiveUp {
            salvage_note: build_salvage_note(coord, exit),
        };
    }
    let attempt = attempts_used.saturating_add(1);
    let backoff = backoff_for_attempt(&coord.config, attempt);
    coord.record_attempt(now, exit.clone());
    RestartDecision::Restart {
        after_backoff: backoff,
        attempt,
    }
}

/// Build the JSON payload for the `worker.oom_restart` audit
/// row. Supervisors call this and pass the result to
/// `AuditSink::write(None, AuditKind::WorkerOomRestart,
/// payload, None)`. Payload shape:
/// `{worker_id, exit_reason, decision, attempt_count,
/// recorded_at}`.
///
/// Returning the payload (instead of writing directly) keeps
/// `argo-runtime` free of an `argo-audit` dependency, which
/// would otherwise close a cycle via `argo-store →
/// argo-runtime`. The smoke test in
/// `crates/argo-cli/tests/worker_restart_smoke.rs` exercises
/// the full path through a live `AuditSink`.
#[must_use]
pub fn build_audit_payload(
    worker_id: &str,
    exit: &WorkerExitReason,
    decision: &RestartDecision,
    attempt_count: u32,
) -> serde_json::Value {
    serde_json::json!({
        "worker_id": worker_id,
        "exit_reason": exit,
        "decision": decision,
        "attempt_count": attempt_count,
        "recorded_at": Utc::now().to_rfc3339(),
    })
}

fn backoff_for_attempt(config: &OomRestartConfig, attempt: u32) -> Duration {
    if attempt <= 1 {
        return config.base_backoff.min(config.max_backoff);
    }
    // Clamp shift to avoid u32 overflow on pathological inputs.
    let shift = attempt.saturating_sub(1).min(20);
    let factor = 1u32.checked_shl(shift).unwrap_or(u32::MAX);
    let scaled = config
        .base_backoff
        .checked_mul(factor)
        .unwrap_or(config.max_backoff);
    scaled.min(config.max_backoff)
}

fn build_salvage_note(coord: &OomRestartCoordinator, exit: &WorkerExitReason) -> String {
    let reason_label = match exit {
        WorkerExitReason::OomKilled { signal, .. } => format!("OOM (signal {signal})"),
        WorkerExitReason::OtherSignal { signal } => format!("signal {signal}"),
        WorkerExitReason::Hung { last_heartbeat } => {
            format!("hung (last heartbeat {last_heartbeat})")
        }
        WorkerExitReason::Clean { code } => format!("clean (code {code}; should not surface)"),
    };
    let recent: Vec<String> = coord
        .restart_history
        .iter()
        .rev()
        .take(3)
        .map(|(ts, r)| match r {
            WorkerExitReason::OomKilled { .. } => format!("{ts}=oom"),
            WorkerExitReason::OtherSignal { signal } => format!("{ts}=sig{signal}"),
            WorkerExitReason::Hung { .. } => format!("{ts}=hung"),
            WorkerExitReason::Clean { .. } => format!("{ts}=clean"),
        })
        .collect();
    format!(
        "supervisor: {} restarts in window exceeded; last exit={}; recent={}. See \
         docs/skycot/runbook/oom-restart.md for fix recipe.",
        coord.config.max_restarts,
        reason_label,
        recent.join(",")
    )
}

#[cfg(unix)]
fn read_cgroup_oom_evidence(cgroup_path: &str) -> Option<String> {
    let path = std::path::Path::new(cgroup_path).join("memory.events");
    let body = std::fs::read_to_string(&path).ok()?;
    body.lines()
        .find(|l| l.starts_with("oom_kill "))
        .map(std::string::ToString::to_string)
}

#[cfg(not(unix))]
fn read_cgroup_oom_evidence(_cgroup_path: &str) -> Option<String> {
    None
}

#[cfg(test)]
#[path = "oom_restart_tests.rs"]
mod oom_restart_tests;
