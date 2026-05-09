//! Tests for the `argo latency` handler — focused on the
//! pure-functional rendering path (markdown + JSON shape) and the
//! row-construction logic. The integration with the in-memory
//! `LatencyTracker` and persistent `latency_buckets` table is
//! covered by `argo_store::latency_tracker` unit tests.
use super::*;

#[test]
fn output_format_parses_known_values_and_falls_back() {
    assert_eq!(OutputFormat::parse("json"), OutputFormat::Json);
    assert_eq!(OutputFormat::parse("JSON"), OutputFormat::Json);
    assert_eq!(OutputFormat::parse(" json "), OutputFormat::Json);
    assert_eq!(OutputFormat::parse("markdown"), OutputFormat::Markdown);
    // Unknown / empty values fall back to markdown so a misconfigured
    // alias still renders something readable instead of erroring.
    assert_eq!(OutputFormat::parse(""), OutputFormat::Markdown);
    assert_eq!(OutputFormat::parse("yaml"), OutputFormat::Markdown);
}

#[test]
fn empty_snapshot_renders_friendly_placeholder() {
    let rows = compute_rows(&HashMap::new());
    assert!(rows.is_empty());
    let md = render_markdown(&rows);
    assert!(md.contains("no samples recorded"));
    assert!(md.contains("| kind | n | p50 | p95 | p99 | max |"));
}

#[test]
fn rows_sort_alphabetically_and_compute_percentiles() {
    let mut snap: HashMap<String, [u64; N_BUCKETS]> = HashMap::new();
    // mcp: 10 samples at 8ms (bucket index 3 → [8, 16) ms)
    let mut mcp = [0u64; N_BUCKETS];
    mcp[3] = 10;
    snap.insert("mcp".to_string(), mcp);
    // human: 5 samples at 2ms (bucket index 1 → [2, 4) ms)
    let mut human = [0u64; N_BUCKETS];
    human[1] = 5;
    snap.insert("human".to_string(), human);
    let rows = compute_rows(&snap);
    assert_eq!(rows.len(), 2);
    // Sorted alphabetically: human, then mcp.
    assert_eq!(rows[0].kind, "human");
    assert_eq!(rows[1].kind, "mcp");
    assert_eq!(rows[0].n, 5);
    assert_eq!(rows[1].n, 10);
    // Multi-sample single-bucket: percentiles interpolate within
    // the bucket — p50 lands near the bucket midpoint, p99 near
    // its upper edge, both inside `[lower, upper]`.
    let h_p50 = rows[0].p50_ms.expect("human p50");
    let h_p99 = rows[0].p99_ms.expect("human p99");
    assert!(
        (2..=4).contains(&h_p50),
        "human p50 should land in [2, 4]ms bucket, got {h_p50}"
    );
    assert!(
        (2..=4).contains(&h_p99) && h_p99 >= h_p50,
        "human p99 should be in bucket and >= p50, got p50={h_p50} p99={h_p99}"
    );
    // Max is the bucket upper bound for non-overflow buckets.
    assert_eq!(rows[0].max_ms, Some(4));
    assert_eq!(rows[1].max_ms, Some(16));
}

#[test]
fn markdown_renders_one_row_per_kind_with_ms_suffixed_cells() {
    let mut snap: HashMap<String, [u64; N_BUCKETS]> = HashMap::new();
    let mut human = [0u64; N_BUCKETS];
    human[2] = 1; // [4, 8) ms
    snap.insert("human".to_string(), human);
    let rows = compute_rows(&snap);
    let md = render_markdown(&rows);
    // Header + separator + 1 row = 3 lines.
    let line_count = md.lines().count();
    assert_eq!(line_count, 3, "expected 3 markdown lines, got:\n{md}");
    // Row line carries the kind and an `ms` suffix on the percentile
    // cells (so operators don't have to guess the unit).
    let row_line = md.lines().nth(2).expect("row line");
    assert!(row_line.contains("human"));
    assert!(row_line.contains("ms"));
    assert!(row_line.contains("| 1 |"), "row line: {row_line}");
}
