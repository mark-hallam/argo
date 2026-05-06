//! `argo classify rules` handler — print resolved per-class
//! retention + redaction rules.

use argo_identity::{ClassificationRules, DataClass};
use argo_types::{ArgoError, ArgoResult};

pub async fn run(json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let rules = ClassificationRules::from_workspace(&cwd);
    if json {
        let mut obj = serde_json::Map::new();
        for class in DataClass::all() {
            let rule = rules.rule_for(class);
            obj.insert(
                class.as_str().to_string(),
                serde_json::json!({
                    "retention_days": rule.retention_days,
                    "redact_in_export": rule.redact_in_export,
                }),
            );
        }
        println!("{}", serde_json::to_string(&obj).unwrap_or_default());
    } else {
        println!(
            "{:<10} {:<16} redact_in_export",
            "class", "retention_days"
        );
        for class in DataClass::all() {
            let rule = rules.rule_for(class);
            let retention = if rule.retention_days == 0 {
                "indefinite".to_string()
            } else {
                rule.retention_days.to_string()
            };
            println!(
                "{:<10} {:<16} {}",
                class.as_str(),
                retention,
                rule.redact_in_export
            );
        }
    }
    Ok(())
}
