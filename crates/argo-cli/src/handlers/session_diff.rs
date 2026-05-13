//! `argo session diff <id1> <id2>` — offline structural diff of
//! two recorded sessions. Lifted out of `session.rs` so that
//! file stays under the 500-LOC budget. No store mutation —
//! pure read.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

pub(super) async fn diff_sessions(
    raw1: &str,
    raw2: &str,
    json: bool,
    format: &str,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let id1 = uuid::Uuid::parse_str(raw1)
        .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw1}': {e}")))?;
    let id2 = uuid::Uuid::parse_str(raw2)
        .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw2}': {e}")))?;
    let s1 = store
        .get_session(SessionId(id1))
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw1}")))?;
    let s2 = store
        .get_session(SessionId(id2))
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw2}")))?;
    let m1 = store.list_messages(SessionId(id1)).await?;
    let m2 = store.list_messages(SessionId(id2)).await?;
    let mut header_diffs: Vec<(&str, String, String)> = Vec::new();
    if s1.model != s2.model {
        header_diffs.push(("model", s1.model.clone(), s2.model.clone()));
    }
    if s1.provider != s2.provider {
        header_diffs.push(("provider", s1.provider.clone(), s2.provider.clone()));
    }
    if s1.title != s2.title {
        header_diffs.push((
            "title",
            s1.title.clone().unwrap_or_default(),
            s2.title.clone().unwrap_or_default(),
        ));
    }
    let pairs = m1.len().min(m2.len());
    let mut first_div: Option<usize> = None;
    let mut diverged = 0_usize;
    for i in 0..pairs {
        if m1[i].role != m2[i].role || m1[i].content_json != m2[i].content_json {
            diverged += 1;
            if first_div.is_none() {
                first_div = Some(i);
            }
        }
    }
    let trailing = m1.len().abs_diff(m2.len());
    // `--json` wins over `--format` for backwards compat.
    let pick = if json { "json" } else { format };
    match pick {
        "json" => render_json(&s1, &s2, &m1, &m2, &header_diffs, pairs, diverged, trailing, first_div),
        "markdown" => render_markdown(&s1, &s2, &m1, &m2, &header_diffs, pairs, diverged, trailing, first_div),
        "text" => {
            render_text(&s1, &s2, &m1, &m2, &header_diffs, pairs, diverged, trailing, first_div);
            Ok(())
        }
        other => Err(ArgoError::Provider(format!(
            "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_json(
    s1: &argo_store::SessionHeader,
    s2: &argo_store::SessionHeader,
    m1: &[argo_store::MessageRow],
    m2: &[argo_store::MessageRow],
    header_diffs: &[(&str, String, String)],
    pairs: usize,
    diverged: usize,
    trailing: usize,
    first_div: Option<usize>,
) -> ArgoResult<()> {
    let header_diff_objects: Vec<_> = header_diffs
        .iter()
        .map(|(field, a, b)| serde_json::json!({"field": field, "a": a, "b": b}))
        .collect();
    let row = serde_json::json!({
        "a": {"id": s1.id.0.to_string(), "messages": m1.len()},
        "b": {"id": s2.id.0.to_string(), "messages": m2.len()},
        "header_diffs": header_diff_objects,
        "messages_compared": pairs,
        "messages_diverged": diverged,
        "trailing_only": trailing,
        "first_divergence": first_div,
        "byte_identical": header_diffs.is_empty() && diverged == 0 && trailing == 0,
    });
    let serialized = serde_json::to_string(&row)
        .map_err(|e| ArgoError::Provider(format!("serialize diff: {e}")))?;
    println!("{serialized}");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_text(
    s1: &argo_store::SessionHeader,
    s2: &argo_store::SessionHeader,
    m1: &[argo_store::MessageRow],
    m2: &[argo_store::MessageRow],
    header_diffs: &[(&str, String, String)],
    pairs: usize,
    diverged: usize,
    trailing: usize,
    first_div: Option<usize>,
) {
    println!("session diff");
    println!("  a: {} ({} messages)", s1.id.0, m1.len());
    println!("  b: {} ({} messages)", s2.id.0, m2.len());
    if header_diffs.is_empty() {
        println!("  header: identical (model/provider/title)");
    } else {
        println!("  header diffs:");
        for (field, a, b) in header_diffs {
            println!("    {field}: a={a:?} b={b:?}");
        }
    }
    println!(
        "  messages: {pairs} compared, {diverged} diverged, {trailing} trailing-only on the longer side"
    );
    if let Some(i) = first_div {
        println!(
            "  first divergence: seq {} (role a={} b={})",
            i, m1[i].role, m2[i].role
        );
    }
    if header_diffs.is_empty() && diverged == 0 && trailing == 0 {
        println!("sessions are byte-identical");
    }
}

#[allow(clippy::too_many_arguments)]
fn render_markdown(
    s1: &argo_store::SessionHeader,
    s2: &argo_store::SessionHeader,
    m1: &[argo_store::MessageRow],
    m2: &[argo_store::MessageRow],
    header_diffs: &[(&str, String, String)],
    pairs: usize,
    diverged: usize,
    trailing: usize,
    first_div: Option<usize>,
) -> ArgoResult<()> {
    let mut out = String::new();
    out.push_str("# Session diff\n\n");
    out.push_str(&format!(
        "- **a**: `{}` ({} messages)\n",
        s1.id.0,
        m1.len()
    ));
    out.push_str(&format!(
        "- **b**: `{}` ({} messages)\n\n",
        s2.id.0,
        m2.len()
    ));
    out.push_str("## Header\n\n");
    if header_diffs.is_empty() {
        out.push_str("_Identical (model/provider/title)._\n\n");
    } else {
        out.push_str("| field | a | b |\n| --- | --- | --- |\n");
        for (field, a, b) in header_diffs {
            out.push_str(&format!("| `{field}` | `{a}` | `{b}` |\n"));
        }
        out.push('\n');
    }
    out.push_str("## Messages\n\n");
    out.push_str(&format!(
        "- **Compared**: {pairs}\n- **Diverged**: {diverged}\n- **Trailing-only**: {trailing}\n",
    ));
    if let Some(i) = first_div {
        out.push_str(&format!(
            "- **First divergence**: seq {i} (role a=`{}` b=`{}`)\n",
            m1[i].role, m2[i].role
        ));
    }
    if header_diffs.is_empty() && diverged == 0 && trailing == 0 {
        out.push_str("\n_Sessions are byte-identical._\n");
    }
    print!("{out}");
    Ok(())
}
