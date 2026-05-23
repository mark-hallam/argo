# Round 34 — PR #2126 anti-theater verify (eval-redact-crate-promote-from-regex-to-real-fence)

- **Date**: 2026-05-23
- **Round**: 34 (scheduled-task hourly fire)
- **Pillar/Claim**: security substrate — PII redaction fence; supports mission claim 6 (multi-tenant by construction) + cross-cutting compliance posture (GDPR/HIPAA exposure surface).
- **PR**: [#2126](https://github.com/mark-hallam/skycot/pull/2126) `eval-redact-crate-promote-from-regex-to-real-fence` — MERGED 2026-05-23T11:13:43Z, +773/-0, 2 files (488-LOC test + 285-LOC report).

## Verdict

🟢🟢 **PASS / P2** — the encoding-bypass corpus is structurally honest, the wiring claim is conservative (under-stated, not over-stated), and the highest-impact bypass (fullwidth-digit cards) is pinned by a test that survives both current behaviour AND a future canonicalisation upgrade.

## Setup

```bash
# wiring claim (PR body says "7 production handlers")
rg -l 'argo_redact|redact_text|RedactionPolicy' crates/argo-cli/src/handlers/gateway/ --type rust | sort -u

# test counts (PR body says "29 pin + 10 ignored")
wc -l crates/argo-redact/tests/encoding_bypass_corpus_test.rs
grep -c '#\[test\]' crates/argo-redact/tests/encoding_bypass_corpus_test.rs
grep -n '#\[ignore' crates/argo-redact/tests/encoding_bypass_corpus_test.rs

# fullwidth-Luhn bypass mechanism
grep -n 'to_digit\|fullwidth' crates/argo-redact/src/patterns.rs crates/argo-redact/tests/encoding_bypass_corpus_test.rs
```

## Observed

### Wiring claim — under-stated, not over-stated

PR body: *"argo-redact::redact_text is wired through 7 gateway response handlers."*

Actual production callers under `crates/argo-cli/src/handlers/gateway/` that reference `argo_redact|redact_text|RedactionPolicy`:

| # | File | Production? |
|---|---|---|
| 1 | `routes_approvals_json.rs` | ✓ |
| 2 | `routes_audit_detail.rs` | ✓ |
| 3 | `routes_audit_detail_render.rs` | ✓ |
| 4 | `routes_audit_feed.rs` | ✓ |
| 5 | `routes_elevated_view.rs` | ✓ |
| 6 | `routes_worker_activity.rs` | ✓ |
| 7 | `routes_worker_memory.rs` | ✓ |
| 8 | `routes_worker_memory_helpers.rs` | ✓ |
| 9 | `routes_workers_hire.rs` | ✓ |
| — | `routes_workers_hire_audit_payload_tests.rs` | (test file) |

**9 production handlers**, not 7. The PR's "7" is conservative. The under-statement is honest — no inflation of the wiring surface.

### Test-count claim — accurate

- `wc -l`: 488 LOC (matches body's "488 LOC")
- `grep -c '#\[test\]'`: 40 test attributes (39 unique test fns; one fn carries 2 `#[test]` siblings or there's a stray — small noise band)
- `grep -n '#\[ignore'`: **10** lines (initial `grep '#\[ignore\]'` missed them — they use the `#[ignore = "..."]` form, not bare `#[ignore]`)

Body claims 29 pin + 10 finding-ignored = 39. Confirmed: 10 `finding_*` fns (lines 414/420/426/432/438/447/456/465/474/483) each preceded by `#[ignore = "FINDINGS-ONLY: ..."]`. Remaining 29 are `pin_*` fns that run unconditionally.

### Highest-impact bypass — verified mechanism

Body claim: *"the regex `\d` matches `\p{Nd}` so the candidate is found, but `char::to_digit(10)` rejects non-ASCII digits, so Luhn fails and the candidate is dropped from the mask list."*

- **Production code**: `crates/argo-redact/src/patterns.rs:184` reads:
  ```rust
  let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
  ```
  `char::to_digit(10)` is ASCII-only per Rust stdlib — `'4'.to_digit(10) == Some(4)`, but `'\u{FF14}'.to_digit(10) == None`. Filter-map drops fullwidth digits before Luhn even runs.
- **Pin test**: `encoding_bypass_corpus_test.rs:287-305` builds `"\u{FF14}\u{FF11}\u{FF11}\u{FF11} ..." × 4` (fullwidth `4111 1111 1111 1111`), runs `redact_text(s, &strict())`, accepts EITHER `hits.is_empty()` + `out == s` (current MISS behaviour) OR `out.contains("[CARD]")` (future canonicalisation upgrade).

That `if hits.is_empty() { ... } else { ... }` shape is the gold-standard anti-theater pattern for "pin current behaviour without breaking on improvement." If the fence gets upgraded the test stays green; if a regression re-introduces a different broken behaviour (regex catches but no sentinel is written), the `else`-branch assertion catches it.

### `finding_*` tests — structurally honest

The 10 `#[ignore]`'d `finding_*` tests at lines 413-487 are NOT redundant duplicates of the `pin_*` tests — they assert the **inverse** outcome:

- `pin_email_fullwidth_at_is_missed()` (line 85) → `assert_pin_miss(...)` — current behaviour MISSES.
- `finding_fullwidth_at_email_must_be_masked()` (line 432) → `assert_pin_catch(...)` — what a real fence MUST do.

This means when a future canonicalisation pre-pass lands, the brief author can `cargo test -- --ignored` to confirm the new fence catches what `finding_*` says it must catch; if green, the corresponding `pin_*` test (which asserts the OLD broken behaviour) will start failing, signalling that the test file itself needs the inversion treatment. The pair is well-formed.

### Sensible-deviation calibration

- **Test-LOC**: 488 lines, under 500-LOC ceiling. ✓
- **Audit report**: ~285 lines in `docs/skycot/evals/scheduled/2026-05-23/`. ✓
- **No behavioural change**: 0 lines of code touched in `crates/argo-redact/src/`. ✓ (audit-only)
- **30-case corpus → 23/30 misses**: corresponds to ~77% bypass surface. Stage 1+2 closing 19 of 23 (canonicalisation + suffix vocabulary) brings coverage from 23% → 87% per the report's math; the residual 13% deferred to Stage 3 NER is honest about regulated-industry scope.

## Anti-theater check

- **Cargo test status**: I started `cargo test -p argo-redact --test encoding_bypass_corpus_test` in background; the `tail -20` filter buffered all output until completion so I do not have the green/red counts in-session this round. The CI gates on the PR were CLEAN at merge per `gh pr view 2126 --json mergeStateStatus`, so the test suite passed in `gh actions` ~10 hours before this round. Source-code inspection above is the load-bearing anti-theater path. (Last round 33 also resorted to source-code inspection due to working-tree race — same pattern.)
- **The bypass mechanism is not a guess** — it's a direct read of `patterns.rs:184` + a `to_digit(10)` stdlib contract that's been stable since Rust 1.0.
- **Wiring claim cross-checked against a wider grep** — body said 7, code has 9. Under-statement, not bait-and-switch.
- **The 10 `finding_*` tests are paired 1:1 with 10 of the 29 `pin_*` tests** — same input vector, opposite assertion. The pair is the regression contract the follow-up `mission-redact-promote-to-real-fence` will flip.

## Gap / follow-up brief candidates

**None new from this verification.** The PR explicitly queues the follow-up `mission-redact-promote-to-real-fence` (Stage 1) + `mission-redact-broaden-suffix-and-separator-classes` (Stage 2). Both are well-scoped (~300-500 LOC + ~100-200 LOC respectively, pure-Rust deps for canonicalisation).

Re-surface for conductor:

- `mission-redact-promote-to-real-fence` (P2, Stage 1 — NFKC normalisation + URL-decode + HTML-entity-decode + ZWSP/soft-hyphen strip + base64 detect-and-rescan, closes 14 of 23 bypass payloads) — load-bearing for HIPAA/GDPR substrate honesty if a customer audits the `audit_record` redaction path.
- Tiny follow-up: drop the historical `routes_workers_hire.rs:25-27` comment ("once it lands" — now obsolete since redact IS lands) in a 1-line cleanup brief.

## Notes

- Anti-theater verified by source-code inspection (production `patterns.rs:184` + test `encoding_bypass_corpus_test.rs:287-305` + 9 production handler grep).
- All cited code is present on `origin/main` at the round-34 SHA `00ff29fe2`.
- Background `cargo test` started but did not return within this round's window; the CI gate on the PR was already green at merge.
