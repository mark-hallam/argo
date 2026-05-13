//! `argo compliance soc2` — SOC2-aligned audit trail report.
//!
//! Reads the existing `audit_record` chain in a `[from, to)` window
//! and emits a categorised summary suitable for handing to an
//! external auditor. Read-only over the audit table.
use argo_audit::{export_ndjson, ExportedRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Soc2Category {
    Access,
    ChangeMgmt,
    DataMgmt,
    Security,
    Other,
}

impl Soc2Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Access => "Access",
            Self::ChangeMgmt => "ChangeMgmt",
            Self::DataMgmt => "DataMgmt",
            Self::Security => "Security",
            Self::Other => "Other",
        }
    }
}

const CATEGORY_ORDER: &[Soc2Category] = &[
    Soc2Category::Access,
    Soc2Category::ChangeMgmt,
    Soc2Category::DataMgmt,
    Soc2Category::Security,
    Soc2Category::Other,
];

/// Static map of audit `kind` → SOC2 control category.
///
/// Unrecognised kinds fall into [`Soc2Category::Other`] so they
/// remain visible in the report rather than being silently lost.
pub static ACTION_CATEGORY: &[(&str, Soc2Category)] = &[
    ("auth.login", Soc2Category::Access),
    ("auth.logout", Soc2Category::Access),
    ("tenant.role_change", Soc2Category::Access),
    ("worker.deploy", Soc2Category::ChangeMgmt),
    ("manifest.publish", Soc2Category::ChangeMgmt),
    ("executor.update", Soc2Category::ChangeMgmt),
    ("compliance.export", Soc2Category::DataMgmt),
    ("compliance.delete", Soc2Category::DataMgmt),
    ("data.import", Soc2Category::DataMgmt),
    ("policy.update", Soc2Category::Security),
    ("secret.rotate", Soc2Category::Security),
    ("audit.verify_chain", Soc2Category::Security),
];

pub fn classify(action: &str) -> Soc2Category {
    ACTION_CATEGORY
        .iter()
        .find_map(|(name, cat)| (*name == action).then_some(*cat))
        .unwrap_or(Soc2Category::Other)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    Csv,
    Markdown,
}

impl ReportFormat {
    fn parse(raw: &str) -> ArgoResult<Self> {
        match raw {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            "markdown" => Ok(Self::Markdown),
            other => Err(ArgoError::Provider(format!(
                "unsupported --format '{other}' (expected json, csv, or markdown)"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComplianceSoc2Args {
    pub from: String,
    pub to: String,
    pub actor: Option<String>,
    pub action: Option<String>,
    pub tenant: Option<String>,
    pub format: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportEvent {
    pub seq: i64,
    pub id: String,
    pub ts: String,
    pub actor: Option<String>,
    pub tenant: Option<String>,
    pub action: String,
    pub target: Option<String>,
    pub session_id: Option<String>,
    pub category: Soc2Category,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CategoryAggregate {
    pub count: u64,
    pub first: Option<String>,
    pub last: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Soc2Report {
    pub window: ReportWindow,
    pub filters: ReportFilters,
    pub categories: BTreeMap<String, CategoryAggregate>,
    pub events: Vec<ReportEvent>,
}

#[derive(Debug, Serialize)]
pub struct ReportWindow {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct ReportFilters {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub tenant: Option<String>,
}

pub async fn run(args: ComplianceSoc2Args, store: &SessionStore) -> ArgoResult<i32> {
    let format = ReportFormat::parse(&args.format)?;
    let report = build_report(args, store).await?;
    print!("{}", render_with_format(&report, format)?);
    Ok(0)
}

pub(crate) async fn build_report(
    args: ComplianceSoc2Args,
    store: &SessionStore,
) -> ArgoResult<Soc2Report> {
    let from = parse_rfc3339(&args.from, "--from")?;
    let to = parse_rfc3339(&args.to, "--to")?;
    if from > to {
        return Err(ArgoError::Provider(
            "--from is after --to (window would be empty)".into(),
        ));
    }
    let mut events: Vec<ReportEvent> = Vec::new();
    export_ndjson(store, |row| {
        if let Some(ev) = filter_row(row, from, to, &args) {
            events.push(ev);
        }
        Ok(())
    })
    .await?;
    Ok(Soc2Report {
        window: ReportWindow {
            from: args.from,
            to: args.to,
        },
        filters: ReportFilters {
            actor: args.actor,
            action: args.action,
            tenant: args.tenant,
        },
        categories: aggregate(&events),
        events,
    })
}

fn filter_row(
    row: &ExportedRow,
    from_micros: i64,
    to_micros: i64,
    args: &ComplianceSoc2Args,
) -> Option<ReportEvent> {
    if row.created_at_micros < from_micros || row.created_at_micros >= to_micros {
        return None;
    }
    if args.action.as_deref().is_some_and(|a| row.kind != a) {
        return None;
    }
    let actor = pluck(&row.payload, ACTOR_POINTERS);
    if args
        .actor
        .as_deref()
        .is_some_and(|want| actor.as_deref() != Some(want))
    {
        return None;
    }
    let tenant = pluck(&row.payload, TENANT_POINTERS);
    if args
        .tenant
        .as_deref()
        .is_some_and(|want| tenant.as_deref() != Some(want))
    {
        return None;
    }
    Some(ReportEvent {
        seq: row.seq,
        id: row.id.clone(),
        ts: micros_to_rfc3339(row.created_at_micros),
        actor,
        tenant,
        action: row.kind.clone(),
        target: pluck(&row.payload, TARGET_POINTERS),
        session_id: row.session_id.clone(),
        category: classify(&row.kind),
    })
}

const ACTOR_POINTERS: &[&str] = &[
    "/actor",
    "/actor_id",
    "/user_id",
    "/worker_id",
    "/from_worker_id",
];
const TENANT_POINTERS: &[&str] = &["/tenant_id", "/tenant"];
const TARGET_POINTERS: &[&str] = &[
    "/target",
    "/target_id",
    "/resource",
    "/resource_id",
    "/subject",
];

fn pluck(payload: &serde_json::Value, pointers: &[&str]) -> Option<String> {
    pointers
        .iter()
        .find_map(|ptr| payload.pointer(ptr).and_then(|v| v.as_str()))
        .map(str::to_string)
}

fn aggregate(events: &[ReportEvent]) -> BTreeMap<String, CategoryAggregate> {
    let mut out: BTreeMap<String, CategoryAggregate> = CATEGORY_ORDER
        .iter()
        .map(|c| (c.label().to_string(), CategoryAggregate::default()))
        .collect();
    for ev in events {
        let entry = out.entry(ev.category.label().to_string()).or_default();
        entry.count += 1;
        if entry.first.is_none() {
            entry.first = Some(ev.ts.clone());
        }
        entry.last = Some(ev.ts.clone());
    }
    out
}

pub(crate) fn render_with_format(report: &Soc2Report, format: ReportFormat) -> ArgoResult<String> {
    match format {
        ReportFormat::Json => {
            let mut out = serde_json::to_string_pretty(report)
                .map_err(|e| ArgoError::Provider(format!("serialize soc2 report: {e}")))?;
            out.push('\n');
            Ok(out)
        }
        ReportFormat::Csv => Ok(render_csv(report)),
        ReportFormat::Markdown => Ok(render_markdown(report)),
    }
}

fn render_csv(report: &Soc2Report) -> String {
    let mut out = String::from("seq,id,ts,actor,tenant,action,category,target,session_id\n");
    for ev in &report.events {
        let fields = [
            ev.seq.to_string(),
            ev.id.clone(),
            ev.ts.clone(),
            ev.actor.clone().unwrap_or_default(),
            ev.tenant.clone().unwrap_or_default(),
            ev.action.clone(),
            ev.category.label().to_string(),
            ev.target.clone().unwrap_or_default(),
            ev.session_id.clone().unwrap_or_default(),
        ];
        out.push_str(
            &fields
                .iter()
                .map(|f| csv_escape(f))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

fn render_markdown(report: &Soc2Report) -> String {
    let mut out = String::new();
    out.push_str("# SOC2 audit trail report\n\n");
    out.push_str(&format!(
        "Window: `{}` → `{}` (inclusive on `from`, exclusive on `to`)\n\n",
        report.window.from, report.window.to,
    ));
    out.push_str(&format!(
        "Filters: actor={}, action={}, tenant={}\n\n",
        opt(&report.filters.actor),
        opt(&report.filters.action),
        opt(&report.filters.tenant),
    ));
    for cat in CATEGORY_ORDER {
        let label = cat.label();
        let agg = report.categories.get(label).cloned().unwrap_or_default();
        out.push_str(&format!("## {label}\n\n"));
        out.push_str(&format!(
            "Count: **{}**, first: {}, last: {}\n\n",
            agg.count,
            opt(&agg.first),
            opt(&agg.last),
        ));
    }
    out.push_str("## Events\n\n");
    out.push_str("| seq | ts | actor | action | category | target |\n");
    out.push_str("|----:|----|-------|--------|----------|--------|\n");
    for ev in &report.events {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            ev.seq,
            ev.ts,
            opt(&ev.actor),
            ev.action,
            ev.category.label(),
            opt(&ev.target),
        ));
    }
    out
}

fn opt(v: &Option<String>) -> String {
    v.clone().unwrap_or_else(|| "—".to_string())
}

fn csv_escape(raw: &str) -> String {
    if raw.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw.to_string()
    }
}

fn parse_rfc3339(raw: &str, flag: &str) -> ArgoResult<i64> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.timestamp_micros())
        .map_err(|e| ArgoError::Schema(format!("invalid {flag} '{raw}': {e} (expected RFC3339)")))
}

fn micros_to_rfc3339(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .unwrap_or_else(Utc::now)
        .to_rfc3339()
}

#[cfg(test)]
#[path = "soc2_tests.rs"]
mod tests;
