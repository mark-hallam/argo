# Mission load-bearing wire — declarative KPI primitives in vertical manifest

**Owner**: claude. **Depends on**: `VerticalManifest` struct SHIPPED (verified at `crates/argo-vertical/src/manifest.rs:29`); `SuccessMetricsSection` SHIPPED (line 200, 4 hardcoded fields); `claude-worker-kpi-tracker` MERGED.
**Authored against**: origin/main `8c4bafaf` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-kpi-declarative`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 2 item #54 ("Add KPI primitives — declarative KPI definitions in vertical manifest"). Today `SuccessMetricsSection` (`manifest.rs:200`) hard-codes 4 fields: deflection_target_pct, median_response_seconds, csat_threshold, required_audit_chain_intact. Each new release-gate metric requires a code change. Mission Claim 2 ("delegation as unit") needs verticals to declare their OWN KPIs in TOML — no Rust change required.

## Scope

ADD a NEW optional `[[kpi]]` array of KPI structs alongside the existing `success_metrics` section (don't replace — operators with existing manifests must remain valid). Each KPI:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KpiDefinition {
    pub name: String,                        // e.g. "deflection_rate", "first_response_p95_seconds"
    pub kind: KpiKind,                       // Percentage / DurationMs / Count / Ratio / Score
    pub threshold: f64,                      // operator-chosen pass/fail line
    pub comparator: KpiComparator,           // Gte / Lte / Eq (within epsilon)
    pub gate: KpiGate,                       // Release / Warn / Inform
    #[serde(default)]
    pub description: Option<String>,
}

pub enum KpiKind { Percentage, DurationMs, Count, Ratio, Score }
pub enum KpiComparator { Gte, Lte, Eq }
pub enum KpiGate { Release, Warn, Inform }
```

The existing 4 hardcoded fields stay as convenience shortcuts — internally they each generate an equivalent `KpiDefinition` if `kpis` is empty (back-compat for existing manifests). New manifests prefer the declarative `[[kpi]]` blocks.

NEW `VerticalManifest::all_kpis() -> Vec<KpiDefinition>` materializes both legacy + declarative into one unified list for downstream consumers (eval CI gate, worker KPI tracker).

This is NOT in scope: enforcement engine (downstream consumers decide what to do with a failing KPI); per-tenant KPI overrides; KPI history rollup; computed/derived KPIs (e.g. ratio of two raw KPIs).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-vertical/src/manifest.rs` — narrowly scoped: ADD `KpiDefinition`, `KpiKind`, `KpiComparator`, `KpiGate` enums/struct; ADD `pub kpis: Vec<KpiDefinition>` field with `#[serde(default)]`; ADD `pub fn all_kpis(&self) -> Vec<KpiDefinition>` that materializes the 4 hardcoded fields into KpiDefinitions when `self.kpis.is_empty()`. NO change to `SuccessMetricsSection`. NO breaking changes. File currently ~300 LOC — verify under 500 after additions.
- `crates/argo-vertical/src/manifest_kpi_tests.rs` (NEW, sibling) — 6 unit tests: parse-kpi-block; empty-kpis-falls-back-to-legacy; both-kpis-and-legacy-merge-correctly; comparator-Gte-semantics; comparator-Lte-semantics; release-vs-warn-vs-inform-gate.
- `crates/argo-vertical/src/manifest_validate.rs` — narrowly scoped: ADD validation: each KPI name must be unique within a manifest; threshold must be finite; description ≤ 200 chars. Reuse existing validation patterns.
- `docs/skycot/manifest-kpi-schema.md` (NEW, ≤ 80 LOC) — short docs for operators: TOML example, semantics of each field, gate semantics.

## Files you must NOT touch

- `crates/argo-vertical/src/manifest_validate_tests.rs` — append-only for sibling test file; if not feasible, narrow edit OK
- Existing `SuccessMetricsSection` definition — adding a sibling field; do NOT modify existing fields
- Any non-vertical crate (downstream consumers will be wired in a separate brief)
- `crates/argo-store/migrations/**` — no schema change (manifest is TOML, not stored)

## Acceptance criteria

- [ ] `KpiDefinition`, `KpiKind`, `KpiComparator`, `KpiGate` types exist with full serde + clap-style derives
- [ ] `[[kpi]]` blocks parse from TOML; legacy `success_metrics` block still parses
- [ ] Empty `kpis` array falls back to materializing the 4 legacy fields (back-compat)
- [ ] Both populated → both included in `all_kpis()` output
- [ ] Validation: duplicate KPI names rejected; non-finite thresholds rejected
- [ ] `all_kpis()` returns a unified list — downstream consumers don't need to check both sections
- [ ] 6 unit tests + manifest validation tests
- [ ] Existing manifest parsing tests still pass byte-identical
- [ ] Short operator-facing docs added
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-vertical` clean
- [ ] `cargo clippy -p argo-vertical --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-vertical` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vertical-kpi-declarative`
- PR title: `vertical: declarative [[kpi]] manifest blocks alongside legacy success_metrics (mission-vertical-kpi-declarative)`

## Notes for the implementer

- The legacy-back-compat path is load-bearing: every shipped vertical manifest has the 4-field `success_metrics`; breaking them on parse is unacceptable. The `all_kpis()` fallback is the bridge.
- `KpiGate::Release` = blocks release on fail. `KpiGate::Warn` = surfaces in dashboards but doesn't block. `KpiGate::Inform` = telemetry-only. Downstream consumers (eval CI, dashboard) honor this; THIS brief just defines the field.
- Don't add an enforcement engine here. That's a follow-up brief (`mission-kpi-enforcement-gate`) and will live in argo-eval or similar.
- The 5 KpiKind variants are intentional — operators want type-safety when interpreting thresholds (e.g. 95 means "95%" for Percentage but "95ms" for DurationMs). The kind drives display formatting and unit suffixes downstream.
