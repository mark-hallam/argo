//! Pure tokenizer + IDF-weighted Jaccard scorer for `canonical_qa`.
//!
//! Kept in a sibling file so `canonical_qa.rs` stays under the
//! 500-LOC core-platform ceiling. No state outside the inputs — every
//! function is referentially transparent and `#[cfg(test)]`-friendly.

use std::collections::{HashMap, HashSet};

/// Split a string into a deduplicated set of lowercase content
/// tokens — alphanumeric runs ≥2 chars, with common English
/// stopwords dropped. Pure function; safe to call per-lookup.
///
/// ```
/// use argo_skill::canonical_qa_score::tokenize;
/// let toks = tokenize("How is Skycot different from Lindy?");
/// assert!(toks.contains("skycot"));
/// assert!(toks.contains("lindy"));
/// assert!(!toks.contains("is"));
/// ```
#[must_use]
pub fn tokenize(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2 && !is_stopword(t))
        .map(str::to_string)
        .collect()
}

fn is_stopword(t: &str) -> bool {
    // English stopwords + interrogatives the matcher ignores so that
    // "How do you compare to Lindy?" and "What's the difference
    // between you and Lindy?" surface "lindy" + "compare" /
    // "difference" as the load-bearing tokens.
    matches!(
        t,
        "the" | "an" | "is" | "are" | "was" | "were" | "be" | "been" | "being"
        | "have" | "has" | "had" | "do" | "does" | "did"
        | "to" | "of" | "in" | "on" | "at" | "by" | "for" | "from" | "with"
        | "and" | "or" | "but" | "if" | "as" | "it" | "its"
        | "this" | "that" | "these" | "those"
        | "you" | "your" | "we" | "our" | "us" | "me" | "my"
        | "so" | "such" | "any" | "all" | "no" | "not"
        | "than" | "then" | "there" | "here" | "into" | "out" | "up" | "down" | "about"
        | "what" | "how" | "why" | "when" | "where" | "who"
    )
}

/// Build the IDF table over a flat slice of token sets. Smoothed
/// `log((N + 1) / (df + 0.5))` so rare tokens (proper nouns like
/// "lindy") dominate the score and stopword-adjacent fillers drop
/// out.
#[must_use]
pub fn build_idf(prompt_token_sets: &[HashSet<String>]) -> HashMap<String, f32> {
    let n = prompt_token_sets.len().max(1) as f32;
    let mut df: HashMap<&str, usize> = HashMap::new();
    for set in prompt_token_sets {
        for tok in set {
            *df.entry(tok.as_str()).or_insert(0) += 1;
        }
    }
    df.into_iter()
        .map(|(tok, c)| (tok.to_string(), ((n + 1.0) / (c as f32 + 0.5)).ln()))
        .collect()
}

/// Score one query against one prompt. IDF-weighted query-recovery —
/// `sum(idf for query tokens also in prompt) / sum(idf for query tokens)`
/// — normalized to `[0.0, 1.0]`.
///
/// Asymmetric on purpose: rewards prompts that *explain the query*
/// without penalizing prompts that happen to carry extra context.
/// Symmetric scoring (Jaccard) over-penalizes when a prompt contains
/// a high-IDF token not in the query — "How is Skycot **different**
/// from Lindy?" against "Skycot versus Lindy?" wouldn't clear the
/// threshold otherwise.
#[must_use]
pub fn score(
    query: &HashSet<String>,
    prompt: &HashSet<String>,
    idf: &HashMap<String, f32>,
) -> f32 {
    if query.is_empty() || prompt.is_empty() {
        return 0.0;
    }
    let oov = oov_idf(idf);
    let mut covered = 0.0_f32;
    let mut total = 0.0_f32;
    for tok in query {
        let w = *idf.get(tok).unwrap_or(&oov);
        total += w;
        if prompt.contains(tok) {
            covered += w;
        }
    }
    if total <= f32::EPSILON {
        0.0
    } else {
        covered / total
    }
}

/// Idf weight assumed for out-of-vocab query tokens — same shape as
/// `build_idf` with df = 0. Tokens unseen at corpus-load time still
/// need *some* weight or the matcher silently ignores them.
fn oov_idf(idf: &HashMap<String, f32>) -> f32 {
    let max_seen = idf.values().copied().fold(f32::NEG_INFINITY, f32::max);
    if max_seen.is_finite() {
        max_seen
    } else {
        1.0
    }
}
