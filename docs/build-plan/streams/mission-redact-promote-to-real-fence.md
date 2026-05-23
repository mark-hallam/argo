# mission-redact-promote-to-real-fence

**Authored against:** `origin/main` e92343c85 on 2026-05-23.
**Validate before claim:** `bash scripts/check-brief-freshness.sh mission-redact-promote-to-real-fence`.

## Why this brief exists

Surfaced as Stage 1 fix-brief from PR #2126 (`eval-redact-crate-promote-from-regex-to-real-fence`, IN_REVIEW 2026-05-23). The new black-box bypass corpus at `crates/argo-redact/tests/encoding_bypass_corpus_test.rs` (488 LOC, 30 cases) demonstrates that `argo-redact`'s v1 regex layer leaks PII through 23 of 30 encoding-shaped bypass payloads. Highest-impact: fullwidth-digit Luhn-valid card numbers (regex finds the candidate, `char::to_digit(10)` rejects non-ASCII digits, Luhn drops the mask — full PAN leaks). 10 `#[ignore]`'d FINDINGS-ONLY tests are pre-baked to un-ignore when this stage lands.

The crate IS wired correctly through 7 gateway response handlers (audit / worker / hire / approvals); the substrate gap is in `redact_text`'s normalisation, not its callers.

## Acceptance criteria

1. Add a canonicalisation pre-pass before the regex scan in `crates/argo-redact/src/` (likely `lib.rs` or a new `canonicalize.rs`):
   - **NFKC normalisation** (Unicode canonical decomposition + compatibility composition — covers fullwidth `＠`, fullwidth digits, half-width katakana variants, etc.)
   - **URL-decode** (`%40` → `@`, `%2E` → `.`)
   - **HTML-entity decode** (`&#64;` → `@`, `&amp;` → `&`)
   - **Zero-width-space + soft-hyphen strip** (`U+200B`, `U+00AD`)
   - **Base64 detect-and-rescan**: any token matching `^[A-Za-z0-9+/=]{12,}$` is decoded and re-scanned through the same redactor.
2. Run the existing scan with the canonicalised text; mask offsets must be projected back to the original byte positions so callers (audit / SPA preview) still see the un-canonicalised text with redaction markers in the right spots.
3. Un-`#[ignore]` the 10 FINDINGS-ONLY tests in `encoding_bypass_corpus_test.rs` that map to Stage 1 (the brief author named them in the PR #2126 body — claim-time worker should re-confirm the list against the test file's `#[ignore]` annotations); they must turn green.
4. Per-crate `cargo clippy -p argo-redact -- -D warnings` clean. Workspace clippy fast-fail clean.
5. No new dependencies beyond `unicode-normalization`, `percent-encoding`, and `base64` (all already in workspace or stdlib-adjacent).

## Parallel-surface declarations

- `rg 'pub fn (redact_text|canonicalize)' crates/argo-redact/` — `redact_text` exists; `canonicalize` does not. New pub fn `canonicalize_for_scan(input: &str) -> (String, OffsetMap)` is fine — single new surface, named so a follow-up Stage 2 brief can extend without parallel-name collision.
- Wire-up consumer: 7 gateway handlers (already calling `redact_text`) — they continue calling the same entry point; canonicalisation is internal to `redact_text`.

## May-touch

- `crates/argo-redact/src/lib.rs` (or split into `canonicalize.rs` if it pushes past 500 LOC)
- `crates/argo-redact/Cargo.toml` (add `unicode-normalization` + `percent-encoding` if not already present)
- `crates/argo-redact/tests/encoding_bypass_corpus_test.rs` (un-`#[ignore]` 10 Stage-1 tests)
- `crates/argo-redact/tests/` (new offset-projection unit test if not covered by corpus)

## Boundary

- Do NOT touch the 7 gateway callers — internal-only change.
- Do NOT add NER / spelled-out / character-jittered handling — that's Stage 3 (DEFERRED-v1.x).
- Do NOT extend address suffixes / phone separators / unit-slash addresses — that's Stage 2 (`mission-redact-broaden-suffix-and-separator-classes`).
- Luhn-on-NFKC (rescan the canonicalised card-number tokens) lands HERE, not in Stage 2 (the PR #2126 report flags it as the highest-impact bypass; the fix is a one-line `s.nfkc().collect::<String>()` before the Luhn call).

## Sizing

~300-500 LOC (canonicalisation pre-pass + offset projection + unit tests + Cargo.toml deps). Stays under the 500-LOC file ceiling if split into `canonicalize.rs`.

**🟡 P1 (PII-LEAK CONTAINMENT)** — closes 14 of 23 PII bypass misses identified in PR #2126's corpus. Higher impact than Stage 2 because it includes Luhn-on-NFKC (fullwidth-digit card-number leak fix).
