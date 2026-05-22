# argo-security-pii-detect-broaden-phone-and-address

**Owner**: claude.
**Depends on**: `mission-hire-audit-payload-shape-validation` (#2067 MERGED-FINDINGS-ONLY).
**Authored against**: origin/main `6e5ab8d5b` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh argo-security-pii-detect-broaden-phone-and-address`.
**fix-by**: `2026-05-30` (P1 budget per `claude-conductor-fix-by-enforcement`).

🟡 **P1 PII REDACTION COMPLETENESS.** Source: PR #2067 Tier-1 review (FINDINGS-ONLY ship). Two PII patterns are NOT redacted by the current `argo-redact` / `pii_detect` implementation:

1. **International phone format** — `+1-555-0123` slips through. The current regex only matches strict US 3-3-4 (`555-555-0123` / `(555) 555-0123`). International prefix `+1-` or `+44-`, etc. is not handled.
2. **Street address** — `123 Main St, San Francisco` slips through. No street-address pattern exists.

#2067 ships these as `#[ignore]`'d tests with FINDINGS-ONLY markers — this brief closes them.

## Scope

### 1. Phone — broaden the regex

Update `argo-redact/src/patterns.rs` (or wherever phone-PII lives) to handle:
- International prefix: `+<1-3 digits>-` followed by 3-3-4 or similar.
- Spaces and dots as separators: `+1 555 0123`, `1.555.0123`.

Match `[PHONE]` (or whatever the existing redaction marker is — preserve the contract). Do NOT over-match — a 4-digit year like "2026" must NOT be matched as a phone number.

### 2. Street address — add a new pattern

Add a `street_address` pattern matching the canonical shape:
- `\d{1,5}\s+\w+(\s+\w+)*\s+(St|Street|Ave|Avenue|Rd|Road|Blvd|Boulevard|Ln|Lane|Dr|Drive|Way|Ct|Court|Pl|Place|Pkwy|Parkway)\b`

Match marker: `[ADDR]` (or check the existing convention — `[ADDRESS]` may be the consensus).

US-shape only is fine for v1.x; international street addresses are a separate brief if needed later.

### 3. Drop `#[ignore]` on #2067's two tests

`crates/argo-cli/src/handlers/gateway/routes_workers_hire_audit_payload_tests.rs` (or sibling file from #2067) — find the `#[ignore]`'d tests for international phone + street address, drop the attribute, and verify both pass against the new patterns.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-redact/src/patterns.rs` (or canonical PII-pattern file — find via `rg "fn redact_text|pub const.*PII"`).
- `crates/argo-redact/src/patterns_tests.rs` — sibling unit tests covering the new patterns + negative cases (year-like 4-digit, single-word that looks address-y, etc.).
- `crates/argo-security/src/pii_detect.rs` (or wherever the detector lives, if it also needs patching).
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire_audit_payload_tests.rs` — drop the 2 `#[ignore]`'s.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The audit pipeline that consumes the redactor — that's already integration-tested via #2067 + the un-ignored tests will prove it works.
- Other unrelated PII patterns (SSN, credit card, etc.) — out of scope.
- The Auditable impl on the hire handler.

## Acceptance criteria

- [ ] `+1-555-0123` and similar international formats redact to `[PHONE]` (or canonical marker).
- [ ] `123 Main St, San Francisco` redacts to `[ADDR]` (or canonical marker).
- [ ] Negative tests: year `2026`, single word `St`, bare integer `123` do NOT trigger redaction.
- [ ] Both `#[ignore]`'s in #2067's test file dropped + both tests pass.
- [ ] `cargo clippy -p argo-redact -p argo-security -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `argo-security-pii-detect-broaden-phone-and-address`
- PR title: `fix(security): broaden PII detector — international phone + street address (argo-security-pii-detect-broaden-phone-and-address)`

## Notes for the implementer

- ~100-180 LOC expected. 2 regex additions ~30, unit tests ~80, test un-ignore + verification ~10.
- **Regex over-match is the failure mode** to avoid. The brief's negative tests (year, single word, bare integer) lock the contract — DO NOT skip them.
- **`+1-555-0123` is a US international format**; if the regex only handles `+1`, document why (v1.x target). `+44`, `+61`, etc. can be added in a follow-up if a customer needs them.
- This is the fix-pair for #2067's FINDINGS-ONLY ship. PR body should explicitly say "closes #2067's 2 `#[ignore]`'d tests."
