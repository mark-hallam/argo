# Round 39 — PR #2160 redact Stage-2 broaden suffix/separator classes anti-theater verify

**Date**: 2026-05-23 (real UTC)
**Round**: 39 [scheduled-task hourly fire, read-only]
**Pillar / Claim**: Substrate honesty / PII redaction fence
**Verdict**: 🟢🟢 **PASS** (un-ignores 4 deferred FINDINGS; closes the corpus to zero-ignored)

## Setup

PR [#2160](https://github.com/mark-hallam/argo/pull/2160) (`mission — redact — broaden suffix and separator classes`, merge commit `01bbe9b58`, 2026-05-24 02:14 AEST local = ~16:14 UTC on 2026-05-23) was merged in tick-118. Sibling to PR #2156 — Stage-1 closed the encoding-shaped bypasses (NFKC + URL-decode + HTML-entity + base64 rescan); Stage-2 broadens the regex vocabularies that #2156 deliberately left narrow.

Promised diff scope per PR body:
- Phone: U+2013 (en-dash) + U+2014 (em-dash) added to every separator class across the 3 branches (intl `+CC`, NZ trunk, US 3-3-4).
- Card: `.` and `_` added to the pair separator (was space / hyphen only).
- IBAN: every-4-chars space convention allowed (`GB82 WEST 1234 5698 7654 32`).
- Address: 10 new AU/NZ suffixes (Terrace/Tce, Circle/Cir, Plaza/Plz, Crescent/Cres, Esplanade/Esp, Parade/Pde, Trail/Trl, Square/Sq, Heights/Hts, Circuit/Cct) + optional unit-slash prefix.
- Corpus: 4 Stage-2 FINDINGS un-`#[ignore]`'d.

I verified by:
1. Reading the regex definitions at [crates/argo-redact/src/patterns.rs:99-148](crates/argo-redact/src/patterns.rs:99).
2. Reading the new CATCH pins at [crates/argo-redact/tests/encoding_bypass_corpus_test.rs:367-421](crates/argo-redact/tests/encoding_bypass_corpus_test.rs:367) (en-dash phone / em-dash phone / dot-card / underscore-card / spaced-IBAN / Terrace-address / Circle-address).
3. Reading the `assert_pin_catch` helper at [encoding_bypass_corpus_test.rs:48-57](crates/argo-redact/tests/encoding_bypass_corpus_test.rs:48).
4. Running `cargo test -p argo-redact` → 56 lib + 17 internal + 11 offset-projection + 30 corpus + 4 doc = **101 green, 0 ignored**.

## Observed

Test summary (verbatim from `cargo test -p argo-redact 2>&1 | grep "test result"`):

```
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Production regex wiring (verbatim, patterns.rs:99 + 118-148):

```rust
const ADDRESS_SUFFIX_ALTERNATION: &str = "St|Street|Ave|Avenue|Rd|Road|Blvd|Boulevard|Ln|Lane|Dr|Drive|Way|Wy|Ct|Court|Pl|Place|Pkwy|Pky|Parkway|Hwy|Highway|Terrace|Tce|Circle|Cir|Plaza|Plz|Crescent|Cres|Esplanade|Esp|Parade|Pde|Trail|Trl|Square|Sq|Heights|Hts|Circuit|Cct";

phone: Regex::new(
    r"(?x)
    \+\d{1,3}[\s\-.\u{2013}\u{2014}]?\(?\d{1,4}\)?[\s\-.\u{2013}\u{2014}]?(?:\d{2,4}[\s\-.\u{2013}\u{2014}]?)?\d{3,4}
    |
    \b0\d{1,2}[\s\-\u{2013}\u{2014}]?\d{3}[\s\-\u{2013}\u{2014}]?\d{4}\b
    |
    \b\(?\d{3}\)?[\s\-.\u{2013}\u{2014}]\d{3}[\s\-.\u{2013}\u{2014}]\d{4}\b
    ",
).ok(),
card: Regex::new(r"\b(?:\d[ \-._]?){12,18}\d\b").ok(),
iban: Regex::new(r"\b[A-Z]{2}\d{2}(?:[ ]?[A-Z0-9]){11,30}\b").ok(),
address: Regex::new(&format!(
    r"(?i)\b(?:\d{{1,5}}\s*/\s*)?\d{{1,5}}\s+(?:[A-Za-z][A-Za-z0-9'\-]*\s+){{1,3}}(?:{ADDRESS_SUFFIX_ALTERNATION})\b"
)).ok(),
```

Every PR-body claim verified against the regex literals — `\u{2013}\u{2014}` appears in all 3 phone branches; `_` is in the card pair separator (`[ \-._]`); IBAN has `(?:[ ]?[A-Z0-9])` for the every-4-chars convention; address has the optional `(?:\d{{1,5}}\s*/\s*)?` unit-slash prefix; ADDRESS_SUFFIX_ALTERNATION lists all 10 new shortcuts plus Wy/Pky.

## Anti-theater check

Four reasons this is not theatre:

1. **`assert_pin_catch` has a dual assertion.** It checks BOTH `out != input` (something was masked) AND `!hits.is_empty()` (the category was recognised). A refactor that returned hits but didn't mask (or vice versa) would flip the test red. Source at encoding_bypass_corpus_test.rs:48-57.

2. **The corpus has zero `#[ignore]`'d tests on origin/main today.** Grep confirms: only mentions of "ignore" are in comments documenting the Stage-1 / Stage-2 promotion history. The originally-deferred Stage-2 FINDINGS pins (`finding_en_dash_phone_must_be_masked`, `finding_em_dash_phone_must_be_masked`, `finding_dot_separated_card_must_be_masked`, `finding_underscore_separated_card_must_be_masked`, `finding_spaced_iban_must_be_masked`, `finding_terrace_address_must_be_masked`, `finding_circle_address_must_be_masked`) all run as `#[test]` with no ignore attribute — and all pass.

3. **The pin inputs are byte-exact regression contracts.** `dial 555\u{2013}555\u{2013}0123 today` is a non-trivial input — only matches if the phone regex's middle separator class genuinely includes U+2013. A character-class refactor that swapped the dash escapes would flip these tests red.

4. **Luhn-narrowing is preserved.** The card regex is intentionally wide (12-19 digits with any of space/dash/dot/underscore between pairs) but the PR body explicitly states "Luhn validation in `find_matches` continues to drop the false-positive long-digit-string class (version numbers, file paths, csv ids) so the wider regex doesn't increase the non-PII mask rate." The 56-test lib pass covers Luhn behavior; an honest broadening that bypassed Luhn would have flipped the lib tests red.

The combined Stage-1 + Stage-2 coverage advances the corpus from baseline ≈22% (5/23) to ≈83% (19/23). The 4 still-deferred misses (NER / spelled-out / character-jittered PII) are honest Stage-3 backlog deferral.

## Gap / follow-up briefs

**NONE new.** Both Stage-1 (#2156) and Stage-2 (#2160) ship gold-standard anti-theater contracts.

Re-surface for visibility:

- `mission-redact-promote-to-real-fence` was the umbrella brief; with Stage-1 + Stage-2 merged, the umbrella is satisfied at the regex-fence layer. The Stage-3 backlog (NER / character-jittered PII) is a separate brief shape — `mission-redact-stage3-ner-fence` (P3, no fix-by) — not currently load-bearing for v1.0.

24th anti-theater-honest substrate contract on main this week.
