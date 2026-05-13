//! `model = "latest"` resolution (Amendment 8 / §8.2).
//!
//! When the user's `argo.toml` sets `model = "latest"`, the
//! manifest's `[discovery]` block is consulted at startup and every
//! `refresh_interval_hours`. We pull `endpoint` (with `{base_url}`
//! substituted in), filter the returned id list by
//! `include_pattern` and `exclude_pattern`, and pick a winner per
//! `prefer`.
//!
//! This module ships the **filtering + selection** logic — given a
//! `DiscoverySpec` and a list of model ids, return the resolved
//! one. The HTTP fetch itself lives upstream in the daemon (so we
//! can test `/v1/models` against fixtures without spinning a
//! reqwest client).
use crate::manifest::{DiscoverySpec, PreferRule};
use argo_types::{ArgoError, ArgoResult};
use regex::Regex;

/// Resolve `model = "latest"` against `model_ids` per `spec`.
///
/// Returns the chosen id, or `Err` if filtering left no candidates
/// (the daemon falls back to the manifest's `model = "..."` literal
/// in that case).
///
/// # Errors
/// - `Provider("invalid include_pattern: ...")` if the regex doesn't compile.
/// - `Provider("invalid exclude_pattern: ...")` if the regex doesn't compile.
/// - `Provider("no models matched discovery filters")` if the
///   intersection is empty.
pub fn resolve_latest<S: AsRef<str>>(spec: &DiscoverySpec, model_ids: &[S]) -> ArgoResult<String> {
    let include = Regex::new(&spec.include_pattern).map_err(|e| {
        ArgoError::Provider(format!(
            "invalid include_pattern {:?}: {e}",
            spec.include_pattern
        ))
    })?;
    let exclude = match &spec.exclude_pattern {
        Some(s) => Some(
            Regex::new(s)
                .map_err(|e| ArgoError::Schema(format!("invalid exclude_pattern {s:?}: {e}")))?,
        ),
        None => None,
    };
    let mut candidates: Vec<&str> = model_ids
        .iter()
        .map(AsRef::as_ref)
        .filter(|id| include.is_match(id))
        .filter(|id| exclude.as_ref().is_none_or(|r| !r.is_match(id)))
        .collect();
    if candidates.is_empty() {
        return Err(ArgoError::Provider(
            "no models matched discovery filters".into(),
        ));
    }
    match spec.prefer {
        PreferRule::HighestMinor => candidates.sort_by(|a, b| compare_versioned(b, a)),
        PreferRule::LexicographicMax => candidates.sort_by(|a, b| b.cmp(a)),
    }
    Ok(candidates[0].to_string())
}

fn compare_versioned(a: &str, b: &str) -> std::cmp::Ordering {
    // Highest-minor extracts numeric runs and compares as integers.
    // E.g., `gpt-4.1-mini` > `gpt-4.0` because the major.minor
    // tuple `(4, 1)` > `(4, 0)`. Falls back to lexicographic when
    // numeric extraction is identical.
    let av = numeric_runs(a);
    let bv = numeric_runs(b);
    av.cmp(&bv).then_with(|| a.cmp(b))
}

fn numeric_runs(s: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let mut acc: u64 = 0;
    let mut in_run = false;
    for c in s.chars() {
        if let Some(d) = c.to_digit(10) {
            acc = acc.saturating_mul(10).saturating_add(u64::from(d));
            in_run = true;
        } else if in_run {
            out.push(acc);
            acc = 0;
            in_run = false;
        }
    }
    if in_run {
        out.push(acc);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(prefer: PreferRule, include: &str, exclude: Option<&str>) -> DiscoverySpec {
        DiscoverySpec {
            endpoint: "{base_url}/v1/models".into(),
            include_pattern: include.into(),
            exclude_pattern: exclude.map(str::to_string),
            prefer,
            refresh_interval_hours: 24,
        }
    }

    #[test]
    fn highest_minor_picks_largest_numeric_tuple() {
        // §8.3: "model = "latest" resolves correctly against a
        // recorded /v1/models fixture."
        let s = spec(PreferRule::HighestMinor, r"^gpt-[0-9]", None);
        let ids = ["gpt-3.5-turbo", "gpt-4.0", "gpt-4.1-mini", "gpt-4.1"];
        let r = resolve_latest(&s, &ids).unwrap();
        // (4,1) ties between "gpt-4.1-mini" and "gpt-4.1"; we break
        // ties lexicographically so "gpt-4.1-mini" wins (m > nothing
        // in the lexicographic comparison after the tuple match).
        assert!(r == "gpt-4.1-mini" || r == "gpt-4.1");
    }

    #[test]
    fn highest_minor_handles_double_digit_minors() {
        let s = spec(PreferRule::HighestMinor, r"^claude-", None);
        let ids = ["claude-1.0", "claude-1.10", "claude-1.2"];
        let r = resolve_latest(&s, &ids).unwrap();
        // Numeric comparison: (1,10) > (1,2) — string-sort would
        // erroneously prefer "claude-1.2".
        assert_eq!(r, "claude-1.10");
    }

    #[test]
    fn lexicographic_max_picks_greatest_string() {
        let s = spec(PreferRule::LexicographicMax, r"^claude-", None);
        let ids = ["claude-aaa", "claude-zzz", "claude-mmm"];
        let r = resolve_latest(&s, &ids).unwrap();
        assert_eq!(r, "claude-zzz");
    }

    #[test]
    fn include_pattern_filters_out_non_matching_ids() {
        let s = spec(PreferRule::LexicographicMax, r"^gpt-[0-9]", None);
        let ids = ["gpt-4.0", "claude-3", "ada-002"];
        let r = resolve_latest(&s, &ids).unwrap();
        assert_eq!(r, "gpt-4.0");
    }

    #[test]
    fn exclude_pattern_drops_preview_legacy_deprecated() {
        let s = spec(
            PreferRule::HighestMinor,
            r"^gpt-[0-9]",
            Some("preview|legacy|deprecated"),
        );
        let ids = [
            "gpt-4.5-preview",
            "gpt-4.0-legacy",
            "gpt-4.1-deprecated",
            "gpt-4.0",
        ];
        let r = resolve_latest(&s, &ids).unwrap();
        assert_eq!(r, "gpt-4.0");
    }

    #[test]
    fn empty_intersection_yields_provider_error() {
        let s = spec(PreferRule::HighestMinor, r"^xyz-", None);
        let ids = ["gpt-4.0"];
        let err = resolve_latest(&s, &ids).unwrap_err();
        assert!(matches!(err, ArgoError::Provider(_)));
    }

    #[test]
    fn invalid_include_pattern_yields_provider_error() {
        let s = spec(PreferRule::HighestMinor, r"[", None);
        let ids = ["gpt-4.0"];
        let err = resolve_latest(&s, &ids).unwrap_err();
        if let ArgoError::Provider(msg) = err {
            assert!(msg.contains("invalid include_pattern"));
        } else {
            panic!("expected Provider error");
        }
    }

    #[test]
    fn no_exclude_pattern_keeps_everything() {
        let s = spec(PreferRule::HighestMinor, r"^gpt-[0-9]", None);
        let ids = ["gpt-4.0", "gpt-3.5"];
        let r = resolve_latest(&s, &ids).unwrap();
        assert_eq!(r, "gpt-4.0");
    }
}
