//! `argo eval validate` — lint a task JSONL file.
use crate::cli::EvalCmd;
use argo_eval::{load_jsonl, EvalMetric};
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub async fn run(cmd: EvalCmd) -> ArgoResult<()> {
    match cmd {
        EvalCmd::Validate { path } => validate(&PathBuf::from(path)).await,
    }
}

async fn validate(path: &std::path::Path) -> ArgoResult<()> {
    let tasks = load_jsonl(path).await?;
    let mut by_metric: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for t in &tasks {
        let kind = match &t.metric {
            EvalMetric::ExactMatch => "exact_match",
            EvalMetric::SubstringContains { .. } => "substring_contains",
            EvalMetric::JaccardTokens => "jaccard_tokens",
        };
        *by_metric.entry(kind).or_insert(0) += 1;
    }
    println!("{} valid task(s) in {}", tasks.len(), path.display());
    if tasks.is_empty() {
        return Err(ArgoError::Provider("no tasks parsed (empty file?)".into()));
    }
    for (kind, count) in by_metric {
        println!("  {count:>4}  {kind}");
    }
    Ok(())
}
