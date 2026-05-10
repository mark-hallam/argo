//! Pure statistical helpers for `ab_test` — no DB, no IO.
//!
//! Houses the SHA-256 deterministic 50/50 split, the chi-square
//! significance test on a 2×2 contingency table, and the value
//! types they share. Lives in a sibling file so the main module
//! stays under the per-file 500-LOC budget; re-exported via
//! `pub use` so callers see the same `ab_test::*` surface.
//!
//! The chi-square / 2-prop z-test we use here is a textbook
//! Pearson statistic with Yates continuity correction so the
//! small-sample case doesn't explode. For α=0.05 with df=1 the
//! critical value is **3.841**.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Critical chi-square value for df=1, α=0.05 — used as the
/// significance threshold.
pub const CHI_SQUARE_CRIT_DF1_ALPHA_05: f64 = 3.841;

/// Default minimum sample per variant before significance is
/// computed. Below this, we never declare a winner — the test
/// statistic isn't reliable on tiny samples.
pub const DEFAULT_MIN_SAMPLES_PER_VARIANT: i64 = 30;

/// Variant identifier — A or B. We carry the operator-supplied
/// version string (e.g. `"v2"`) so the report references the same
/// label the human typed.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    A,
    B,
}

#[allow(missing_docs)]
impl Variant {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
        }
    }
}

/// Per-variant counts pulled from the outcome table.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantCounts {
    pub trials: i64,
    pub successes: i64,
}

/// Significance-check verdict at this moment.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Significance {
    pub chi_square: f64,
    pub significant: bool,
    pub a: VariantCounts,
    pub b: VariantCounts,
}

/// Deterministic 50/50 split on the invocation id. Hash → high bit
/// → A or B. The same `(run_id, invocation_id)` always picks the
/// same variant, so retries don't shuffle the assignment.
#[must_use]
pub fn select_variant(run_id: &str, invocation_id: &str) -> Variant {
    let mut hasher = Sha256::new();
    hasher.update(run_id.as_bytes());
    hasher.update(b"::");
    hasher.update(invocation_id.as_bytes());
    let digest = hasher.finalize();
    if digest[0] & 0x01 == 0 {
        Variant::A
    } else {
        Variant::B
    }
}

/// Pearson chi-square on a 2×2 contingency table with Yates
/// continuity correction. Conservative for small samples — refuses
/// to declare significance below `min_samples_per_variant`.
#[must_use]
pub fn significance(
    a: VariantCounts,
    b: VariantCounts,
    min_samples_per_variant: i64,
) -> Significance {
    if a.trials < min_samples_per_variant || b.trials < min_samples_per_variant {
        return Significance {
            chi_square: 0.0,
            significant: false,
            a,
            b,
        };
    }
    let na = a.trials as f64;
    let nb = b.trials as f64;
    let sa = a.successes as f64;
    let sb = b.successes as f64;
    let fa = na - sa;
    let fb = nb - sb;
    let total = na + nb;
    if total <= 0.0 {
        return Significance {
            chi_square: 0.0,
            significant: false,
            a,
            b,
        };
    }
    // Expected counts under H_0 (rates equal).
    let row_succ = sa + sb;
    let row_fail = fa + fb;
    let exp_a_succ = na * row_succ / total;
    let exp_a_fail = na * row_fail / total;
    let exp_b_succ = nb * row_succ / total;
    let exp_b_fail = nb * row_fail / total;
    // Yates correction term — bias guard for small samples.
    let yates_term = |obs: f64, exp: f64| -> f64 {
        if exp <= 0.0 {
            return 0.0;
        }
        let diff = (obs - exp).abs() - 0.5;
        let diff = diff.max(0.0);
        diff * diff / exp
    };
    let chi = yates_term(sa, exp_a_succ)
        + yates_term(fa, exp_a_fail)
        + yates_term(sb, exp_b_succ)
        + yates_term(fb, exp_b_fail);
    Significance {
        chi_square: chi,
        significant: chi >= CHI_SQUARE_CRIT_DF1_ALPHA_05,
        a,
        b,
    }
}
