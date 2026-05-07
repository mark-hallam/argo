# Mission Phase 2 — PII detection + soft-flag with optional auto-redact

**Owner**: claude. **Depends on**: none (independent).

## Scope

Argo stores user messages, audit rows, and forge run inputs. Today, nothing
detects when a user pastes their SSN into a chat or accidentally logs a credit
card number. This brief adds a `pii_detect` module that scans strings for known
PII patterns (SSN, credit-card via Luhn, email, US phone, IBAN) and returns a
structured `PiiFindings` report. By default, findings are *soft-flagged* (logged
+ surfaced in admin UI later). With `ARGO_PII_AUTOREDACT=1`, findings are
replaced inline with `<redacted:kind>` before the value is persisted.

Hooks: message-write path and audit-write path.

## Approach (concrete)

1. **NEW** `crates/argo-security/src/pii_detect.rs` (~280 LOC):
   - `pub enum PiiKind { Ssn, CreditCard, Email, UsPhone, Iban }`
   - `pub struct PiiFinding { kind: PiiKind, span: Range<usize>, masked_value: String }`
   - `pub struct PiiFindings(pub Vec<PiiFinding>); impl PiiFindings { pub fn is_empty(&self) -> bool; pub fn redact(&self, input: &str) -> String; }`
   - `pub fn scan(input: &str) -> PiiFindings`
   - SSN: `\b\d{3}-\d{2}-\d{4}\b` (with shape filter to avoid false positives)
   - Credit card: regex for 13-19 digits + Luhn check (only Luhn-valid hits flagged)
   - Email: standard RFC-5321-ish lite regex
   - US phone: `\b\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b`
   - IBAN: country prefix + 13-30 alphanumerics + checksum
2. **Decide crate location**:
   - If `crates/argo-security/` exists → add module there.
   - If NOT → create the crate (`Cargo.toml`, `src/lib.rs`, `src/pii_detect.rs`)
     and add to workspace members. Keep deps minimal: `regex`, `serde`,
     `argo-types` only.
3. **Wire into write paths** (narrowly):
   - Message persist path (search for the message-insert helper) — call
     `pii_detect::scan` on body, log findings, optionally redact
   - Audit insert helper (post-#audit-log-immutability if ordering allows) —
     same pattern on `target` field
4. **Env knobs**:
   - `ARGO_PII_DETECT=0` disables scanning entirely (default on, low cost)
   - `ARGO_PII_AUTOREDACT=1` replaces findings inline before persist
   - `ARGO_PII_LOG_FINDINGS=1` emits a `tracing::warn!` per finding (default
     on; off in production once ops decides)

## Files you may touch

- `crates/argo-security/src/pii_detect.rs` (NEW, ≤ 320 LOC) — OR a new crate if
  `argo-security` does not exist
- `crates/argo-security/src/pii_detect_tests.rs` (NEW, ≤ 250 LOC) — 7 unit tests:
  - SSN detected
  - Credit card with valid Luhn detected, with invalid Luhn ignored
  - Email detected
  - US phone detected
  - IBAN detected
  - clean string → empty findings
  - `redact` produces correct masked output
- `crates/argo-security/src/lib.rs` — narrowly: 1 pub mod line (or new crate)
- Message-write call site (1 file) — narrowly
- Audit-write call site (1 file) — narrowly
- Workspace `Cargo.toml` — narrowly, only if creating new crate

## Files you must NOT touch

- Read paths — detection is write-side only
- `argo-types` — keep `PiiKind` etc. inside `argo-security`
- Any UI surfaces — flagging-in-UI is a follow-up brief

## Acceptance criteria

- [ ] `pii_detect::scan` returns correct findings for each kind
- [ ] Luhn check rejects invalid card-shaped digits
- [ ] `ARGO_PII_AUTOREDACT=1` replaces values with `<redacted:kind>` before persist
- [ ] `ARGO_PII_DETECT=0` short-circuits the scanner
- [ ] 7 unit tests
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-security` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-security-pii-detection`
- PR title: `security: PII detection + soft-flag write-path scanner (claude-security-pii-detection)`

## Notes for the implementer

- Compile regexes once via `OnceLock<Vec<Regex>>`.
- Luhn implementation is ~15 LOC — do not pull a new crate just for that.
- Be conservative on email/SSN regex; false positives generate audit-noise.
- Findings are *spans*, so the `redact` helper can produce minimal-edit
  redacted output even with multiple hits.
