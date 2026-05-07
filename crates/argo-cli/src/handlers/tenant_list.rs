//! Tenant list support for `argo tenant`.

use super::tenant_archive::{open_store, print_json};
use argo_store::tenant_query::list_tenants_with_stats;
use argo_types::ArgoResult;
use chrono::Utc;
use serde_json::{json, Value};

pub async fn run(include_archived: bool, json_out: bool) -> ArgoResult<()> {
    let (store, _) = open_store().await?;
    let tenants = list_tenants_with_stats(&store, include_archived, Utc::now()).await?;
    if json_out {
        print_json(&envelope(include_archived, tenants))
    } else {
        print_table(&tenants);
        Ok(())
    }
}

fn envelope(include_archived: bool, tenants: Vec<Value>) -> Value {
    json!({"ok": true, "include_archived": include_archived, "tenants": tenants})
}

fn print_table(rows: &[Value]) {
    if rows.is_empty() {
        println!("(no tenants)");
        return;
    }
    println!(
        "{:<36}  {:<24}  {:<24}  workers  {:>12}  {:>12}",
        "tenant_id", "created_at", "archived_at", "spend_today", "spend_month"
    );
    for row in rows {
        println!(
            "{:<36}  {:<24}  {:<24}  {:>7}  {:>12}  {:>12}",
            text(row, "id"),
            text(row, "created_at_rfc3339"),
            text(row, "archived_at_rfc3339"),
            row["worker_count"].as_i64().unwrap_or_default(),
            text(row, "spend_today"),
            text(row, "spend_this_month")
        );
    }
}

fn text(row: &Value, key: &str) -> String {
    row[key].as_str().unwrap_or("-").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_marks_json_success() {
        let out = envelope(false, vec![json!({"id":"t"})]);
        assert_eq!(out["ok"], true);
        assert_eq!(out["include_archived"], false);
        assert_eq!(out["tenants"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn text_defaults_missing_values_to_dash() {
        assert_eq!(text(&json!({}), "missing"), "-");
    }
}
