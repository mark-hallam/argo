//! `argo gateway routes` CLI subcommand — pretty-prints the static
//! route catalog the gateway serves. Filters apply in order:
//! method → grep → limit. Extracted from `mod.rs` to keep that file
//! under the 500-LOC ceiling.

use argo_types::{ArgoError, ArgoResult};

use super::routes_catalog;

pub(super) fn print_routes(
    json: bool,
    format: &str,
    method: Option<&str>,
    grep: Option<&str>,
    limit: Option<usize>,
) -> ArgoResult<()> {
    let (_status, body) = routes_catalog::routes_response(method);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| ArgoError::Provider(format!("parse: {e}")))?;
    let mut rows = parsed.as_array().cloned().unwrap_or_default();
    if let Some(needle) = grep {
        let lower = needle.to_lowercase();
        rows.retain(|r| {
            let path = r["path"].as_str().unwrap_or("").to_lowercase();
            let desc = r["description"].as_str().unwrap_or("").to_lowercase();
            path.contains(&lower) || desc.contains(&lower)
        });
    }
    if let Some(n) = limit {
        rows.truncate(n);
    }
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            println!(
                "{}",
                serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
            );
            return Ok(());
        }
        "markdown" => {
            println!("# Gateway routes ({} endpoints)", rows.len());
            println!();
            if rows.is_empty() {
                return Ok(());
            }
            println!("| method | path | description |");
            println!("| :---: | --- | --- |");
            for row in &rows {
                let m = row["method"].as_str().unwrap_or("?");
                let p = row["path"].as_str().unwrap_or("?");
                let d = row["description"]
                    .as_str()
                    .unwrap_or("")
                    .replace('|', "\\|");
                println!("| `{m}` | `{p}` | {d} |");
            }
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    println!("argo gateway routes ({} endpoints)", rows.len());
    let width = (rows.len().max(1) as f64).log10().ceil() as usize;
    for (i, row) in rows.iter().enumerate() {
        let m = row["method"].as_str().unwrap_or("?");
        let p = row["path"].as_str().unwrap_or("?");
        let d = row["description"].as_str().unwrap_or("");
        let n = i + 1;
        println!("  {n:>width$}. {m:<5} {p:<40} {d}");
    }
    Ok(())
}
