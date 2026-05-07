//! `argo skill distill-now <vertical>` — on-demand cohort
//! distillation via the Dreams API.
//!
//! Reads `TrajectoryInput` rows as JSON from stdin (one array
//! or NDJSON), groups them into cohorts, and for each cohort
//! that meets [`argo_skill::MIN_COHORT_SIZE`] fires the Dreams
//! distillation call (per-cohort cost charged against the
//! worker-shared per-day cap from
//! `argo_memory::dreams_backend::DailyCapTracker`).
//!
//! The producer of trajectory rows is out of v1 scope — the
//! brief sketches a nightly tick that walks recent CU runs;
//! this handler is the operator-driven entry point that lets
//! that data shape be replayed offline (and gives the smoke
//! test a deterministic surface).
//!
//! Brief: `claude-dreams-skill-distillation` (V2.0 Integration B).
use argo_memory::dreams_backend::DailyCapTracker;
use argo_skill::dreams_distillation::{
    distill_cohort, group_into_cohorts, Cohort, DistillationResult,
    SkillDistillationCaller, TrajectoryInput,
};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use chrono::Utc;
use serde::Serialize;
use std::io::{self, Read};

/// Default Dreams endpoint for skill distillation. Overridable
/// via [`ENV_DREAMS_URL`].
pub const DEFAULT_DREAMS_URL: &str = "https://api.anthropic.com/v1/dreams";

/// Env var override for [`DEFAULT_DREAMS_URL`].
pub const ENV_DREAMS_URL: &str = "ANTHROPIC_DREAMS_URL";

/// Default per-call cost estimate when the response omits one.
pub const DEFAULT_COST_USD_MICROS: i64 = 750_000;

/// Per-cohort summary returned in the JSON envelope.
#[derive(Debug, Serialize)]
pub struct CohortSummary {
    /// Vertical the cohort belongs to.
    pub vertical_id: String,
    /// Action kind shared by every member.
    pub action_type: String,
    /// Number of trajectories in the cohort (always
    /// `>= MIN_COHORT_SIZE`).
    pub size: usize,
    /// Whether the Dreams call succeeded.
    pub distilled: bool,
    /// Reason the call was skipped (cap exceeded, transport
    /// error, …) — populated only when `distilled = false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    /// Charged cost in USD micros — present when distilled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<i64>,
}

/// Run the distillation pipeline. Reads `TrajectoryInput` rows
/// from `reader` (a JSON array), groups them into cohorts, and
/// distills each that meets the threshold.
///
/// Daily cap is rooted at `<state_root>/.skycot/dreams/<worker>/`
/// — same prefix as `memory dream-now` so the budget is shared.
/// For v1 the handler uses a single shared bucket per
/// `--worker` argument (default `skill-distillation`); per-worker
/// scope can land once trajectory data carries worker_id.
///
/// # Errors
/// - JSON parse failure
/// - I/O failures on the daily-cap state file (other than
///   cap-exceeded, which is reported per cohort)
pub async fn run(
    worker: &str,
    state_root: &std::path::Path,
    caller: &dyn SkillDistillationCaller,
    reader: &mut dyn Read,
    json: bool,
) -> ArgoResult<()> {
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|e| ArgoError::Provider(format!("distill-now read stdin: {e}")))?;
    let trajectories: Vec<TrajectoryInput> = if buf.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&buf)
            .map_err(|e| ArgoError::Provider(format!("distill-now parse: {e}")))?
    };

    let cohorts = group_into_cohorts(trajectories);
    let cap = DailyCapTracker::new(state_root, worker);
    let today = Utc::now().date_naive();

    let mut summaries: Vec<CohortSummary> = Vec::new();
    for cohort in &cohorts {
        match try_distill(cohort, caller, &cap, today).await {
            Ok(result) => summaries.push(CohortSummary {
                vertical_id: cohort.vertical_id.clone(),
                action_type: cohort.action_type.clone(),
                size: cohort.members.len(),
                distilled: true,
                skipped_reason: None,
                cost_micros: Some(result.cost_micros),
            }),
            Err(reason) => summaries.push(CohortSummary {
                vertical_id: cohort.vertical_id.clone(),
                action_type: cohort.action_type.clone(),
                size: cohort.members.len(),
                distilled: false,
                skipped_reason: Some(reason),
                cost_micros: None,
            }),
        }
    }

    let total_distilled = summaries.iter().filter(|s| s.distilled).count();
    let total_skipped = summaries.len() - total_distilled;
    if json {
        let payload = serde_json::json!({
            "worker": worker,
            "cohorts_total": summaries.len(),
            "distilled": total_distilled,
            "skipped": total_skipped,
            "details": summaries,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload)
                .map_err(|e| ArgoError::Provider(format!("distill-now json: {e}")))?
        );
    } else {
        println!(
            "distill-now: worker={worker} cohorts={total} distilled={d} skipped={s}",
            total = summaries.len(),
            d = total_distilled,
            s = total_skipped,
        );
        for c in &summaries {
            let tag = if c.distilled { "ok" } else { "skip" };
            let extra = c
                .skipped_reason
                .as_deref()
                .map(|r| format!(" — {r}"))
                .unwrap_or_default();
            println!(
                "  [{tag}] {v}/{a} size={n}{extra}",
                v = c.vertical_id,
                a = c.action_type,
                n = c.size,
            );
        }
    }
    Ok(())
}

async fn try_distill(
    cohort: &Cohort,
    caller: &dyn SkillDistillationCaller,
    cap: &DailyCapTracker,
    day: chrono::NaiveDate,
) -> Result<DistillationResult, String> {
    let (_candidate, result) = distill_cohort(cohort, caller)
        .await
        .map_err(|e| format!("distill: {e}"))?;
    cap.check_and_charge(day, result.cost_micros)
        .map_err(|e| format!("daily cap: {e}"))?;
    Ok(result)
}

/// HTTP-backed [`SkillDistillationCaller`] using
/// `ANTHROPIC_API_KEY`. Endpoint is overridable via
/// [`ENV_DREAMS_URL`] (useful for fixtures or proxies).
#[derive(Debug, Clone)]
pub struct EnvSkillDistillationCaller {
    api_key: String,
    url: String,
    client: reqwest::Client,
}

impl EnvSkillDistillationCaller {
    /// Build a caller from environment.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] when `ANTHROPIC_API_KEY`
    /// is missing.
    pub fn from_env() -> ArgoResult<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| ArgoError::Provider("ANTHROPIC_API_KEY not set".into()))?;
        let url = std::env::var(ENV_DREAMS_URL).unwrap_or_else(|_| DEFAULT_DREAMS_URL.into());
        Ok(Self {
            api_key,
            url,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl SkillDistillationCaller for EnvSkillDistillationCaller {
    async fn distill(&self, cohort: &Cohort) -> ArgoResult<DistillationResult> {
        let body = build_request_body(cohort);
        let resp = self
            .client
            .post(&self.url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("distill send: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(ArgoError::Provider(format!(
                "distill http {status}: {txt}"
            )));
        }
        let parsed: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("distill decode: {e}")))?;
        parse_response(&parsed)
    }
}

fn build_request_body(cohort: &Cohort) -> serde_json::Value {
    serde_json::json!({
        "model": "claude-dreams-1",
        "scope": "skill_distillation",
        "vertical_id": cohort.vertical_id,
        "action_type": cohort.action_type,
        "trajectories": cohort
            .members
            .iter()
            .map(|m| serde_json::json!({
                "session_id": m.session_id,
                "steps": m.trajectory,
                "final_text": m.final_text,
                "success_metric": m.success_metric,
            }))
            .collect::<Vec<_>>(),
    })
}

fn parse_response(parsed: &serde_json::Value) -> ArgoResult<DistillationResult> {
    let body = parsed
        .pointer("/dream/body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ArgoError::Provider("distill response missing /dream/body".into()))?
        .to_string();
    let cost_micros = parsed
        .pointer("/dream/cost_usd_micros")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_COST_USD_MICROS);
    let model = parsed
        .pointer("/dream/model")
        .and_then(|v| v.as_str())
        .unwrap_or("claude-dreams-1")
        .to_string();
    Ok(DistillationResult {
        body,
        model,
        cost_micros,
    })
}

/// CLI entry point — wires stdin to [`run`] with the
/// production HTTP caller (constructed lazily so a no-cohort
/// invocation does not require `ANTHROPIC_API_KEY`).
pub async fn run_cli(worker: &str, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = crate::home::resolve_home(&cwd)?;
    crate::home::ensure_dir(&home)?;
    // Read stdin, parse, group. If no cohorts qualify there's
    // no need to construct the HTTP caller — keeps the CLI
    // testable offline + makes the smoke deterministic.
    let mut stdin_buf = String::new();
    io::stdin()
        .read_to_string(&mut stdin_buf)
        .map_err(|e| ArgoError::Provider(format!("distill-now read stdin: {e}")))?;
    let trajectories: Vec<TrajectoryInput> = if stdin_buf.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&stdin_buf)
            .map_err(|e| ArgoError::Provider(format!("distill-now parse: {e}")))?
    };
    let cohorts = group_into_cohorts(trajectories);
    if cohorts.is_empty() {
        if json {
            println!("{}", serde_json::json!({
                "worker": worker,
                "cohorts_total": 0,
                "distilled": 0,
                "skipped": 0,
                "details": Vec::<CohortSummary>::new(),
            }));
        } else {
            println!("distill-now: worker={worker} cohorts=0 distilled=0 skipped=0");
        }
        return Ok(());
    }
    let caller = EnvSkillDistillationCaller::from_env()?;
    // Re-serialize so the inner `run` can re-parse from a
    // single source-of-truth path.
    let payload = serde_json::to_string(
        &cohorts
            .iter()
            .flat_map(|c| c.members.clone())
            .collect::<Vec<_>>(),
    )
    .map_err(|e| ArgoError::Provider(format!("distill-now reserialize: {e}")))?;
    let mut cursor = std::io::Cursor::new(payload.into_bytes());
    run(worker, &home, &caller, &mut cursor, json).await
}
