//! Per-Forge-run cost attribution (MISSION_BACKLOG Section 6 #206).
//!
//! `ForgeRun.budget_usd_micros` caps spending but the **actual spend
//! per run isn't tracked** beyond the aggregate counter
//! `ForgeRun.spend_usd_micros`. Operators tuning Forge budget need
//! per-skill / per-vertical / per-tenant cost breakdowns so they can
//! identify which skills cost too much and which return value (Mission
//! Claim 4 — "skills shared and learned").
//!
//! This module exposes:
//!
//! - [`ForgeCost`] — the accumulator carried on every `ForgeRun`.
//! - [`PricingTable`] — hardcoded Anthropic + E2B v1 pricing snapshot.
//! - [`accumulate_llm_call`] / [`accumulate_e2b_seconds`] — pure
//!   helpers that mutate a `ForgeCost` in place. The Python DSPy/GEPA
//!   worker plugin calls these from its event-fold path before
//!   handing the finalized run back to the Rust caller.
//!
//! Pricing is hardcoded for v1 because Anthropic and E2B rates rarely
//! change and pulling from configurable manifests adds complexity for
//! marginal benefit. The pricing snapshot below is dated; a follow-on
//! brief (`mission-forge-provider-manifest-pricing`) will pull these
//! from `argo-provider` manifests.
//!
//! ## Pricing source / date stamp
//!
//! All USD-per-token rates were captured **2026-05-11** from public
//! pricing pages:
//!
//! | model id                     | input $/MTok | output $/MTok | cache-read $/MTok |
//! |------------------------------|-------------:|--------------:|------------------:|
//! | `claude-3-5-sonnet-20241022` |          3.0 |          15.0 |              0.30 |
//! | `claude-3-5-haiku-20241022`  |          0.8 |           4.0 |              0.08 |
//! | `claude-opus-4-20250514`     |         15.0 |          75.0 |              1.50 |
//! | `claude-opus-4-7`            |         15.0 |          75.0 |              1.50 |
//! | `claude-sonnet-4-6`          |          3.0 |          15.0 |              0.30 |
//! | `claude-haiku-4-5`           |          0.8 |           4.0 |              0.08 |
//!
//! E2B sandbox time is billed at **$0.00029/second** (E2B public
//! pricing 2026-05-11).
//!
//! Unknown model ids fall through to Sonnet-4-equivalent rates so a
//! mis-spelled model id doesn't silently report zero cost.
use serde::{Deserialize, Serialize};

/// Per-Forge-run cost attribution. Stamped onto the persisted
/// `forge_run` row on finalize. All cost figures are in USD-micros
/// (`1_000_000 = $1.00`) so they fit cleanly in `INTEGER` SQLite
/// columns alongside the existing `spend_usd_micros`.
///
/// ```
/// use argo_forge::cost::{accumulate_llm_call, ForgeCost};
/// let mut c = ForgeCost::default();
/// accumulate_llm_call(&mut c, "claude-sonnet-4-6", 1_000, 500, 0);
/// assert!(c.total_cost_usd_micros > 0);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgeCost {
    /// Cumulative LLM input tokens (uncached).
    pub llm_input_tokens: u64,
    /// Cumulative LLM output tokens.
    pub llm_output_tokens: u64,
    /// Cumulative LLM cache-read tokens. Cached reads are ~10x
    /// cheaper than uncached inputs, so this is tracked separately
    /// — a Forge run that hits cache heavily looks dramatically
    /// cheaper, and operators tuning prompt structure need this
    /// number to be honest.
    pub llm_cache_read_tokens: u64,
    /// Cumulative LLM cost (USD-micros). Computed from per-model
    /// pricing in [`pricing_for_model`] as calls accumulate.
    pub llm_cost_usd_micros: u64,
    /// Total E2B sandbox wall-clock seconds (rounded UP per
    /// [`accumulate_e2b_seconds`]). Zero when the run uses no
    /// sandbox.
    pub e2b_seconds: u64,
    /// Cumulative E2B sandbox cost (USD-micros).
    pub e2b_cost_usd_micros: u64,
    /// Sum of `llm_cost_usd_micros + e2b_cost_usd_micros`. Kept as
    /// a stamped column so the cost-report aggregator doesn't need
    /// to re-derive it on every read.
    pub total_cost_usd_micros: u64,
}

/// Per-model USD-per-million-token rates (input, output, cache_read).
/// All rates × `1_000_000 / 1_000_000_000_000` collapse to "USD-micros
/// per token" via [`PricingTable::cost_usd_micros_for_call`].
#[derive(Debug, Clone, Copy)]
pub struct PricingTable {
    /// Uncached input tokens, $/MTok.
    pub input_usd_per_mtok: f64,
    /// Output tokens, $/MTok.
    pub output_usd_per_mtok: f64,
    /// Cache-read tokens, $/MTok.
    pub cache_read_usd_per_mtok: f64,
}

impl PricingTable {
    /// Compute USD-micros for a single LLM call with the given token
    /// counts. Rounds half-up to the nearest USD-micro.
    #[must_use]
    pub fn cost_usd_micros_for_call(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
    ) -> u64 {
        // $/MTok × tokens / 1_000_000 = USD; ×1_000_000 = USD-micros.
        // Algebra collapses to ($/MTok) × tokens × 1 = USD-micros per
        // token-million... but we want per-token: ($/MTok × tokens) is
        // already USD-micros (since /MTok divides by 1e6 and ×$1.00 in
        // micros multiplies by 1e6).
        let input_um = self.input_usd_per_mtok * (input_tokens as f64);
        let output_um = self.output_usd_per_mtok * (output_tokens as f64);
        let cache_um = self.cache_read_usd_per_mtok * (cache_read_tokens as f64);
        let total = input_um + output_um + cache_um;
        // Round half-up; saturating cast covers wildly-large inputs.
        if total <= 0.0 {
            0
        } else if total >= (u64::MAX as f64) {
            u64::MAX
        } else {
            (total + 0.5) as u64
        }
    }
}

/// Look up the pricing snapshot for a given model id. Unknown ids
/// fall through to Sonnet-4-equivalent rates ($3/$15/$0.30) so a
/// mis-spelled model id is loud, not silent.
#[must_use]
pub fn pricing_for_model(model_id: &str) -> PricingTable {
    match model_id {
        "claude-3-5-haiku-20241022" | "claude-haiku-4-5" | "claude-haiku-4-5-20251001" => {
            PricingTable {
                input_usd_per_mtok: 0.8,
                output_usd_per_mtok: 4.0,
                cache_read_usd_per_mtok: 0.08,
            }
        }
        "claude-opus-4-20250514" | "claude-opus-4-7" | "claude-opus-4-7[1m]" => PricingTable {
            input_usd_per_mtok: 15.0,
            output_usd_per_mtok: 75.0,
            cache_read_usd_per_mtok: 1.5,
        },
        // Sonnet-tier (3.5-sonnet, 4.6-sonnet) and unknown fallback.
        _ => PricingTable {
            input_usd_per_mtok: 3.0,
            output_usd_per_mtok: 15.0,
            cache_read_usd_per_mtok: 0.30,
        },
    }
}

/// USD-micros per E2B sandbox second (2026-05-11 public rate: $0.00029/s).
pub const E2B_USD_MICROS_PER_SECOND: u64 = 290;

/// Accumulate one LLM provider call into a `ForgeCost`. Mutates in
/// place; updates token counters, the model-priced cost contribution,
/// and the cached `total_cost_usd_micros`.
pub fn accumulate_llm_call(
    cost: &mut ForgeCost,
    model_id: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
) {
    let pricing = pricing_for_model(model_id);
    let call_cost = pricing.cost_usd_micros_for_call(input_tokens, output_tokens, cache_read_tokens);
    cost.llm_input_tokens = cost.llm_input_tokens.saturating_add(input_tokens);
    cost.llm_output_tokens = cost.llm_output_tokens.saturating_add(output_tokens);
    cost.llm_cache_read_tokens = cost.llm_cache_read_tokens.saturating_add(cache_read_tokens);
    cost.llm_cost_usd_micros = cost.llm_cost_usd_micros.saturating_add(call_cost);
    cost.total_cost_usd_micros = cost
        .llm_cost_usd_micros
        .saturating_add(cost.e2b_cost_usd_micros);
}

/// Accumulate E2B sandbox wall-clock into a `ForgeCost`. Fractional
/// seconds are rounded UP to the nearest whole second so callers
/// don't game pricing by chopping milliseconds.
pub fn accumulate_e2b_seconds(cost: &mut ForgeCost, fractional_seconds: f64) {
    if fractional_seconds <= 0.0 {
        return;
    }
    let whole = fractional_seconds.ceil();
    let whole_u64 = if whole >= (u64::MAX as f64) {
        u64::MAX
    } else {
        whole as u64
    };
    cost.e2b_seconds = cost.e2b_seconds.saturating_add(whole_u64);
    let added_cost = whole_u64.saturating_mul(E2B_USD_MICROS_PER_SECOND);
    cost.e2b_cost_usd_micros = cost.e2b_cost_usd_micros.saturating_add(added_cost);
    cost.total_cost_usd_micros = cost
        .llm_cost_usd_micros
        .saturating_add(cost.e2b_cost_usd_micros);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulator_sums_correctly_over_multiple_calls() {
        let mut c = ForgeCost::default();
        accumulate_llm_call(&mut c, "claude-sonnet-4-6", 1_000, 500, 0);
        accumulate_llm_call(&mut c, "claude-sonnet-4-6", 2_000, 1_000, 0);
        assert_eq!(c.llm_input_tokens, 3_000);
        assert_eq!(c.llm_output_tokens, 1_500);
        // Sonnet: 3 $/MTok input × 3000 + 15 $/MTok output × 1500
        //       = 9_000 + 22_500 = 31_500 USD-micros
        assert_eq!(c.llm_cost_usd_micros, 31_500);
        assert_eq!(c.total_cost_usd_micros, 31_500);
    }

    #[test]
    fn cache_read_cheaper_than_uncached_input() {
        let pricing = pricing_for_model("claude-sonnet-4-6");
        let uncached = pricing.cost_usd_micros_for_call(1_000, 0, 0);
        let cached = pricing.cost_usd_micros_for_call(0, 0, 1_000);
        assert!(
            cached < uncached / 5,
            "cache-read should be at least 5x cheaper than uncached input: cached={cached}, uncached={uncached}"
        );
        // Sonnet cache-read: 0.30 $/MTok × 1000 = 300 USD-micros.
        assert_eq!(cached, 300);
        // Sonnet input: 3.0 $/MTok × 1000 = 3000 USD-micros.
        assert_eq!(uncached, 3_000);
    }

    #[test]
    fn e2b_seconds_rounded_up_to_nearest_whole_second() {
        let mut c = ForgeCost::default();
        accumulate_e2b_seconds(&mut c, 0.1);
        assert_eq!(c.e2b_seconds, 1);
        accumulate_e2b_seconds(&mut c, 2.4);
        assert_eq!(c.e2b_seconds, 4); // 1 + ceil(2.4)=3 = 4
        accumulate_e2b_seconds(&mut c, 0.0);
        assert_eq!(c.e2b_seconds, 4); // zero is a no-op
        assert_eq!(c.e2b_cost_usd_micros, 4 * E2B_USD_MICROS_PER_SECOND);
        assert_eq!(c.total_cost_usd_micros, 4 * E2B_USD_MICROS_PER_SECOND);
    }

    #[test]
    fn total_equals_sum_of_llm_and_e2b() {
        let mut c = ForgeCost::default();
        accumulate_llm_call(&mut c, "claude-sonnet-4-6", 1_000, 500, 0);
        accumulate_e2b_seconds(&mut c, 10.0);
        assert_eq!(
            c.total_cost_usd_micros,
            c.llm_cost_usd_micros + c.e2b_cost_usd_micros
        );
        assert!(c.llm_cost_usd_micros > 0);
        assert!(c.e2b_cost_usd_micros > 0);
    }

    #[test]
    fn zero_tokens_produce_zero_cost() {
        let mut c = ForgeCost::default();
        accumulate_llm_call(&mut c, "claude-sonnet-4-6", 0, 0, 0);
        assert_eq!(c.llm_input_tokens, 0);
        assert_eq!(c.llm_cost_usd_micros, 0);
        assert_eq!(c.total_cost_usd_micros, 0);
    }

    #[test]
    fn multi_call_accumulates_across_models_and_e2b() {
        let mut c = ForgeCost::default();
        // Opus call: 15/75 $/MTok.
        accumulate_llm_call(&mut c, "claude-opus-4-7", 1_000, 100, 0);
        let after_opus = c.llm_cost_usd_micros;
        assert_eq!(after_opus, 15_000 + 7_500); // 15 × 1000 + 75 × 100
        // Sonnet call: 3/15 $/MTok.
        accumulate_llm_call(&mut c, "claude-sonnet-4-6", 1_000, 100, 0);
        assert_eq!(c.llm_cost_usd_micros, after_opus + 3_000 + 1_500);
        // E2B contribution.
        accumulate_e2b_seconds(&mut c, 5.0);
        assert_eq!(c.e2b_seconds, 5);
        assert_eq!(c.e2b_cost_usd_micros, 5 * E2B_USD_MICROS_PER_SECOND);
        assert_eq!(
            c.total_cost_usd_micros,
            c.llm_cost_usd_micros + c.e2b_cost_usd_micros
        );
    }

    #[test]
    fn unknown_model_falls_through_to_sonnet_rates() {
        // Brief says "Unknown model ids fall through to
        // Sonnet-4-equivalent rates so a mis-spelled model id doesn't
        // silently report zero cost."
        let unknown = pricing_for_model("not-a-real-model-2099");
        let sonnet = pricing_for_model("claude-sonnet-4-6");
        assert!((unknown.input_usd_per_mtok - sonnet.input_usd_per_mtok).abs() < 1e-9);
        assert!((unknown.output_usd_per_mtok - sonnet.output_usd_per_mtok).abs() < 1e-9);
        assert!(
            (unknown.cache_read_usd_per_mtok - sonnet.cache_read_usd_per_mtok).abs() < 1e-9
        );
    }
}
