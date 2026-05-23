# Round 39 — PR #2156 redact Stage-1 canonicalisation pre-pass anti-theater verify

**Date**: 2026-05-23 (real UTC)
**Round**: 39 [scheduled-task hourly fire, read-only]
**Pillar / Claim**: Substrate honesty / PII redaction fence (cross-cutting; supports Claim 6 multi-tenant-by-construction)
**Verdict**: 🟢🟢 **PASS** (real seams, negative controls, projection invariant pinned)

## Setup

PR [#2156](https://github.com/mark-hallam/argo/pull/2156) (`redact: Stage-1 canonicalisation pre-pass + offset projection`, merge commit `f204a9942`, 2026-05-24 01:49 AEST local = ~15:49 UTC on 2026-05-23) was merged in tick-117. The PR's load-bearing claim: closes 14 of 23 PII bypass misses identified by `encoding_bypass_corpus_test`, including the fullwidth-digit PAN leak that v1 Luhn dropped because `char::to_digit(10)` rejects non-ASCII digits.

I verified by:
1. Reading the new module at [crates/argo-redact/src/canonicalize.rs:1-88](crates/argo-redact/src/canonicalize.rs) — `Canonical` struct + `canonicalize_for_scan` + offset projection.
2. Reading the production wiring at [crates/argo-redact/src/redact.rs:78-95](crates/argo-redact/src/redact.rs:78) — `redact_text_inner` calls `canonicalize_for_scan` then `find_matches` on canonical, then projects every match span back via `canonical.original_span` before applying masks.
3. Reading the integration tests at [crates/argo-redact/tests/canonicalize_offset_projection_test.rs](crates/argo-redact/tests/canonicalize_offset_projection_test.rs) — 11 tests across positive + negative paths.
4. Running `cargo test -p argo-redact --test canonicalize_offset_projection_test` and `cargo test -p argo-redact --test encoding_bypass_corpus_test` → 11/11 + 30/30 green; zero `#[ignore]`'d tests remain in the bypass corpus.

## Observed

```
running 11 tests
test base64_email_token_masks_whole_token_inplace ... ok
test html_entity_at_masks_five_byte_entity ... ok
test base64_without_pii_is_not_masked ... ok
test multiple_canonicalisations_in_one_input_all_caught ... ok
test url_encoded_at_masks_three_byte_escape ... ok
test zero_width_space_strip_keeps_unaffected_bytes ... ok
test soft_hyphen_strip_keeps_unaffected_bytes ... ok
test fullwidth_at_preserves_surrounding_original_bytes ... ok
test invalid_url_escape_passes_through ... ok
test canonicalisation_does_not_break_plain_ascii_path ... ok
test fullwidth_digit_card_passes_luhn_after_canonicalisation ... ok
test result: ok. 11 passed; 0 failed; 0 ignored
```

Production wiring (verbatim, redact.rs:78-95):

```rust
fn redact_text_inner(text: &str, policy: &RedactionPolicy, depth: u8) -> (String, Vec<Category>) {
    let canonical = canonicalize_for_scan(text);
    let mut hits = find_matches(&canonical.text);
    if depth == 0 {
        hits.extend(base64_rescan_hits(&canonical.text, policy));
    }
    let projected: Vec<PatternMatch> = hits
        .into_iter()
        .map(|m| {
            let orig = canonical.original_span(m.span.clone());
            PatternMatch { category: m.category, span: orig }
        })
        .collect();
    apply_masks(text, projected, policy)
}
```

The bypass corpus on origin/main currently shows `0 ignored` — all 6 Stage-1 FINDINGS-only tests that PR #2126 originally `#[ignore]`'d have been promoted to active CATCH pins per the PR body's "Un-ignores the 6 Stage-1 FINDINGS tests" claim. The 4 Stage-2 FINDINGS pins were closed in sibling PR #2160 (separate report).

## Anti-theater check

Five reasons this is not theatre:

1. **Offset-projection invariant has a dedicated stress test.** `fullwidth_at_preserves_surrounding_original_bytes` doesn't just assert `out.contains("[EMAIL]")` — it ALSO asserts `out.starts_with("ping ")`, `out.ends_with(" today")`, AND the absence of `\u{FF20}`, `sam`, and `acmeco` in the masked output. A refactor that masked the wrong span (the canonical ASCII bytes instead of the original fullwidth bytes) would flip the test red because the leftover original bytes would leak into the output.

2. **Two genuine negative controls.** `base64_without_pii_is_not_masked` proves the base64-rescan doesn't false-positive on benign 16-char tokens (e.g. cache keys, IDs). `invalid_url_escape_passes_through` proves `100% off` doesn't get URL-decoded into garbage. Without these the canonicalisation pipe could mask everything and still pass positive cases.

3. **The fullwidth-card test pins the exact v1 failure mode.** `fullwidth_digit_card_passes_luhn_after_canonicalisation` constructs PAN `4111-1111-1111-1111` entirely in fullwidth (U+FF14, U+FF11) — the test docstring spells out "`char::to_digit(10)` rejects fullwidth, so the v1 Luhn check would have dropped the candidate". The test would have been red on v1; it's green only because the NFKC pass at `nfkc_strip` (lines 80+) folds fullwidth → ASCII before Luhn runs.

4. **`multiple_canonicalisations_in_one_input_all_caught` exercises composition.** Three different canonicalisation paths (fullwidth `@`, URL-encoded `%40`, HTML entity `&#64;`) in a single input must ALL get caught and emit exactly 3 `[EMAIL]` sentinels. Composition bugs in `compose` (where offset maps from successive passes chain) would either drop a hit or double-mask.

5. **Round-trip sanity pin.** `canonicalisation_does_not_break_plain_ascii_path` asserts the v1 happy path (`Contact me at sam@acmeco.com or call 027 555 1234`) still produces exactly `Contact me at [EMAIL] or call [PHONE]` byte-for-byte. A bug in the identity short-circuit or offset bookkeeping would shift bytes around the sentinel and flip this.

The Canonical struct's `offsets: Vec<usize>` carries one entry per canonical byte plus a trailing entry at `text.len()` (line 30-33 docstring) — the half-open range projection at lines 36-45 is correct by construction for non-overlapping passes.

## Gap / follow-up briefs

**NONE new.** The 4 still-deferred bypass misses (NER / spelled-out / character-jittered PII) are honest Stage-3 backlog deferral per PR body — combined Stage-1 + Stage-2 coverage now ≈83% (19 of 23) up from baseline ≈22% (5 of 23).

Re-surface for visibility:

- `pin_email_combining_acute_in_local_is_missed` still pinned as MISS — per `nfkc_strip` docstring (line 76-79), combining-mark composition that crosses char boundaries (e.g. `a` + U+0301 → `á`) is deliberately NOT performed because it would require relaxing the per-char NFKC invariant the offset map depends on. Honest tradeoff; flagged for Stage-3 NER pass.

23rd anti-theater-honest substrate contract on main this week.
