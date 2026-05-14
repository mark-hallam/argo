//! `argo judge eval-disagreement` — judge-vs-human calibration CLI.
//!
//! Walks the fixture directory, runs each labelled case through an
//! [`LlmJudge`] built from `--provider` / `--model`, and emits a
//! disagreement report. Designed to be wired into CI gates around
//! judge-model upgrades via `--bail-below <rate>`.
//!
//! The provider / model surface mirrors `argo eval skill-honesty`
//! intentionally — both wire the same `LlmJudge` against a manifest
//! transport. Ongoing fixture-set maintenance is the operator's job;
//! see `crates/argo-judge/fixtures/disagreement/README` (TBD).
use argo_judge::{
    load_fixtures, run_disagreement_eval, DisagreementReport, LlmJudge, Rubric, DEFAULT_FIXTURES_DIR,
};
use argo_provider::{
    AnthropicTransport, BedrockTransport, OpenAiCompatTransport, OpenAiResponsesTransport,
    ProviderFamily, ProviderManifest, Transport,
};
use argo_types::{ArgoError, ArgoResult, ProviderModel};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// CLI entry. Mirrors the `JudgeCmd::EvalDisagreement` variant.
pub async fn run(
    fixtures: Option<PathBuf>,
    provider: &str,
    model: &str,
    json_out: bool,
    bail_below: f32,
) -> ArgoResult<()> {
    let dir = resolve_fixtures_dir(fixtures.as_deref())?;
    let cases = load_fixtures(&dir)?;
    if cases.is_empty() {
        return Err(ArgoError::Provider(format!(
            "no fixtures found in {} — vendor at least one *.json case",
            dir.display()
        )));
    }
    let judge = build_judge(provider, model).await?;
    let report = run_disagreement_eval(&judge, &cases).await?;
    if json_out {
        print_json(&report);
    } else {
        print_text(&report, &dir);
    }
    if !report.passes_threshold(bail_below) {
        return Err(ArgoError::Provider(format!(
            "judge disagreement: agreement rate {:.3} below threshold {:.3}",
            report.agreement_rate, bail_below
        )));
    }
    Ok(())
}

fn resolve_fixtures_dir(arg: Option<&Path>) -> ArgoResult<PathBuf> {
    if let Some(p) = arg {
        return Ok(p.to_path_buf());
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    Ok(cwd.join(DEFAULT_FIXTURES_DIR))
}

fn print_text(report: &DisagreementReport, dir: &Path) {
    println!("argo judge eval-disagreement");
    println!("  fixtures dir : {}", dir.display());
    println!("  total cases  : {}", report.total);
    println!(
        "  agreement    : {} / {} ({:.1}%)",
        report.agreed,
        report.total,
        report.agreement_rate * 100.0
    );
    if !report.buckets.is_empty() {
        println!("  per bucket:");
        for (tag, b) in &report.buckets {
            println!(
                "    {tag:<14} {:>3}/{:<3} ({:>5.1}%)",
                b.agreed,
                b.total,
                b.agreement_rate * 100.0
            );
        }
    }
    if !report.mismatches.is_empty() {
        println!("  mismatches:");
        for m in &report.mismatches {
            println!(
                "    {} [{}] human={} judge={}",
                m.id,
                m.tag.label(),
                m.human_verdict,
                m.judge_verdict
            );
        }
    }
}

fn print_json(report: &DisagreementReport) {
    let envelope = json!({
        "kind": "judge_disagreement",
        "report": report,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".into())
    );
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
