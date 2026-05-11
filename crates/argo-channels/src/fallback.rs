//! Approval-channel fallback chain types and persistence.
//!
//! Implements the `[notify_channels.fallback_chain]` config block
//! and the `DeliveryAttempt` / `DeliveryReport` records that
//! `worker_actions_notify_fallback.rs` produces and that
//! `argo doctor channels` reads.
//!
//! Persistence is JSONL on disk (mirrors the
//! `crates/argo-channels/src/webinbox.rs` pattern). The orchestrator
//! lives one layer up in `argo-cli`; this module only owns the
//! types and the append/read helpers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default per-step timeout (matches the brief's example).
pub const DEFAULT_PER_STEP_TIMEOUT_SECS: u64 = 30;

/// Default JSONL path under `<cwd>/`. Overridable via
/// `ARGO_DELIVERY_REPORTS_PATH`.
pub const DEFAULT_REPORTS_RELPATH: &str = ".skycot/delivery-reports.jsonl";

/// `[notify_channels.fallback_chain]` parser shape. The runtime
/// type [`FallbackChain`] is the validated form.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FallbackChainManifest {
    /// Channel kinds tried in order on each notification.
    pub order: Vec<String>,
    /// Per-step send timeout in seconds.
    #[serde(default = "default_per_step_timeout_secs")]
    pub per_step_timeout_secs: u64,
}

const fn default_per_step_timeout_secs() -> u64 {
    DEFAULT_PER_STEP_TIMEOUT_SECS
}

impl FallbackChainManifest {
    /// Convert the parsed manifest into the runtime [`FallbackChain`].
    /// Empty `order` returns `None` — an empty chain is treated as
    /// "no fallback configured".
    #[must_use]
    pub fn into_chain(self) -> Option<FallbackChain> {
        if self.order.is_empty() {
            None
        } else {
            Some(FallbackChain {
                order: self.order,
                per_step_timeout_secs: self.per_step_timeout_secs.max(1),
            })
        }
    }
}

/// Validated fallback chain used at dispatch time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FallbackChain {
    /// Channel kinds in attempt order. First success wins.
    pub order: Vec<String>,
    /// Per-step send timeout in seconds (clamped to ≥1).
    pub per_step_timeout_secs: u64,
}

impl FallbackChain {
    /// Build a chain from a slice of channel kinds.
    #[must_use]
    pub fn from_order(order: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            order: order.into_iter().map(Into::into).collect(),
            per_step_timeout_secs: DEFAULT_PER_STEP_TIMEOUT_SECS,
        }
    }

    /// Pilot integration with the per-operator preferences resolver.
    ///
    /// Given a `preferred` channel resolved upstream by
    /// [`crate::operator_preferences::OperatorPreferencesResolver`],
    /// return a new chain with the preferred channel promoted to
    /// the head. If the preferred channel is missing, prepend it.
    /// If `preferred` is `None`, the chain is returned unchanged —
    /// operators with no `[operator.<id>.preferences]` block see
    /// today's behavior. Channel-specific config silos remain
    /// authoritative for in-channel rendering; this method only
    /// reorders the attempt sequence.
    #[must_use]
    pub fn with_operator_preferred(mut self, preferred: Option<&str>) -> Self {
        let Some(preferred) = preferred else {
            return self;
        };
        if self.order.first().is_some_and(|c| c == preferred) {
            return self;
        }
        self.order.retain(|c| c != preferred);
        self.order.insert(0, preferred.to_string());
        self
    }
}

/// Outcome of a single channel attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AttemptOutcome {
    /// Channel accepted the notification.
    Success,
    /// Per-step timeout fired before the channel returned.
    Timeout,
    /// Channel returned an error (transport, rate-limit, parse, ...).
    /// The caller passes through the rate-limit / retry-after detail
    /// in the `error` string when the channel surfaces one.
    Failure {
        /// Compact error string from the channel send result.
        error: String,
    },
    /// The chain referenced a channel kind that the operator did not
    /// configure in `.skycot/notify.toml`. The attempt is recorded so
    /// the operator can see the misconfiguration in `argo doctor`.
    Unconfigured,
}

impl AttemptOutcome {
    /// `true` when the chain should try the next channel after this
    /// outcome. Only [`AttemptOutcome::Success`] terminates.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, AttemptOutcome::Success)
    }
}

/// One channel attempt inside a [`DeliveryReport`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryAttempt {
    /// Channel kind tried (matches `FallbackChain::order` entry).
    pub channel: String,
    /// Wall-clock time the attempt started.
    pub started_at: DateTime<Utc>,
    /// Outcome.
    pub outcome: AttemptOutcome,
}

/// Full record of one fallback dispatch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryReport {
    /// Approval action id this delivery is for.
    pub action_id: String,
    /// Wall-clock time the dispatch started.
    pub started_at: DateTime<Utc>,
    /// Wall-clock time the dispatch returned.
    pub finished_at: DateTime<Utc>,
    /// First channel that succeeded, if any.
    pub succeeded_via: Option<String>,
    /// Per-channel attempts in chain order.
    pub attempts: Vec<DeliveryAttempt>,
}

impl DeliveryReport {
    /// `true` when at least one attempt succeeded.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.succeeded_via.is_some()
    }

    /// Compact one-line summary for `argo doctor channels` output.
    #[must_use]
    pub fn summary(&self) -> String {
        let chain: Vec<String> = self
            .attempts
            .iter()
            .map(|a| {
                let label = match &a.outcome {
                    AttemptOutcome::Success => "ok",
                    AttemptOutcome::Timeout => "timeout",
                    AttemptOutcome::Failure { .. } => "fail",
                    AttemptOutcome::Unconfigured => "unconfigured",
                };
                format!("{}={label}", a.channel)
            })
            .collect();
        let status = if self.ok() {
            format!(
                "ok via {}",
                self.succeeded_via.as_deref().unwrap_or("(unknown)")
            )
        } else {
            "all-failed".into()
        };
        format!(
            "{}  action={}  {}",
            self.started_at.to_rfc3339(),
            self.action_id,
            format_args!("{status} [{}]", chain.join(","))
        )
    }
}

/// Resolve the JSONL path for delivery reports.
///
/// Order: `ARGO_DELIVERY_REPORTS_PATH` env override, else
/// `<cwd>/<DEFAULT_REPORTS_RELPATH>`. The file is created on first
/// append.
#[must_use]
pub fn reports_path(cwd: &Path) -> PathBuf {
    if let Ok(p) = std::env::var("ARGO_DELIVERY_REPORTS_PATH") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    cwd.join(DEFAULT_REPORTS_RELPATH)
}

/// Append a [`DeliveryReport`] to the JSONL file at `path`. The
/// parent directory is created on demand. Best-effort: errors are
/// surfaced but the caller is expected to log and move on (the
/// audit-export path is the durable record).
pub fn append_report(path: &Path, report: &DeliveryReport) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

/// Read up to the last `n` reports from the JSONL file at `path`.
/// Newest entry first. Missing file → empty vec. Malformed lines
/// are skipped silently — the file is a best-effort audit log, not
/// a source of truth.
#[must_use]
pub fn read_recent_reports(path: &Path, n: usize) -> Vec<DeliveryReport> {
    if n == 0 {
        return Vec::new();
    }
    let body = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<DeliveryReport> = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<DeliveryReport>(l).ok())
        .collect();
    out.reverse();
    out.truncate(n);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn report(action: &str, succeeded: Option<&str>) -> DeliveryReport {
        let now = Utc::now();
        DeliveryReport {
            action_id: action.into(),
            started_at: now,
            finished_at: now,
            succeeded_via: succeeded.map(str::to_string),
            attempts: vec![DeliveryAttempt {
                channel: "slack".into(),
                started_at: now,
                outcome: AttemptOutcome::Success,
            }],
        }
    }

    #[test]
    fn manifest_into_chain_drops_empty() {
        let m = FallbackChainManifest {
            order: vec![],
            per_step_timeout_secs: 30,
        };
        assert!(m.into_chain().is_none());
    }

    #[test]
    fn manifest_into_chain_clamps_timeout() {
        let m = FallbackChainManifest {
            order: vec!["slack".into()],
            per_step_timeout_secs: 0,
        };
        let chain = m.into_chain().expect("chain");
        assert_eq!(chain.per_step_timeout_secs, 1);
    }

    #[test]
    fn manifest_round_trip_toml() {
        let src = r#"
            order = ["slack", "email", "webinbox"]
            per_step_timeout_secs = 12
        "#;
        let m: FallbackChainManifest = toml::from_str(src).expect("parse");
        assert_eq!(m.order, vec!["slack", "email", "webinbox"]);
        assert_eq!(m.per_step_timeout_secs, 12);
        let chain = m.into_chain().expect("chain");
        assert_eq!(chain.order.len(), 3);
    }

    #[test]
    fn manifest_default_timeout_used_when_omitted() {
        let src = r#"order = ["slack"]"#;
        let m: FallbackChainManifest = toml::from_str(src).expect("parse");
        assert_eq!(m.per_step_timeout_secs, DEFAULT_PER_STEP_TIMEOUT_SECS);
    }

    #[test]
    fn report_summary_includes_outcomes() {
        let now = Utc::now();
        let r = DeliveryReport {
            action_id: "act-1".into(),
            started_at: now,
            finished_at: now,
            succeeded_via: Some("email".into()),
            attempts: vec![
                DeliveryAttempt {
                    channel: "slack".into(),
                    started_at: now,
                    outcome: AttemptOutcome::Failure {
                        error: "401".into(),
                    },
                },
                DeliveryAttempt {
                    channel: "email".into(),
                    started_at: now,
                    outcome: AttemptOutcome::Success,
                },
            ],
        };
        let s = r.summary();
        assert!(s.contains("act-1"));
        assert!(s.contains("ok via email"));
        assert!(s.contains("slack=fail"));
        assert!(s.contains("email=ok"));
    }

    #[test]
    fn append_then_read_round_trip() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("d.jsonl");
        append_report(&path, &report("a-1", Some("slack"))).unwrap();
        append_report(&path, &report("a-2", None)).unwrap();
        append_report(&path, &report("a-3", Some("email"))).unwrap();
        let recent = read_recent_reports(&path, 10);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].action_id, "a-3", "newest first");
        assert_eq!(recent[2].action_id, "a-1");
    }

    #[test]
    fn read_recent_truncates_to_n() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("d.jsonl");
        for i in 0..5 {
            append_report(&path, &report(&format!("a-{i}"), Some("slack"))).unwrap();
        }
        let recent = read_recent_reports(&path, 2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].action_id, "a-4");
        assert_eq!(recent[1].action_id, "a-3");
    }

    #[test]
    fn read_recent_skips_garbage_lines() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("d.jsonl");
        append_report(&path, &report("a-1", Some("slack"))).unwrap();
        // Append a junk line and another good line.
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        std::fs::write(
            &path,
            std::fs::read_to_string(&path).unwrap() + "{not-json}\n",
        )
        .unwrap();
        append_report(&path, &report("a-2", None)).unwrap();
        let recent = read_recent_reports(&path, 10);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn read_recent_missing_file_returns_empty() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("missing.jsonl");
        let recent = read_recent_reports(&path, 5);
        assert!(recent.is_empty());
    }

    #[test]
    fn reports_path_uses_env_override() {
        let tmp = tempdir().expect("tempdir");
        let custom = tmp.path().join("custom.jsonl");
        let key = "ARGO_DELIVERY_REPORTS_PATH";
        let prev = std::env::var(key).ok();
        std::env::set_var(key, &custom);
        let resolved = reports_path(tmp.path());
        assert_eq!(resolved, custom);
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn outcome_is_terminal_only_for_success() {
        assert!(AttemptOutcome::Success.is_terminal());
        assert!(!AttemptOutcome::Timeout.is_terminal());
        assert!(!(AttemptOutcome::Failure { error: "x".into() }).is_terminal());
        assert!(!AttemptOutcome::Unconfigured.is_terminal());
    }
}
