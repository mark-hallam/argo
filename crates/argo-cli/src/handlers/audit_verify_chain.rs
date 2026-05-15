//! `argo audit verify-chain` — operator-facing full-chain integrity check.

use argo_audit::VerifyReport;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct VerifyChainSummary {
    pub(crate) head_seq: i64,
    pub(crate) verified: bool,
    pub(crate) break_at: Option<i64>,
    pub(crate) rows: usize,
    pub(crate) breaks: usize,
    pub(crate) verified_at: String,
}

pub(crate) async fn run(store: &Arc<SessionStore>, json: bool) -> ArgoResult<()> {
    let report = argo_audit::verify_all(store).await?;
    let head_seq = head_seq(store).await?;
    let summary = summarize(&report, head_seq);
    if json {
        let serialized = serde_json::to_string(&summary)
            .map_err(|e| ArgoError::Provider(format!("serialize verify-chain: {e}")))?;
        println!("{serialized}");
    } else if summary.verified {
        println!(
            "audit chain verified: head_seq={} rows={} breaks=0 verified_at={}",
            summary.head_seq, summary.rows, summary.verified_at
        );
    } else {
        println!(
            "audit chain broken: head_seq={} rows={} breaks={} break_at={}",
            summary.head_seq,
            summary.rows,
            summary.breaks,
            summary.break_at.unwrap_or_default()
        );
        for b in &report.breaks {
            println!("  seq {} kind={} ({})", b.seq, b.kind, b.reason);
        }
    }
    if summary.verified {
        Ok(())
    } else {
        Err(ArgoError::Provider(
            "audit chain verification failed".into(),
        ))
    }
}

fn summarize(report: &VerifyReport, head_seq: i64) -> VerifyChainSummary {
    VerifyChainSummary {
        head_seq,
        verified: report.ok,
        break_at: report.breaks.iter().map(|b| b.seq).min(),
        rows: report.rows,
        breaks: report.breaks.len(),
        verified_at: chrono::Utc::now().to_rfc3339(),
    }
}

async fn head_seq(store: &SessionStore) -> ArgoResult<i64> {
    sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(seq), 0) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("audit verify-chain head seq: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_audit::ChainBreak;

    #[test]
    fn summary_reports_clean_chain() {
        let report = VerifyReport {
            rows: 2,
            ok: true,
            ..Default::default()
        };

        let summary = summarize(&report, 7);

        assert_eq!(summary.head_seq, 7);
        assert!(summary.verified);
        assert_eq!(summary.break_at, None);
        assert_eq!(summary.rows, 2);
        assert_eq!(summary.breaks, 0);
    }

    #[test]
    fn summary_reports_first_synthetic_break() {
        let report = VerifyReport {
            rows: 4,
            ok: false,
            breaks: vec![
                ChainBreak {
                    seq: 9,
                    kind: "tool_call".into(),
                    reason: "later".into(),
                },
                ChainBreak {
                    seq: 3,
                    kind: "turn_start".into(),
                    reason: "first".into(),
                },
            ],
            ..Default::default()
        };

        let summary = summarize(&report, 9);

        assert!(!summary.verified);
        assert_eq!(summary.break_at, Some(3));
        assert_eq!(summary.breaks, 2);
    }

    #[test]
    fn summary_reports_empty_chain() {
        let report = VerifyReport {
            rows: 0,
            ok: true,
            ..Default::default()
        };

        let summary = summarize(&report, 0);

        assert!(summary.verified);
        assert_eq!(summary.head_seq, 0);
        assert_eq!(summary.rows, 0);
        assert_eq!(summary.break_at, None);
    }
}
