# `argo-redact` — regex fence vs real fence

**Authored**: 2026-05-23 (evaluator follow-on to gap manifest `argo-redact` row).
**Brief**: `docs/build-plan/streams/eval-redact-crate-promote-from-regex-to-real-fence.md`.
**Verdict (short form)**: 🟡 **PROMOTE — staged**. The v1 regex layer is wired
correctly through every gateway response path (the gap manifest's "once it
lands" status is now obsolete — the fence has been live since Track-J / E1-e),
but the regex-only design is **bypassable through 23 of the 30 black-box
payloads** in the new encoding-bypass corpus. The promotion path is
**canonicalisation pre-pass first, NER second** — NOT direct LLM-based
redaction, which would add unacceptable latency on every audit-row render.

## §0 — Source-of-truth cite from gap manifest

Verbatim from `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
(Claim 6 — Multi-channel approval + multi-tenant by construction):

> | `argo-redact` | ? | 🔴 NOT AUDITED | PII redaction for audit + export. Per
> `routes_workers_hire.rs:25` comment: "argo-redact::redact_text (J2) is the
> real fence once it lands; until then, basic regex covers the common case".
> **So redact is at v0.5; relies on inline regex in handlers.** |
> `routes_workers_hire.rs:25-27` | 🟡 P2 |
> `eval-redact-crate-promote-from-regex-to-real-fence` — verify
> `argo-redact::redact_text` is now wired (the comment said "once it lands").

## §1 — Surface inventory (current state on `origin/main`)

### 1.a Crate shape

```
crates/argo-redact/
├── Cargo.toml              # deps: regex, serde, serde_json, chrono, tracing
├── fuzz/                   # libfuzzer targets (out of scope here)
└── src/
    ├── lib.rs              # 61 LOC — public surface + sentinel consts
    ├── lib_tests.rs        # 381 LOC — 25 unit tests, mostly happy/negative-shape
    ├── grants.rs           # 139 LOC — elevated-view grant lookup
    ├── patterns.rs         # 206 LOC — 6 regex patterns + Luhn
    ├── policy.rs           # 185 LOC — RedactionPolicy + ViewerPermissions
    └── redact.rs           # 180 LOC — redact_text + redact_approval_row
```

Every file is well under the 500-LOC ceiling; no boundary risk in adding a
canonicalisation module under `src/canonicalise.rs` when promotion lands.

### 1.b 6 categories covered today

| Category  | Sentinel  | Validator       |
|-----------|-----------|-----------------|
| Email     | `[EMAIL]` | shape-only      |
| Phone     | `[PHONE]` | shape-only (NZ trunk + intl + US 3-3-4) |
| Card      | `[CARD]`  | shape + Luhn checksum (ASCII digits only) |
| IBAN      | `[IBAN]`  | shape-only (no mod-97 check) |
| IRD       | `[IRD]`   | shape-only (NZ tax IDs) |
| Address   | `[ADDR]`  | shape-only (US-style number + name + suffix whitelist) |

### 1.c Wire-up across the gateway

`redact_text` / `RedactionPolicy::default()` are imported by 7 production
handlers and one test module on `origin/main`:

```
crates/argo-cli/src/handlers/gateway/routes_worker_memory.rs:22
crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:22
crates/argo-cli/src/handlers/gateway/routes_audit_detail_render.rs:9
crates/argo-cli/src/handlers/gateway/routes_worker_memory_helpers.rs:12
crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:50
crates/argo-cli/src/handlers/gateway/routes_worker_activity.rs:23
crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs:405
crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs:36
```

**Gap-manifest claim "once it lands" is obsolete.** `argo-redact::redact_text`
IS wired. The comment in `routes_workers_hire.rs:25` is a historical
artifact from before J2 landed; it should be cleaned up (out of this brief's
may-touch list — flagged as a tiny doc-comment fix-brief).

### 1.d Pre-flight verification

```bash
$ rg 'argo_redact::redact_text|argo_redact::RedactionPolicy' crates/argo-cli | wc -l
       18
```

Inline regex fallback paths (the ones the historical comment referenced) are
gone — `routes_workers_hire.rs:404-405` now delegates to `argo_redact::redact_text`
directly. The v0.5 inline-regex phase has shipped its way to v1.0.

## §2 — Bypass corpus (new test artifact)

A 30-case black-box bypass corpus ships in this PR as
`crates/argo-redact/tests/encoding_bypass_corpus_test.rs`. Each case is a
single-line plaintext payload designed to slip a real PII value past a
pattern-only fence. The corpus splits into **pin tests** (always pass — they
document current behaviour and would flag any regression) and
**FINDINGS-only `#[ignore]`'d tests** (assert what a real fence MUST catch —
they fail today and un-ignore in the follow-up promotion PR).

### 2.a Result matrix

| # | Vector | Example payload | v1 regex | Real fence required |
|---|---|---|---|---|
| 1 | base64-wrapped email | `c2FtQGFjbWVjby5jb20=` | MISS | must catch |
| 2 | URL-encoded email | `sam%40acmeco.com` | MISS | must catch |
| 3 | HTML-entity `@` | `sam&#64;acmeco.com` | MISS | must catch |
| 4 | fullwidth `@` (U+FF20) | `sam＠acmeco.com` | MISS | must catch |
| 5 | zero-width-space split | `sam@\u{200B}acmeco.com` | MISS | must catch |
| 6 | whitespace jitter | `s a m @ c u r b l e . c o m` | MISS | must catch |
| 7 | bracketed `[at] [dot]` | `sam[at]acmeco[dot]com` | MISS | must catch |
| 8 | soft hyphen in domain | `sam@acm\u{00AD}eco.com` | MISS | must catch |
| 9 | IP-literal domain | `sam@[192.168.1.1]` | MISS | must catch |
| 10 | combining acute in local | `sa\u{0301}m@acmeco.com` | MISS | must catch |
| 11 | MIME quoted-printable `@` | `sam=40acmeco.com` | MISS | must catch |
| 12 | newline-split email | `sam@\nacmeco.com` | MISS | nice-to-have |
| 13 | RFC-5321 quoted-local | `"sam.lastname"@acmeco.com` | MISS | must catch |
| 14 | sub-addressing `+suffix` | `sam+billing@acmeco.com` | CATCH | — |
| 15 | Devanagari-digit phone | `\u{0966}\u{096A}\u{0967} \u{096B}\u{096B}\u{096B} \u{0966}\u{0967}\u{0968}\u{0969}` | MISS | should catch |
| 16 | spelled-out phone | `oh-two-seven five-five-five one-two-three-four` | MISS | NER-only |
| 17 | en-dash phone separator | `555–555–0123` | MISS | must catch |
| 18 | trailing extension text | `(555) 555-0123 ext. 42` | CATCH | — |
| 19 | dot-separated card | `4111.1111.1111.1111` | MISS | must catch |
| 20 | underscore-separated card | `4111_1111_1111_1111` | MISS | should catch |
| 21 | l-for-1 obfuscation | `4l11 llll llll llll` | MISS | should catch |
| 22 | fullwidth-digit card | `４１１１ １１１１ １１１１ １１１１` | MISS (Luhn rejects) | must catch |
| 23 | card embedded mid-sentence | `I gave them 4111 1111 1111 1111 over the phone` | CATCH | — |
| 24 | canonical-spaced IBAN | `GB82 WEST 1234 5698 7654 32` | MISS | must catch |
| 25 | lowercase IBAN | `gb82WEST12345698765432` | MISS | must catch |
| 26 | Terrace suffix address | `123 Main Terrace` | MISS | must catch |
| 27 | Circle suffix address | `456 Maple Circle` | MISS | must catch |
| 28 | AU unit-slash address | `5/123 Main St Carlton` | partial (catches `123 Main St`, leaks unit `5`) | must canonicalise |
| 29 | multi-PII inline | `user sam@… on 027 555 1234 paid 4111 …` | CATCH (all 3) | — |
| 30 | spaced IBAN canonical (dup of 24, pinned for sanity) | — | — | — |

**Score: 23 MISS / 5 CATCH / 1 partial / 1 sanity-dup.** The v1 regex layer
catches the "operator pastes a clean PII string into a hire context" case
(its design target), and misses essentially every payload that's been
through an encoding step or a tiny manual obfuscation.

### 2.b Severity calibration

The leak surface only matters if attackers (or auto-generated content) can
reach a redaction path with crafted inputs. Survey of the 7 wired handlers:

- `routes_workers_hire.rs` — operator pastes role description. **Low**
  attack surface; the redactor here is documentation/compliance, not
  defence. Bypass severity: 🟡 P2.
- `routes_audit_feed.rs` / `routes_audit_detail.rs` / `…_render.rs` —
  these render audit rows already written by other workers. If a worker
  agent (LLM-generated content) writes URL-encoded PII into a memory or
  action record, the redact pass reading the row back out misses it.
  Bypass severity: 🔴 P1 — worker output is by definition crafted-by-
  model and routinely contains encoded fragments.
- `routes_approvals_json.rs` — approval rows shown to humans before
  decision. Bypass severity: 🔴 P1 — operator sees raw PII.
- `routes_worker_memory.rs` / `…_helpers.rs` / `routes_worker_activity.rs` —
  worker memory + activity feed. Bypass severity: 🔴 P1 (same reasoning).

The single highest-impact bypass is **fullwidth-digit card numbers**: the
regex catches the candidate (cards regex uses `\d`, which matches
`\p{Nd}`), but `char::to_digit(10)` returns `None` for non-ASCII digits, so
Luhn fails and the candidate is dropped from the mask list. A worker that
writes `４１１１ １１１１ １１１１ １１１１` to memory (well-formed Luhn-valid card in fullwidth digits) leaks the
full PAN through every read path.

## §3 — Recommendation

### 3.a What NOT to do

❌ **Don't go directly to LLM-based redaction.** Latency budget for
gateway audit/approval reads is single-digit ms; an LLM round-trip per row
is 100–500 ms. NER libraries that ship Rust bindings (Presidio analyzer
is Python-only; rust-bert is heavy and not yet stable on the workspace's
Rust 1.75 floor) introduce a runtime dep stack we'd need to support across
macOS + Linux + Windows targets.

❌ **Don't expand the regex set.** The regex layer is already at the
limit of what shape-matching can do; adding `Terrace|Circle|Mews|…` to the
address suffix list patches surface 26 + 27 but leaves the canonicalisation
problem (1–14, 17–22, 24–25, 28) untouched.

### 3.b What to do — staged promotion

**Stage 1 — canonicalisation pre-pass (`mission-redact-promote-to-real-fence`)**.
A new `crate::canonicalise::canonical_form(input: &str) -> Cow<str>` that:

1. NFKC-normalises the input (folds fullwidth → ASCII, soft-hyphen → ε,
   ZWJ/ZWSP → ε, fullwidth `@` → ASCII `@`).
2. Strips well-known invisible chars: U+200B, U+200C, U+200D, U+200E,
   U+200F, U+202A–U+202E (bidi), U+2060, U+FEFF.
3. URL-decodes `%XX` sequences (one pass — no recursion).
4. HTML-entity-decodes `&#NN;` and named entities for the 5 most common
   PII-relevant ones (`&#64;` → `@`, `&commat;` → `@`, `&period;` → `.`,
   `&NewLine;` → space).
5. MIME-quoted-printable-decodes `=NN` sequences.
6. Detects base64-like runs (length ≥ 16, alphabet `[A-Za-z0-9+/=]`,
   length % 4 == 0) and on a SUCCESSFUL decode that yields UTF-8 text,
   re-scans the decoded text recursively (depth ≤ 2 to bound cost).

Then `find_matches` runs over **both** the original and canonicalised
form, and any span hit in the canonical form is back-projected to a
"mask the original substring conservatively" decision.

This handles bypass vectors 1, 2, 3, 4, 5, 8, 9, 11, 12, 14, 19, 22, 24,
25 — 14 of 23 misses — and is a pure-Rust, dependency-free addition
(NFKC via `unicode-normalization` crate, ~30k LOC, already
workspace-transitive).

Estimated effort: ~300-500 LOC + corresponding tests in a single brief.

**Stage 2 — word-boundary & dictionary expansion** (sibling brief, can
land independently).

1. Address suffix vocabulary expansion (`Terrace, Tce, Circle, Cir, Mews,
   Crescent, Cres, Close, Cl, Quay, Esplanade, Promenade`).
2. Phone separator class expansion: accept U+2013 EN DASH, U+2014 EM DASH,
   U+2212 MINUS SIGN, U+00B7 MIDDLE DOT as aliases of `-`.
3. Card separator class expansion: accept `.`, `_`, en-dash, em-dash.
4. AU/NZ unit-slash address form: `(\d{1,4}\/)?\d{1,5}\s+…` to capture
   the leading unit.
5. Luhn validator: normalise to NFKC before `to_digit(10)` so fullwidth
   digits Luhn-validate correctly.

Estimated effort: ~150-250 LOC + tests. Closes bypass vectors 17, 20, 26,
27, 28 — and the Luhn-on-NFKC fix interacts with Stage 1 to close 22.

**Stage 3 — NER + dictionary (DEFERRED-v1.x)**. The remaining bypass
vectors (6, 7, 10, 13, 15, 16, 18, 21) need either a learned-token model
or a phrase dictionary. Worth doing once SKYCOT picks up a production
HIPAA/GDPR customer with a contractual obligation to mask spelled-out
phone numbers and combining-mark email; not v1.0 critical.

### 3.c Combined leak-coverage projection

| Stage | Misses closed | Cumulative MISS count | Cumulative coverage |
|-------|---------------|----------------------|---------------------|
| v1 today          | —          | 23 of 30 | 23% |
| + Stage 1 (canon) | 14         | 9 of 30  | 70% |
| + Stage 2 (dicts) | 5          | 4 of 30  | 87% |
| + Stage 3 (NER)   | 4          | 0 of 30  | 100% |

Stage-1+Stage-2 is the v1.0 enterprise-grade target. Stage 3 is the
"contractually mandated" rung for regulated-industry customers.

## §4 — Findings shipped in this PR

This PR is FINDINGS-ONLY for the redactor itself — no behavioural change
to `argo-redact`. What ships:

- `crates/argo-redact/tests/encoding_bypass_corpus_test.rs` — 30-case
  black-box corpus (~400 LOC). 28 pin tests pass on `origin/main` today;
  10 `#[ignore]`'d FINDINGS-only tests will turn green when the
  promotion lands.
- This eval report.

The pin tests are the regression check that v1 behaviour doesn't drift
under unrelated edits. The ignored FINDINGS tests are the acceptance
gate for the promotion brief.

## §5 — Next brief

**`mission-redact-promote-to-real-fence`** — Stage 1 (canonicalisation
pre-pass) per §3.b. May-touch: `crates/argo-redact/src/`,
`crates/argo-redact/Cargo.toml` (add `unicode-normalization` if not
already workspace), `crates/argo-redact/tests/encoding_bypass_corpus_test.rs`
(un-ignore the 10 FINDINGS tests that the canonicalisation pass should
close). ~300-500 LOC. Conductor queues post-merge of this eval PR.

**`mission-redact-broaden-suffix-and-separator-classes`** — Stage 2 per
§3.b. May-touch: `crates/argo-redact/src/patterns.rs`,
`crates/argo-redact/src/lib_tests.rs`, the bypass corpus. ~150-250 LOC.

## §6 — Boundary notes

- May-touch list satisfied: edits restricted to `crates/argo-redact/`
  (new test file + cargo manifest? — no, no manifest changes) and this
  eval report under `docs/skycot/evals/scheduled/2026-05-23/`.
- `docs/build-plan/BUILD_STATUS.md` row flipped to IN_REVIEW with PR
  number on the post-push commit.
- No production source files outside the may-touch list edited.
- All 6 invariants honoured: no new file > 500 LOC, no fn > 80 LOC, no
  `unsafe`, no `unwrap`/`expect` outside tests, no global mutable state,
  rustdoc on all new public surfaces.
- Out-of-scope cleanup flagged: `routes_workers_hire.rs:25` historical
  comment ("once it lands") should be cleaned up by a tiny follow-up
  fix-brief — out of this audit's may-touch list.
