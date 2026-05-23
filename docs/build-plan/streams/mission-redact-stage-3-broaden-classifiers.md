# mission-redact-stage-3-broaden-classifiers

**Owner**: claude.
**Depends on**: nothing (Stage 1 #2156 + Stage 2 #2160 both MERGED on origin/main).
**Authored against**: origin/main `ca02058c9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-redact-stage-3-broaden-classifiers`.

🟡 **P2 — Stage 3 PII coverage extension (the final 4 of 23 bypass misses).** Source: PR #2160 §"Coverage 14 → 19 of 23 (≈83%); 4 deferred misses (NER / spelled-out / character-jittered PII) are Stage-3 backlog" + PR #2156 §"combining-mark composition across char boundaries stays on the Stage 3 backlog".

Stage 1 (`mission-redact-promote-to-real-fence`, #2156) shipped the canonicalisation pre-pass (NFKC / invisible-strip / URL-decode / HTML-entity decode / base64 detect-and-rescan) and Stage 2 (`mission-redact-broaden-suffix-and-separator-classes`, #2160) broadened the regex vocabularies (en/em-dash phone separators, `.`/`_` card separators, IBAN 4-char-space, AU/NZ unit-slash address, expanded suffix vocab). Combined coverage is 19 of 23 corpus misses.

The remaining 4 misses sit at the limits of what regex-only redaction can catch:

1. **NER** — proper-name PII (`John Smith`, `Jane Doe`). Regex has no semantic anchor; needs a lightweight named-entity recogniser. Spec: ship a small dictionary-backed name detector under a feature flag (default off in v1), or wire a third-party NER crate behind a capability gate. Acceptance: at least the 6 most common first-name + 6 most common surname combinations in the corpus (`john_smith_should_be_masked` etc.) flip from MISS → CATCH.
2. **Spelled-out PII** — `four one two five five five one two three four` (a phone number written in English). Phrase-pattern detector that walks token windows and converts digit-words to digits before re-running the existing phone/card patterns.
3. **Character-jittered PII** — `j0hn.sm1th@example.com` (digit-letter substitution to evade dictionary lookup). Add a leet-speak normaliser to the Stage-1 canonicalisation pass (`0 → o`, `1 → l/i`, `3 → e`, `4 → a`, `5 → s`, `7 → t`) applied only inside `@`-bracketed local parts of the email regex, plus a span-level retry on the canonicalised local part. Must NOT apply globally (would mangle innocuous text).
4. **Combining-mark composition across char boundaries** — `émail` rendering as `émail`. Extend NFKC normalisation to use NFKC + canonical recomposition (NFC) BEFORE the existing canonicalisation pipeline so combining marks fold into single grapheme clusters.

The 4 corpus FINDINGS-ONLY tests are pre-baked `#[ignore]` in `crates/argo-redact/tests/encoding_bypass_corpus_test.rs` from Stage 1's authoring round — un-`#[ignore]` them as Stage-3 acceptance pins:
- `finding_ner_proper_name_must_be_masked`
- `finding_spelled_out_phone_must_be_masked`
- `finding_leetspeak_email_must_be_masked`
- `finding_combining_mark_email_must_be_masked`

## Scope

1. Extend `crates/argo-redact/src/canonicalize.rs` (or add `combining_mark.rs` sibling) with the NFC composition pass — must keep the offset-projection map clean (combining marks fold per-char or per-cluster; document the choice).
2. Add `crates/argo-redact/src/ner.rs` (~150-250 LOC) — dictionary-backed name detector. Source the dictionary from a small `static` array of the top 50 first names + top 50 surnames (US census + AU electoral roll subset is fine for v1); span-by-span lookup against canonicalised text.
3. Add `crates/argo-redact/src/spelled_out.rs` (~100-150 LOC) — token-window walker that converts English digit-words (`zero`/`one`/…/`nine` plus `oh`) to digits in a sliding 7-15 token window, then re-runs the existing phone/card regex on the converted span.
4. Add a leet-speak normaliser at the inside of the email-local-part match — applied per-match, not globally. Walk the original text + the leet-normalised text together; if leet-normalised matches an email pattern but original doesn't, the original is the leak.
5. Un-`#[ignore]` the 4 corpus FINDINGS tests in `tests/encoding_bypass_corpus_test.rs` and verify they go GREEN.
6. Add 3-5 new `CATCH` regression tests for each new classifier (8-15 new asserts total) to prevent the classifier from drifting.

## Files you may touch

- `crates/argo-redact/src/canonicalize.rs` — extend to NFC + combining-mark composition.
- `crates/argo-redact/src/ner.rs` — NEW (~150-250 LOC).
- `crates/argo-redact/src/spelled_out.rs` — NEW (~100-150 LOC).
- `crates/argo-redact/src/leet.rs` — NEW (~50-80 LOC) email-local-part normaliser.
- `crates/argo-redact/src/lib.rs` — wire new modules into the `redact_text` pipeline.
- `crates/argo-redact/src/redact.rs` — add the four new classifier calls in pipeline order (canonicalise → invisible-strip → URL/HTML decode → leet for email → spelled-out → existing regex pass → NER fallback).
- `crates/argo-redact/tests/encoding_bypass_corpus_test.rs` — un-`#[ignore]` the 4 FINDINGS tests; add CATCH regressions.
- `crates/argo-redact/tests/ner_classifier_test.rs` — NEW.
- `crates/argo-redact/tests/spelled_out_classifier_test.rs` — NEW.

## Files you must NOT touch

- `crates/argo-redact/src/patterns.rs` — Stage 2's regex vocab is locked; do not edit.
- The 7 gateway handlers that call `redact_text` — they are agnostic to the classifier set.

## Acceptance criteria

- [ ] All 4 corpus FINDINGS tests are un-`#[ignore]`'d and pass.
- [ ] `cargo test -p argo-redact` — 30 lib + 17 internal canonicalize + 11 offset-projection + 30 corpus + new NER/spelled-out/leet tests, all green.
- [ ] `cargo clippy -p argo-redact --all-targets -- -D warnings` clean.
- [ ] PII coverage 19 of 23 → 23 of 23 (100%).
- [ ] No regression in any existing CATCH test (the NER + leet must not introduce false positives that mask innocuous text).
- [ ] Each new module < 500 LOC.
- [ ] Eval report `eval-redact-crate-promote-from-regex-to-real-fence.md` updated with Stage-3 verdict + PR back-cite.

## Test plan

- Crate-scoped clippy + tests for `argo-redact`.
- Negative tests: pass innocuous text containing common names ("Apple smith jar"), spelled-out numbers in non-PII context ("we tested four configurations"), leetspeak in usernames ("h4x0r"), and combining marks in foreign-language text. None of these should be masked.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- ML-based NER (transformer/CRF) — v1 stays dictionary-backed.
- Multi-language PII coverage (only English + transliterated forms).
- Streaming-mode redaction — current pipeline is whole-document; streaming is a separate brief.
- Wiring a `cargo` feature flag to disable NER at compile time — operator decision, not author judgment.

~400-700 LOC across 4 new modules + corpus test updates + 2 new test files.
