# `argo-security` crate surface audit

**Date**: 2026-05-24
**Brief**: `eval-argo-security-crate-surface`
**Mission link**: Cross-cutting security substrate flagged 🔴 NOT AUDITED in the mission-vs-code gap manifest. `argo-security::pii_detect` is the only currently-shipped module and is consumed on every audit + message write path.
**Verdict**: Surface is small, coherent, and now pinned by an active integration test. No new `argo-security` substrate fix-brief is needed. One operational gap (no `tests/` directory existed for `argo-security` before this PR) is closed by this work.

## Public Surface Inventory

`crates/argo-security/src/lib.rs` re-exports a single module:

| Slice | Re-exports |
|---|---|
| Module | `pub mod pii_detect` |

`crates/argo-security/src/pii_detect.rs` exposes 11 public names:

| Slice | Re-exports |
|---|---|
| Types | `PiiKind` (enum with 6 variants: `Ssn`, `CreditCard`, `Email`, `UsPhone`, `Iban`, `StreetAddress`), `PiiFinding` (pub fields `kind`, `span`, `masked_value`), `PiiFindings` (newtype over `Vec<PiiFinding>`), `PolicyConfig` (pub fields `detect`, `autoredact`, `log_findings`) |
| Functions | `scan`, `scan_and_maybe_redact`, `apply_policy` |
| Methods | `PiiKind::label`, `PiiFindings::is_empty`, `PiiFindings::redact`, `PolicyConfig::from_env` |

The new contract test at `crates/argo-security/tests/surface_contract.rs` pins all 11 names with a compile-fence (any rename or removal turns into a downstream compile failure) plus behavior pins for every detector class and every `PolicyConfig` branch.

## Test Coverage Added

`tests/surface_contract.rs` adds active coverage for:

| Contract | Pin |
|---|---|
| Re-export inventory | Compile-fence: `use argo_security::pii_detect::{apply_policy, scan, scan_and_maybe_redact, PiiFinding, PiiFindings, PiiKind, PolicyConfig};` plus explicit function-type assertions for `scan`, `scan_and_maybe_redact`, `apply_policy`. |
| `PiiKind` wire labels | `PiiKind::Ssn.label() == "ssn"` etc. for all 6 variants — these labels persist into `<redacted:label>` markers in the audit log, so a rename breaks downstream JSON consumers. |
| SSN detector | Positive on `123-45-6789`, negative on `912-...` (9XX issuance-reserved area) and `...-0000` (all-zero serial). |
| Credit-card detector | Positive on Luhn-valid `4111 1111 1111 1111`; negative on shape-identical Luhn-invalid `4111 1111 1111 1112`. |
| Email detector | Positive on `alice.smith+work@example.co.uk`; negative on bare-host `alice@localhost` and 1-character TLD `bob@x`. |
| Phone detector | Positive on `(415) 555-0123` and `+44 20 7946 0958`; negative on bare 4-digit year `2026`. |
| IBAN detector | Positive on canonical mod-97-valid `GB82WEST12345698765432`; negative on shape-identical bad-checksum `GB99WEST12345698765432`. |
| Street-address detector | Positive on `456 North Main Ave`; negative on bare-integer `123` and lone-suffix `St Andrews`. |
| Byte-span stability (multi-byte) | Email span after a 🚀 emoji prefix slices cleanly via `&input[f.span]`; phone span after a `联系 ` CJK prefix slices cleanly. |
| Byte-span stability (punctuation) | Credit-card span inside `(card:4111-1111-1111-1111;ok)` anchors to the 16-digit run, excluding surrounding punctuation. |
| Multi-finding ordering | Two non-overlapping findings (SSN + email) return spans sorted by `start` and `redact` preserves the inter-finding text. |
| `PolicyConfig::detect = false` | `apply_policy` returns the input as `Cow::Borrowed` with no allocation, regardless of `autoredact`. |
| `PolicyConfig` soft-flag (`detect=true`, `autoredact=false`) | Returns original input as `Cow::Borrowed`. |
| `PolicyConfig` autoredact (`detect=true`, `autoredact=true`) | Returns redacted form as `Cow::Owned`. |
| No-findings + autoredact | Returns input as `Cow::Borrowed` (no spurious allocation when nothing matches). |
| Masking contract | `PiiFinding::masked_value` always contains `*` and never equals the raw match — guarantees that an audit consumer logging the masked field cannot inadvertently leak the underlying PII. |

This is intentionally no-network and no-env-mutation: every test runs `apply_policy` with an explicit `PolicyConfig` rather than `scan_and_maybe_redact`, so the contract test stays race-free under cargo's parallel test runner. The unit tests in `src/pii_detect_tests.rs` use the same discipline.

## Production Consumers

| Consumer | Symbols | Assessment |
|---|---|---|
| `argo-audit::sink::canonicalize_event_row` | `pii_detect::scan_and_maybe_redact` | Live audit write path. Every event canonicalization passes through PII redaction with site label `"audit_write"`. |
| `argo-audit::sink_tx` | `pii_detect::scan_and_maybe_redact` | Same as `sink.rs`, transaction-scoped variant. |
| `argo-store::message_query` | `pii_detect::scan_and_maybe_redact` | Live message write path. Site label `"message_write"`. |

All three consumers route through `scan_and_maybe_redact`, which the contract test pins behaviorally via the equivalent `apply_policy` calls. No production caller of `argo-security` is unwired.

## Findings

No new finding in `argo-security` itself.

Surface design observations (informational, no fix-brief warranted):

1. **No prior `tests/` directory existed for `argo-security`.** All behavior coverage was in `src/pii_detect_tests.rs` (a private `#[cfg(test)] mod`), which exercises the API from inside the crate. That's fine for unit tests but doesn't pin the *public* surface that downstream crates rely on — a private rename of `pub fn scan` could in principle pass crate-local tests while breaking `argo-audit`. Closed by this PR.
2. **Env-knob mutation contract is unit-tested via `apply_policy`, not `scan_and_maybe_redact`.** This is the right call (parallel cargo tests can't safely mutate `ARGO_PII_*` env vars), and the contract test continues the pattern. `PolicyConfig::from_env` is referenced once in the inventory test as a compile-fence so accidental removal still surfaces.
3. **Crate has no `dev-dependencies`.** The contract test stays inside `std` + the crate's public types, so the empty `[dev-dependencies]` section in `Cargo.toml` is intentionally untouched.

## Follow-Up Briefs

None. The audit closes cleanly with the contract test plus this report.

## Verification

Required command:

```bash
cargo test -p argo-security --tests
```

The PR body should report the final output from that command.
