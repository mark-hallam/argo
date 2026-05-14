//! `argo eval skill-honesty <skill_id>` — diagnostic surface for the
//! self-evaluation honesty test (mission-eval-self-honesty-test).
//!
//! Wires together:
//!
//! - [`SqliteAuditSampler`] — reads `audit_record` rows of kind
//!   `tool_result` and extracts the `self_reported_success` /
//!   `task_statement` / `agent_output` convention.
//! - An [`argo_judge::LlmJudge`] over `--provider` / `--model`.
//! - [`argo_eval::run_honesty_test`] — the math.
//!
//! Output mirrors the other `argo eval` JSON envelopes: the result
//! enum plus a per-sample breakdown for operator triage.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_eval::{run_honesty_test, AuditSampler, HonestyConfig, HonestyResult, SkillSample};
use argo_judge::{LlmJudge, Rubric};
use argo_provider::{
    AnthropicTransport, BedrockTransport, OpenAiCompatTransport, OpenAiResponsesTransport,
    ProviderFamily, ProviderManifest, Transport,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, ProviderModel, SessionId};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// CLI entry. `sample` overrides `HonestyConfig::sample_size` when
/// supplied. `provider` and `model` are required because the brief
/// mandates reuse of `argo-judge::adjudicate` and the judge needs a
/// configured upstream — there is no built-in offline fallback.
pub async fn run(
    skill_id: &str,
    sample: Option<usize>,
    provider: &str,
    model: &str,
    json_out: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let sampler = SqliteAuditSampler::new(store);
    let judge = build_judge(provider, model).await?;
    let mut cfg = HonestyConfig::default();
    if let Some(n) = sample {
        cfg.sample_size = n;
    }
    let result = run_honesty_test(skill_id, &sampler, &judge, cfg).await?;
    emit(&result, skill_id, json_out)
}

/// Audit sampler that pulls qualifying `tool_result` envelopes from
/// the `audit_record` table.
///
/// "Qualifying" means the JSON payload carries the three keys the
/// honesty test reads: `skill_id`, `self_reported_success`,
/// `task_statement`, `agent_output`. Rows lacking any of them are
/// filtered out, matching the contract documented on
/// [`AuditSampler::sample`].
pub struct SqliteAuditSampler {
    store: SessionStore,
}

impl SqliteAuditSampler {
    /// New sampler bound to an open store.
    #[must_use]
    pub fn new(store: SessionStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl AuditSampler for SqliteAuditSampler {
    async fn total_count(&self, skill_id: &str) -> ArgoResult<usize> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT payload_json FROM audit_record WHERE kind = 'tool_result'",
        )
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("honesty count: {e}")))?;
        Ok(rows
            .into_iter()
            .filter_map(|(p,)| extract_sample(&p, skill_id, Uuid::nil()))
            .count())
    }

    async fn sample(&self, skill_id: &str, limit: usize) -> ArgoResult<Vec<SkillSample>> {
        let rows: Vec<(String, Option<Vec<u8>>)> = sqlx::query_as(
            "SELECT payload_json, session_id FROM audit_record \
             WHERE kind = 'tool_result' ORDER BY seq DESC",
        )
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("honesty sample: {e}")))?;
        let mut out = Vec::with_capacity(limit.min(rows.len()));
        for (payload, sess) in rows {
            let session = sess
                .as_deref()
                .and_then(|b| Uuid::from_slice(b).ok())
                .unwrap_or_else(Uuid::nil);
            if let Some(s) = extract_sample(&payload, skill_id, session) {
                out.push(s);
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }
}

/// Parse one `tool_result` audit payload into a [`SkillSample`].
/// `None` if the row does not match `skill_id` or is missing any
/// required convention key.
fn extract_sample(payload: &str, skill_id: &str, session: Uuid) -> Option<SkillSample> {
    let v: Value = serde_json::from_str(payload).ok()?;
    let obj = v.as_object()?;
    if obj.get("skill_id").and_then(Value::as_str)? != skill_id {
        return None;
    }
    let self_reported = obj.get("self_reported_success")?.as_bool()?;
    let task = obj.get("task_statement")?.as_str()?.to_string();
    let output = obj.get("agent_output")?.as_str()?.to_string();
    Some(SkillSample {
        session_id: SessionId(session),
        task_statement: task,
        agent_output: output,
        self_reported_success: self_reported,
    })
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

async fn build_judge(provider_id: &str, model_id: &str) -> ArgoResult<LlmJudge> {
    let manifest = load_manifest(provider_id)?;
    let transport = build_transport(manifest).await?;
    Ok(LlmJudge::new(
        transport,
        ProviderModel {
            provider: provider_id.to_string(),
            model: model_id.to_string(),
        },
        Rubric::default_rubric(),
    ))
}

fn load_manifest(provider_id: &str) -> ArgoResult<ProviderManifest> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let path: PathBuf = cwd
        .join("manifests")
        .join("providers")
        .join(format!("{provider_id}.toml"));
    let body = std::fs::read_to_string(&path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    ProviderManifest::from_toml_str(&body)
}

async fn build_transport(manifest: ProviderManifest) -> ArgoResult<Arc<dyn Transport>> {
    let api_env = manifest.auth.api_key_env.clone();
    let family = manifest.provider.family;
    let transport: Arc<dyn Transport> = match family {
        ProviderFamily::Bedrock => {
            let cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            Arc::new(BedrockTransport::from_manifest(manifest, &cfg)?)
        }
        ProviderFamily::Anthropic => {
            let api_key = argo_config::cred::get(&api_env)
                .map_err(|_| ArgoError::Provider(format!("{api_env} not set")))?;
            Arc::new(AnthropicTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiCompatible => {
            let api_key = argo_config::cred::get(&api_env)
                .map_err(|_| ArgoError::Provider(format!("{api_env} not set")))?;
            Arc::new(OpenAiCompatTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiResponses => {
            let api_key = argo_config::cred::get(&api_env)
                .map_err(|_| ArgoError::Provider(format!("{api_env} not set")))?;
            Arc::new(OpenAiResponsesTransport::from_manifest(manifest, &api_key)?)
        }
    };
    Ok(transport)
}

fn emit(result: &HonestyResult, skill_id: &str, json_out: bool) -> ArgoResult<()> {
    if json_out {
        let env = json!({ "ok": true, "skill_id": skill_id, "honesty": result });
        let body = serde_json::to_string(&env)
            .map_err(|e| ArgoError::Provider(format!("serialize honesty: {e}")))?;
        println!("{body}");
        return Ok(());
    }
    match result {
        HonestyResult::NotEnoughHistory { available, required } => {
            println!("honesty: {skill_id}: not enough history ({available}/{required})");
        }
        HonestyResult::Honest { self_reported, observed, delta, breakdown } => {
            println!(
                "honesty: {skill_id}: HONEST  self={self_reported:.2}  observed={observed:.2}  delta={delta:.2}  n={}",
                breakdown.len()
            );
        }
        HonestyResult::Suspicious {
            self_reported, observed, delta, sample_size, breakdown,
        } => {
            println!(
                "honesty: {skill_id}: SUSPICIOUS  self={self_reported:.2}  observed={observed:.2}  delta={delta:.2}  n={sample_size}"
            );
            for (i, row) in breakdown.iter().enumerate() {
                let marker = if row.disagreement { "≠" } else { "=" };
                println!(
                    "  [{i:>2}] session={} claimed={} judge={} {marker}",
                    row.session_id.0, row.self_reported_success, row.judge_observed_success
                );
            }
        }
    }
    Ok(())
}
