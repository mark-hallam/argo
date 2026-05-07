//! Edit-distance fuzzy matching for worker names.
//!
//! Whisper.cpp output is approximate (operator says "Maria",
//! transcription emits "Mariah"); raw substring match against the
//! pending-approval roster would miss. We use Levenshtein edit
//! distance ≤ 2 plus a substring fast-path: if the spoken argument
//! is a prefix or substring of the worker's full name (case-
//! insensitive), accept immediately.
//!
//! Tie-breaking: substring match wins over edit-distance match.
//! Among edit-distance candidates, the lowest distance wins; ties
//! at the same distance are broken by lexicographic order on the
//! candidate name (deterministic, no nondeterminism in tests).

/// Iterative Levenshtein edit distance (no allocation per row past
/// the initial pair of vectors). Insertion / deletion / substitution
/// each cost 1; transposition is NOT a single operation (we use
/// classical Levenshtein, not Damerau).
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr: Vec<usize> = vec![0; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (curr[j - 1] + 1)
                .min(prev[j] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}

/// Return the best worker-name candidate for `spoken`, scanning
/// `candidates`. Returns `None` if no candidate matches under the
/// substring + edit-distance ≤ `max_distance` rules.
///
/// `spoken` and each candidate are lowercased before comparison.
/// For multi-word candidate names, each word is tried independently
/// — that's how "Maria" matches "Maria Hernandez".
pub fn best_match<'a, I, S>(spoken: &str, candidates: I, max_distance: usize) -> Option<&'a str>
where
    I: IntoIterator<Item = &'a S>,
    S: AsRef<str> + 'a,
{
    let spoken = spoken.trim().to_lowercase();
    if spoken.is_empty() {
        return None;
    }
    let mut substr_hit: Option<&str> = None;
    let mut best_dist: Option<(usize, &str)> = None;
    for cand in candidates {
        let cand_full = cand.as_ref();
        let cand_lower = cand_full.to_lowercase();
        // Substring fast-path: full name OR any whitespace-split
        // word starts with / contains the spoken argument.
        if cand_lower.contains(&spoken) {
            // Tie-break across substring hits by lexicographic order
            // on the candidate (stable + deterministic).
            substr_hit = Some(match substr_hit {
                None => cand_full,
                Some(prev) if cand_full < prev => cand_full,
                Some(prev) => prev,
            });
            continue;
        }
        // Edit distance against the full name AND each word —
        // pick the smallest of those distances for this candidate.
        let mut min_d = edit_distance(&spoken, &cand_lower);
        for word in cand_lower.split_whitespace() {
            let d = edit_distance(&spoken, word);
            if d < min_d {
                min_d = d;
            }
        }
        if min_d <= max_distance {
            best_dist = Some(match best_dist {
                None => (min_d, cand_full),
                Some((prev_d, _)) if min_d < prev_d => (min_d, cand_full),
                Some((prev_d, prev)) if min_d == prev_d && cand_full < prev => {
                    (min_d, cand_full)
                }
                Some(prev) => prev,
            });
        }
    }
    substr_hit.or(best_dist.map(|(_, name)| name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_basic_distances() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("maria", "mariah"), 1);
    }

    #[test]
    fn substring_match_beats_edit_distance() {
        let names = ["Maria Hernandez".to_string(), "Marin Hayes".to_string()];
        // "Maria" is a substring of "Maria Hernandez" — that wins
        // even though "Marin" is also distance 1 from "Maria".
        let m = best_match("Maria", &names, 2);
        assert_eq!(m, Some("Maria Hernandez"));
    }

    #[test]
    fn edit_distance_two_accepted() {
        let names = ["Maria".to_string()];
        // "Mariah" vs "Maria" = 1 edit.
        assert_eq!(best_match("Mariah", &names, 2), Some("Maria"));
    }

    #[test]
    fn edit_distance_three_rejected() {
        let names = ["Maria".to_string()];
        // "Maaaaa" vs "Maria" = 4 edits — over the cap of 2.
        assert_eq!(best_match("Maaaaa", &names, 2), None);
    }

    #[test]
    fn no_candidate_returns_none() {
        let names: [String; 0] = [];
        assert_eq!(best_match("Maria", &names, 2), None);
    }

    #[test]
    fn multi_word_lookup_uses_first_word() {
        let names = ["Maria Hernandez".to_string()];
        // Spoken "Hernandez" matches the second word substring.
        assert_eq!(best_match("Hernandez", &names, 2), Some("Maria Hernandez"));
    }

    #[test]
    fn empty_spoken_returns_none() {
        let names = ["Maria".to_string()];
        assert_eq!(best_match("", &names, 2), None);
        assert_eq!(best_match("   ", &names, 2), None);
    }

    #[test]
    fn deterministic_tie_break_lex_order() {
        let names = ["Bob".to_string(), "Bib".to_string()];
        // Both at distance 1 from "Bob" (Bib differs by 1 char,
        // Bob is identical at distance 0). Identical wins.
        let m = best_match("Bob", &names, 2);
        assert_eq!(m, Some("Bob"));
    }
}
