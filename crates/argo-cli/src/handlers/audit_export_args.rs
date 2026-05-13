//! Input parsing & filter normalization for `argo audit export`.
//!
//! Lifted out of `audit_export.rs` so that file stays under the
//! 500-LOC HARD RULE 6 ceiling. Owns the public `ExportArgs` input,
//! the internal `NormalizedExport` (validated form), the
//! `ExportFormat` enum, and the row-filter `matches` predicate. Pure
//! parsing — no I/O, no async.
use argo_audit::ExportedRow;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, NaiveDate, Utc};

pub struct ExportArgs {
    pub format: String,
    pub bundle_out: Option<String>,
    pub worker_id: Option<String>,
    pub tenant: Option<String>,
    pub session: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub since_minutes: Option<i64>,
    pub gdpr_redact_emails: bool,
    pub json: bool,
}

#[derive(Debug)]
pub(super) struct NormalizedExport {
    pub(super) format: ExportFormat,
    pub(super) bundle_out: Option<String>,
    pub(super) worker_id: Option<String>,
    pub(super) tenant: Option<String>,
    pub(super) session: Option<String>,
    pub(super) session_hex: Option<String>,
    pub(super) since: Option<String>,
    pub(super) until: Option<String>,
    pub(super) since_micros: Option<i64>,
    pub(super) until_micros: Option<i64>,
    pub(super) gdpr_redact_emails: bool,
    pub(super) json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum ExportFormat {
    Ndjson,
    Csv,
    Tarball,
}

impl NormalizedExport {
    pub(super) fn new(args: ExportArgs) -> ArgoResult<Self> {
        let format = parse_format(&args.format)?;
        let session_hex = match args.session.as_deref() {
            Some(raw) => {
                let uuid = uuid::Uuid::parse_str(raw)
                    .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw}': {e}")))?;
                Some(hex::encode(uuid.as_bytes()))
            }
            None => None,
        };
        let since_micros = if let Some(raw) = args.since.as_deref() {
            Some(parse_bound(raw, false, "--since")?)
        } else {
            args.since_minutes
                .map(|minutes| (Utc::now() - chrono::Duration::minutes(minutes)).timestamp_micros())
        };
        let until_micros = args
            .until
            .as_deref()
            .map(|raw| parse_bound(raw, true, "--until"))
            .transpose()?;
        if let (Some(since), Some(until)) = (since_micros, until_micros) {
            if since > until {
                return Err(ArgoError::Provider(
                    "invalid audit export time range: --since is after --until".into(),
                ));
            }
        }
        Ok(Self {
            format,
            bundle_out: args.bundle_out,
            worker_id: args.worker_id,
            tenant: args.tenant,
            session: args.session,
            session_hex,
            since: args.since,
            until: args.until,
            since_micros,
            until_micros,
            gdpr_redact_emails: args.gdpr_redact_emails,
            json: args.json,
        })
    }

    pub(super) fn matches(&self, row: &ExportedRow) -> bool {
        if self
            .since_micros
            .is_some_and(|since| row.created_at_micros < since)
        {
            return false;
        }
        if self
            .until_micros
            .is_some_and(|until| row.created_at_micros > until)
        {
            return false;
        }
        if self
            .session_hex
            .as_deref()
            .is_some_and(|sid| row.session_id.as_deref() != Some(sid))
        {
            return false;
        }
        if self
            .worker_id
            .as_deref()
            .is_some_and(|worker| !payload_matches(&row.payload, worker, WORKER_POINTERS))
        {
            return false;
        }
        if self
            .tenant
            .as_deref()
            .is_some_and(|tenant| !payload_matches(&row.payload, tenant, TENANT_POINTERS))
        {
            return false;
        }
        true
    }
}

const WORKER_POINTERS: &[&str] = &["/worker_id", "/from_worker_id", "/to/worker_id"];
const TENANT_POINTERS: &[&str] = &["/tenant_id"];

fn payload_matches(payload: &serde_json::Value, expected: &str, pointers: &[&str]) -> bool {
    pointers
        .iter()
        .any(|ptr| payload.pointer(ptr).and_then(|v| v.as_str()) == Some(expected))
}

fn parse_format(raw: &str) -> ArgoResult<ExportFormat> {
    match raw {
        "ndjson" => Ok(ExportFormat::Ndjson),
        "csv" => Ok(ExportFormat::Csv),
        "tarball" => Ok(ExportFormat::Tarball),
        other => Err(ArgoError::Provider(format!(
            "unsupported audit export format '{other}' (expected ndjson, csv, or tarball)"
        ))),
    }
}

fn parse_bound(raw: &str, until: bool, flag: &str) -> ArgoResult<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.timestamp_micros());
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|e| {
        ArgoError::Provider(format!(
            "invalid {flag} date '{raw}': {e} (expected YYYY-MM-DD or RFC3339)"
        ))
    })?;
    let time = if until {
        date.and_hms_micro_opt(23, 59, 59, 999_999)
    } else {
        date.and_hms_micro_opt(0, 0, 0, 0)
    }
    .ok_or_else(|| ArgoError::Schema(format!("invalid {flag} date '{raw}'")))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(time, Utc).timestamp_micros())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bound_accepts_date_and_rfc3339() {
        assert_eq!(
            parse_bound("2026-05-06", false, "--since").unwrap(),
            1_778_025_600_000_000
        );
        assert_eq!(
            parse_bound("2026-05-06T01:02:03Z", true, "--until").unwrap(),
            1_778_029_323_000_000
        );
    }
}
