# mission-redact-broaden-suffix-and-separator-classes

**Authored against:** `origin/main` e92343c85 on 2026-05-23.
**Validate before claim:** `bash scripts/check-brief-freshness.sh mission-redact-broaden-suffix-and-separator-classes`.

## Why this brief exists

Surfaced as Stage 2 fix-brief from PR #2126 (`eval-redact-crate-promote-from-regex-to-real-fence`, IN_REVIEW 2026-05-23). After Stage 1 (`mission-redact-promote-to-real-fence`) lands the canonicalisation pre-pass, 5 of the remaining 9 corpus misses are still leaks because the regex vocabulary itself is too narrow:

- Address suffixes: regex catches `Street / Avenue / Road / Drive` but not `Terrace / Circle / Court / Way / Plaza / Crescent / Lane / Esplanade` (and AU-specific suffixes).
- Phone separators: regex catches ASCII `-` but not en-dash (U+2013) / em-dash (U+2014).
- Card separators: regex catches space + ASCII hyphen but not `.` or `_`.
- AU/NZ unit-slash address form: `5/123 Main St` partial-leaks because the regex treats the slash as a non-address character.
- (Luhn-on-NFKC for fullwidth-digit cards is in Stage 1 — DO NOT reimplement here.)

**DEPENDS ON**: `mission-redact-promote-to-real-fence` MERGED first. This brief assumes canonicalisation pre-pass is in place — the new regexes scan the canonicalised text, not raw input. **BLOCKED until Stage 1 lands.**

## Acceptance criteria

1. Extend the address-suffix vocabulary in `crates/argo-redact/src/` to include at minimum: `Terrace, Circle, Court, Way, Plaza, Crescent, Lane, Esplanade, Parade, Boulevard, Pkwy, Pky, Trail, Pl, Pkwy`. Confirm against PR #2126's report — the worker may broaden further if the corpus calls for it.
2. Extend phone-separator regex to include `en-dash` (U+2013) and `em-dash` (U+2014).
3. Extend card-separator regex to include `.` and `_` between groups of digits — but only when the resulting digit-stream passes Luhn (avoid false positives on version numbers, file paths, etc.).
4. Add AU/NZ unit-slash address form: `\d+\s*/\s*\d+\s+[A-Z][a-z]+\s+(<suffix-vocab>)`.
5. Un-`#[ignore]` the remaining Stage-2 FINDINGS-ONLY tests in `crates/argo-redact/tests/encoding_bypass_corpus_test.rs` (worker re-confirms the list against the file at claim time; should be ~5 tests).
6. Per-crate `cargo clippy -p argo-redact -- -D warnings` clean. Workspace clippy fast-fail clean.

## Parallel-surface declarations

- `rg 'pub fn (redact_text|canonicalize)' crates/argo-redact/` — extends Stage 1's surfaces; no new pub fn.
- Wire-up consumer: same 7 gateway handlers, unchanged.

## May-touch

- `crates/argo-redact/src/lib.rs` (or its split modules from Stage 1)
- `crates/argo-redact/tests/encoding_bypass_corpus_test.rs` (un-`#[ignore]` Stage-2 tests)

## Boundary

- BLOCKED until `mission-redact-promote-to-real-fence` MERGED. Conductor: do not flip this row to TODO until Stage 1 is on main.
- Do NOT touch NER / spelled-out / character-jittered PII — that's Stage 3 (DEFERRED-v1.x).
- Do NOT touch the gateway handlers.

## Sizing

~150-250 LOC (regex extensions + suffix table + unit-slash form + Luhn-aware card extension).

**🟡 P2 (PII-LEAK CONTAINMENT, BLOCKED on Stage 1)** — closes 5 more bypass misses (corpus coverage 14 → 19 of 23 misses after Stage 1+2; combined ~87% from baseline 23%).
