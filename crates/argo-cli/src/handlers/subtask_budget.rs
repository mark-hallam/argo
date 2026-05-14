//! `argo subtask budget <root-id> [--json]` — render the
//! per-delegation budget tree rooted at one subtask id (Stream
//! `mission-delegation-budget-tree`). Reads
//! `argo_store::subtask_budget::walk_tree` and prints either a
//! human-readable hierarchical tree or a JSON envelope.
use argo_store::subtask_budget::{walk_tree, WalkedSubtask};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::collections::HashMap;

use crate::home::{db_path, ensure_dir, resolve_home};

pub async fn run(root_id: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;

    let rows = walk_tree(store.pool(), root_id).await?;

    if json {
        let payload: Vec<serde_json::Value> = rows.iter().map(walked_to_json).collect();
        let envelope = serde_json::json!({
            "ok": true,
            "root_id": root_id,
            "count": payload.len(),
            "rows": payload,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
        return Ok(());
    }

    if rows.is_empty() {
        println!("(no subtask budget tree rooted at '{root_id}')");
        return Ok(());
    }
    render_tree(root_id, &rows);
    Ok(())
}

fn walked_to_json(w: &WalkedSubtask) -> serde_json::Value {
    serde_json::json!({
        "id": w.row.id,
        "parent_subtask_id": w.parent_subtask_id,
        "worker_id": w.row.worker_id,
        "plan_node_path": w.row.plan_node_path,
        "description": w.row.description,
        "state": w.row.state.as_str(),
        "budget_usd_cents": w.budget_usd_cents,
        "spent_usd_cents": w.spent_usd_cents,
        "aggregated_spent_usd_cents": w.aggregated_spent_usd_cents,
    })
}

fn render_tree(root_id: &str, rows: &[WalkedSubtask]) {
    // Build the parent → children index in created_at order (already
    // walked into `rows` ascending), so the printed tree mirrors
    // insert order at every level.
    let mut children_of: HashMap<&str, Vec<&WalkedSubtask>> = HashMap::new();
    let mut root: Option<&WalkedSubtask> = None;
    for w in rows {
        if w.row.id == root_id {
            root = Some(w);
        } else if let Some(p) = w.parent_subtask_id.as_deref() {
            children_of.entry(p).or_default().push(w);
        }
    }
    let Some(root) = root else {
        println!("(root '{root_id}' not present in walk result)");
        return;
    };
    println!(
        "{} budget={} spent={} aggregated={}",
        node_label(root),
        fmt_cents(root.budget_usd_cents),
        fmt_signed(root.spent_usd_cents),
        fmt_signed(root.aggregated_spent_usd_cents),
    );
    print_children(&root.row.id, &children_of, "");
}

fn print_children(
    parent_id: &str,
    children_of: &HashMap<&str, Vec<&WalkedSubtask>>,
    indent: &str,
) {
    let Some(kids) = children_of.get(parent_id) else {
        return;
    };
    let last = kids.len().saturating_sub(1);
    for (i, kid) in kids.iter().enumerate() {
        let connector = if i == last { "└──" } else { "├──" };
        println!(
            "{indent}{connector} {} budget={} spent={} aggregated={}",
            node_label(kid),
            fmt_cents(kid.budget_usd_cents),
            fmt_signed(kid.spent_usd_cents),
            fmt_signed(kid.aggregated_spent_usd_cents),
        );
        let child_indent = if i == last {
            format!("{indent}    ")
        } else {
            format!("{indent}│   ")
        };
        print_children(&kid.row.id, children_of, &child_indent);
    }
}

fn node_label(w: &WalkedSubtask) -> String {
    format!(
        "{} (id={} worker={})",
        w.row.plan_node_path, w.row.id, w.row.worker_id
    )
}

fn fmt_cents(c: Option<i64>) -> String {
    match c {
        Some(v) => fmt_signed(v),
        None => "$unbounded".into(),
    }
}

fn fmt_signed(c: i64) -> String {
    let negative = c < 0;
    let abs = c.unsigned_abs();
    let dollars = abs / 100;
    let cents = abs % 100;
    let prefix = if negative { "-$" } else { "$" };
    format!("{prefix}{dollars}.{cents:02}")
}
