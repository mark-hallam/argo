//! `argo trace explore <session>` — walk a session's local OTLP span
//! tree from `.skycot/traces/<session>.ndjson` and render it as
//! markdown (default) or nested JSON.
//!
//! The local NDJSON sink is one-span-per-line. Each line deserializes
//! into a [`SpanRecord`]. Spans are linked into a tree by
//! `parent_span_id`; spans with no matching parent become roots
//! (orphans). The walker is defensive against circular `parent_span_id`
//! chains: a depth cap of [`MAX_DEPTH`] terminates any cycle.
//!
//! This handler is a read-only consumer of the sink — it does not
//! create or modify NDJSON files. The producer side is wired in the
//! OTLP exporter (`argo-telemetry`) when the local-sink extension
//! lands; until then, hand-written NDJSON fixtures drive tests and
//! adventurous operators.

use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Default directory where the local OTLP sink writes NDJSON files.
pub const DEFAULT_TRACE_DIR: &str = ".skycot/traces";

/// Defensive cap on tree depth — protects against circular
/// `parent_span_id` chains and pathologically deep trees alike.
pub const MAX_DEPTH: usize = 32;

/// Output format selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceFormat {
    /// Indented markdown bullet list.
    Markdown,
    /// Nested `{ span, children: [...] }` JSON.
    Json,
}

impl TraceFormat {
    /// Parse a `--format` string. Defaults to markdown for empty input.
    pub fn parse(s: &str) -> ArgoResult<Self> {
        match s.to_ascii_lowercase().as_str() {
            "" | "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            other => Err(ArgoError::Provider(format!(
                "trace explore: unknown --format '{other}' (expected 'markdown' or 'json')"
            ))),
        }
    }
}

/// One OTLP span as serialized into the local NDJSON sink. Field
/// names match the OTLP/JSON conventions used by the exporter so
/// fixtures stay human-readable. Kept private (only the renderer
/// consumes it).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpanRecord {
    span_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent_span_id: Option<String>,
    name: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    start_unix_nano: u64,
    #[serde(default)]
    end_unix_nano: u64,
    #[serde(default)]
    status: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    attributes: HashMap<String, serde_json::Value>,
}

impl SpanRecord {
    fn duration_ms(&self) -> f64 {
        if self.end_unix_nano <= self.start_unix_nano {
            return 0.0;
        }
        let nanos = self.end_unix_nano - self.start_unix_nano;
        // f64 has 53 bits of mantissa; nano counts up to ~2^63 — for
        // single-span durations this is plenty of precision.
        (nanos as f64) / 1_000_000.0
    }

    fn status_label(&self) -> &str {
        if self.status.is_empty() {
            "unset"
        } else {
            self.status.as_str()
        }
    }
}

/// Tree node — a span plus its (already sorted) children.
#[derive(Debug, Serialize)]
struct TreeNode {
    span: SpanRecord,
    children: Vec<TreeNode>,
}

/// Entry point invoked from `main.rs` dispatch.
pub fn run(session: &str, format: TraceFormat, path: Option<&Path>) -> ArgoResult<()> {
    let file_path = resolve_session_path(session, path);
    let spans = load_spans(&file_path)?;
    let roots = build_tree(spans);
    match format {
        TraceFormat::Markdown => render_markdown(&roots),
        TraceFormat::Json => render_json(&roots)?,
    }
    Ok(())
}

fn resolve_session_path(session: &str, path: Option<&Path>) -> PathBuf {
    let dir = match path {
        Some(p) => p.to_path_buf(),
        None => PathBuf::from(DEFAULT_TRACE_DIR),
    };
    dir.join(format!("{session}.ndjson"))
}

fn load_spans(file_path: &Path) -> ArgoResult<Vec<SpanRecord>> {
    let file = File::open(file_path).map_err(|e| {
        ArgoError::Provider(format!(
            "trace explore: cannot open '{}': {e}",
            file_path.display()
        ))
    })?;
    let reader = BufReader::new(file);
    let mut spans = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| {
            ArgoError::Provider(format!(
                "trace explore: read error in '{}' at line {}: {e}",
                file_path.display(),
                line_no + 1
            ))
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let span: SpanRecord = serde_json::from_str(trimmed).map_err(|e| {
            ArgoError::Provider(format!(
                "trace explore: invalid NDJSON in '{}' at line {}: {e}",
                file_path.display(),
                line_no + 1
            ))
        })?;
        spans.push(span);
    }
    Ok(spans)
}

/// Build a forest from the flat span list. Spans whose
/// `parent_span_id` is `None`, empty, or references an unknown
/// span_id become roots. Children are sorted by `start_unix_nano`
/// for deterministic output. Defensive against duplicate span_ids
/// (later wins; the first is dropped) and cycles (depth cap below).
fn build_tree(spans: Vec<SpanRecord>) -> Vec<TreeNode> {
    let mut by_id: HashMap<String, SpanRecord> = HashMap::with_capacity(spans.len());
    let mut order: Vec<String> = Vec::with_capacity(spans.len());
    for span in spans {
        let id = span.span_id.clone();
        if !by_id.contains_key(&id) {
            order.push(id.clone());
        }
        by_id.insert(id, span);
    }

    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut root_ids: Vec<String> = Vec::new();
    for id in &order {
        let span = &by_id[id];
        let parent = span
            .parent_span_id
            .as_ref()
            .filter(|p| !p.is_empty())
            .filter(|p| by_id.contains_key(p.as_str()) && p.as_str() != id);
        match parent {
            Some(parent_id) => children_of
                .entry(parent_id.clone())
                .or_default()
                .push(id.clone()),
            None => root_ids.push(id.clone()),
        }
    }

    let mut roots = Vec::with_capacity(root_ids.len());
    for id in root_ids {
        roots.push(build_node(&id, &mut by_id, &children_of, 0));
    }
    sort_forest(&mut roots);
    roots
}

fn build_node(
    id: &str,
    by_id: &mut HashMap<String, SpanRecord>,
    children_of: &HashMap<String, Vec<String>>,
    depth: usize,
) -> TreeNode {
    let span = by_id.remove(id).unwrap_or_else(|| SpanRecord {
        span_id: id.to_string(),
        parent_span_id: None,
        name: format!("<missing span {id}>"),
        kind: String::new(),
        start_unix_nano: 0,
        end_unix_nano: 0,
        status: String::new(),
        attributes: HashMap::new(),
    });
    let mut children = Vec::new();
    if depth < MAX_DEPTH {
        if let Some(child_ids) = children_of.get(id) {
            for child_id in child_ids {
                if by_id.contains_key(child_id) {
                    children.push(build_node(child_id, by_id, children_of, depth + 1));
                }
            }
        }
    }
    TreeNode { span, children }
}

fn sort_forest(forest: &mut Vec<TreeNode>) {
    forest.sort_by_key(|n| n.span.start_unix_nano);
    for node in forest {
        sort_forest(&mut node.children);
    }
}

fn render_markdown(roots: &[TreeNode]) {
    if roots.is_empty() {
        return;
    }
    for root in roots {
        render_md_node(root, 0);
    }
}

fn render_md_node(node: &TreeNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let span = &node.span;
    println!(
        "{indent}- {name} ({kind}, dur={dur:.1}ms, status={status})",
        name = span.name,
        kind = if span.kind.is_empty() { "INTERNAL" } else { span.kind.as_str() },
        dur = span.duration_ms(),
        status = span.status_label(),
    );
    for child in &node.children {
        render_md_node(child, depth + 1);
    }
}

fn render_json(roots: &[TreeNode]) -> ArgoResult<()> {
    let payload = serde_json::to_string_pretty(roots)
        .map_err(|e| ArgoError::Provider(format!("trace explore: json render: {e}")))?;
    println!("{payload}");
    Ok(())
}

#[cfg(test)]
#[path = "trace_explore_tests.rs"]
mod tests;
