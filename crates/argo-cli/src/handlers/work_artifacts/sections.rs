//! Markdown section splitter for worker output (#726).
//!
//! Replaces the old `extract_section` which built a lowercased
//! copy of the assistant text, found header substrings inside
//! that copy, then sliced the ORIGINAL text using lowercase-
//! byte indices. That approach broke in two ways:
//!   1. Lowercase byte indices don't always align with the
//!      original (Turkish `İ` → `i\u{0307}` is 2→3 bytes; ß
//!      may map to `ss`). Latent panic risk.
//!   2. Header detection used `str::find("## name")` which
//!      matched mid-line, and the `## ` "next-heading" stop
//!      walk ran an O(n²) byte sum to recompute offsets.
//!
//! The user observed the second issue concretely on a real
//! skate-resale run: `approval_checklist.md` ended up bleeding
//! the subsequent `## pricing` and `## listings` sections (and
//! their placeholder strings) into its body.
//!
//! ## New design — line-based, owned bodies
//!
//! 1. `text.lines()` walks line-by-line; every borrowed `&str`
//!    is on a guaranteed UTF-8 boundary. No manual byte
//!    indexing.
//! 2. A line is a header iff it matches `^#+\s+<name>` after
//!    trimming leading whitespace. Mid-line `## something`
//!    mentions are NOT treated as headers.
//! 3. Section bodies are accumulated as `Vec<&str>` and joined
//!    with `\n` only at flush time. Cheap: each section is a
//!    handful of lines.
//! 4. Name matching is token-based: the actual header name and
//!    the target candidate are both lowercased, split on any
//!    non-alphanumeric character (so `_`, `-`, `:`, `&`, emoji,
//!    whitespace all act as separators), and we match
//!    candidate-tokens against the prefix of actual-tokens.
//!
//! Examples:
//!
//! - `## Pricing & comp data` ↦ matches `pricing`
//! - `## approval_checklist` ↦ matches `approval_checklist`
//! - `## 📋 Approval` ↦ matches `approval`
//! - `## Section 1: Research` ↦ does NOT match `research` (target
//!   tokens must be the PREFIX of actual tokens, not appear later)

/// One captured section: the trimmed header name as written by
/// the model, plus the body text (joined with `\n`, no trailing
/// newline).
pub(super) type Section = (String, String);

/// Walk the text line-by-line and collect every section we see,
/// in document order. A new section starts at any line matching
/// `^#+\s+<name>`; everything between two header lines is the
/// preceding section's body.
pub(super) fn split_sections(text: &str) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    // (header_name, body_lines) for the section we're currently
    // accumulating into. None until we hit the first header.
    let mut current: Option<(String, Vec<&str>)> = None;
    for line in text.lines() {
        if let Some(name) = parse_header_line(line) {
            if let Some((n, body_lines)) = current.take() {
                out.push((n, body_lines.join("\n")));
            }
            current = Some((name, Vec::new()));
        } else if let Some((_, body_lines)) = current.as_mut() {
            body_lines.push(line);
        }
    }
    if let Some((n, body_lines)) = current {
        out.push((n, body_lines.join("\n")));
    }
    out
}

/// Find the FIRST section whose header matches any candidate
/// (candidates tried in order — earlier candidates are more
/// specific, e.g. `approval_checklist` before plain `approval`).
/// Returns the trimmed body, or `None` if no section matches or
/// the matched section has empty body.
pub(super) fn find_section(sections: &[Section], candidates: &[&str]) -> Option<String> {
    for cand in candidates {
        for (name, body) in sections {
            if name_matches(name, cand) {
                let trimmed = body.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

/// Return `Some(name)` when `line` starts with `#+\s+` and has
/// a non-empty trimmed remainder, else `None`. The number of
/// leading hashes isn't constrained — `#`, `##`, `###`, `####`
/// are all treated as section markers (subsections of the same
/// run still count as a section boundary, which is what we
/// want for "no bleed-through").
fn parse_header_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let after_hashes = trimmed.trim_start_matches('#');
    // Must be `#+ ` (one or more hashes then whitespace), not
    // bare `#####` or `#word`.
    if !after_hashes.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    let name = after_hashes.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// `actual` (a header name as written by the model) matches
/// `target` (one of our canonical names) when the target's
/// tokens are a prefix of the actual's tokens. See module-level
/// docs for examples.
fn name_matches(actual: &str, target: &str) -> bool {
    let a = tokens(actual);
    let t = tokens(target);
    if t.is_empty() || a.len() < t.len() {
        return false;
    }
    a[..t.len()] == t
}

/// Tokenise: lowercase, split on any non-alphanumeric character
/// (so `_`, `-`, `:`, `&`, whitespace, emoji all separate).
fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The user's exact reported bug — model emits all four
    /// sections in order; pre-#726 the splitter bled the
    /// `## pricing` and `## listings` content (and their
    /// placeholder strings) into `approval_checklist.md`.
    /// Each section must extract independently with NO
    /// cross-contamination.
    #[test]
    fn four_sections_split_independently_no_bleed_through() {
        let body = "\
preamble that should not be captured

## research
finding from web_search.\nSource: https://example.com.

## pricing
| AU comp | $250 |
| AU comp 2 | $300 |

## listings
**Title:** ProSkates 110mm

**Body:** lightly used, AU.

## approval_checklist
- [ ] confirm reserve price
- [ ] confirm shipping budget

[STOP: APPROVAL_REQUIRED]
";
        let sections = split_sections(body);
        let r = find_section(&sections, &["research"]).expect("research");
        let p = find_section(&sections, &["pricing"]).expect("pricing");
        let l = find_section(&sections, &["listings", "listing"]).expect("listings");
        let a = find_section(
            &sections,
            &["approval_checklist", "approval checklist", "approval"],
        )
        .expect("approval_checklist");

        assert!(r.contains("finding from"));
        assert!(p.contains("$250"));
        assert!(l.contains("ProSkates"));
        assert!(a.contains("reserve price"));

        // CRITICAL: the bug we're fixing — approval body must
        // NOT bleed pricing/listings content.
        assert!(!a.contains("$250"), "approval bled pricing into its body");
        assert!(
            !a.contains("ProSkates"),
            "approval bled listings into its body"
        );
        // And — symmetric — pricing must not bleed listings, etc.
        assert!(!p.contains("ProSkates"));
        assert!(!l.contains("reserve price"));
    }

    /// Direct shape of the user's report: `approval_checklist`
    /// first in the doc, with `## pricing` and `## listings`
    /// IMMEDIATELY following (no body lines between). Exercises
    /// the "consecutive headers" path through split_sections.
    #[test]
    fn consecutive_headers_terminate_each_other_cleanly() {
        let body = "## approval_checklist\n## pricing\nbody1\n## listings\nbody2\n";
        let sections = split_sections(body);
        // 3 sections, in order, the first one with EMPTY body.
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].0, "approval_checklist");
        assert!(
            sections[0].1.is_empty(),
            "empty body when next line is another header"
        );
        assert_eq!(sections[1].0, "pricing");
        assert!(sections[1].1.contains("body1"));
        assert_eq!(sections[2].0, "listings");
        assert!(sections[2].1.contains("body2"));

        // Empty approval_checklist body → find_section returns None.
        assert!(find_section(
            &sections,
            &["approval_checklist", "approval checklist", "approval"]
        )
        .is_none());
        // pricing/listings still extract.
        assert!(find_section(&sections, &["pricing"]).is_some());
        assert!(find_section(&sections, &["listings", "listing"]).is_some());
    }

    /// Emoji / unicode in headers — `## 📋 approval_checklist`,
    /// `## 💰 Pricing`, `## ✨ listings`. tokens() filters
    /// non-alphanumeric, so emoji acts as a separator and the
    /// real word(s) still match.
    #[test]
    fn header_with_emoji_prefix_still_matches() {
        let body = "## 📋 approval_checklist\n- [ ] check it\n## 💰 Pricing\n$100\n## ✨ listings\nad copy\n";
        let sections = split_sections(body);
        let a = find_section(&sections, &["approval_checklist", "approval"])
            .expect("approval w/ emoji prefix");
        assert!(a.contains("check it"));
        assert!(find_section(&sections, &["pricing"])
            .unwrap()
            .contains("$100"));
        assert!(find_section(&sections, &["listings"])
            .unwrap()
            .contains("ad copy"));
    }

    /// Unicode in the body (CJK, emoji, currency) round-trips
    /// without truncation or panic. The body is captured as
    /// owned `String`s built from `Vec<&str>::join`, so no byte
    /// slicing happens.
    #[test]
    fn unicode_in_section_body_is_preserved() {
        let body = "## research\n商品研究: 価格は¥10,000\nemoji: 📋 ✅\n";
        let sections = split_sections(body);
        let r = find_section(&sections, &["research"]).expect("research");
        assert!(r.contains("¥10,000"));
        assert!(r.contains("商品研究"));
        assert!(r.contains("📋"));
    }

    /// Case-insensitive matching: `## RESEARCH`, `## Pricing`,
    /// `## LISTINGS`, `## Approval Checklist` all match.
    #[test]
    fn case_insensitive_header_matching() {
        let body = "## RESEARCH\nfinding\n## Pricing\n$100\n## LISTINGS\nad\n## Approval Checklist\nchecklist\n";
        let sections = split_sections(body);
        assert!(find_section(&sections, &["research"]).is_some());
        assert!(find_section(&sections, &["pricing"]).is_some());
        assert!(find_section(&sections, &["listings", "listing"]).is_some());
        assert!(find_section(
            &sections,
            &["approval_checklist", "approval checklist", "approval"],
        )
        .is_some());
    }

    /// Bare `## approval` — when the model uses just that
    /// instead of `approval_checklist` — must still surface
    /// into the approval_checklist field (per #726 spec).
    #[test]
    fn bare_approval_header_matches_approval_checklist_target() {
        let body = "## approval\n- [ ] sign-off\n";
        let sections = split_sections(body);
        let a = find_section(
            &sections,
            &["approval_checklist", "approval checklist", "approval"],
        )
        .expect("bare approval header should match the approval_checklist target");
        assert!(a.contains("sign-off"));
    }

    /// Underscore vs space in `approval_checklist`: both map to
    /// the same tokens after splitting on non-alphanumerics.
    #[test]
    fn approval_checklist_underscore_or_space_both_match() {
        let cands = &["approval_checklist", "approval checklist", "approval"];
        let s1 = split_sections("## approval_checklist\nbody1\n");
        assert!(find_section(&s1, cands).unwrap().contains("body1"));
        let s2 = split_sections("## approval checklist\nbody2\n");
        assert!(find_section(&s2, cands).unwrap().contains("body2"));
    }

    /// Headers with trailing qualifiers (`## Pricing & AU comp`,
    /// `## Research findings`) — target tokens are a PREFIX of
    /// actual tokens, so they still match.
    #[test]
    fn header_with_trailing_qualifiers_still_matches() {
        let body =
            "## Pricing & comparable AU listings\ncomp data\n## Research findings\nresearch text\n";
        let sections = split_sections(body);
        assert!(find_section(&sections, &["pricing"])
            .unwrap()
            .contains("comp data"));
        assert!(find_section(&sections, &["research"])
            .unwrap()
            .contains("research text"));
    }

    /// Inline `## research` mid-paragraph must NOT be treated
    /// as a header. Only line-start hashes count.
    #[test]
    fn inline_hashes_mid_line_not_a_header() {
        let body = "Random sentence with ## research mention inline.\nNo header here.\n";
        let sections = split_sections(body);
        assert!(sections.is_empty(), "no header lines → no sections");
    }

    /// Different heading depths (`#`, `##`, `###`) all act as
    /// section boundaries.
    #[test]
    fn matches_any_heading_depth() {
        let s1 = split_sections("# pricing\nsingle hash body\n");
        assert!(find_section(&s1, &["pricing"])
            .unwrap()
            .contains("single hash body"));
        let s2 = split_sections("### pricing\ntriple hash body\n");
        assert!(find_section(&s2, &["pricing"])
            .unwrap()
            .contains("triple hash body"));
    }

    /// `parse_header_line` rejects bare `#####`, `#nospace`, and
    /// `## ` (header with no name).
    #[test]
    fn parse_header_line_rejects_invalid_shapes() {
        assert_eq!(parse_header_line(""), None);
        assert_eq!(parse_header_line("#"), None);
        assert_eq!(parse_header_line("##"), None);
        assert_eq!(parse_header_line("####"), None);
        assert_eq!(parse_header_line("#word"), None); // no space
        assert_eq!(parse_header_line("## "), None); // empty name
                                                    // Valid:
        assert_eq!(
            parse_header_line("## research").as_deref(),
            Some("research")
        );
        assert_eq!(
            parse_header_line("###  pricing  ").as_deref(),
            Some("pricing")
        );
        assert_eq!(
            parse_header_line("  ## listings"),
            Some("listings".to_string())
        );
    }

    /// More-specific candidates win when both exist. Document
    /// has `## approval` AND `## approval_checklist`; the
    /// candidate-list-order means we prefer the more specific
    /// `approval_checklist` body.
    #[test]
    fn prefers_more_specific_candidate_when_both_present() {
        let body = "## approval\nbody-of-bare-approval\n## approval_checklist\nbody-of-checklist\n";
        let sections = split_sections(body);
        let a = find_section(
            &sections,
            &["approval_checklist", "approval checklist", "approval"],
        )
        .expect("must find one");
        assert!(a.contains("body-of-checklist"));
        assert!(!a.contains("body-of-bare-approval"));
    }
}
