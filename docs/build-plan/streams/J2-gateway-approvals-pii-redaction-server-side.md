# Stream J2 — Gateway approvals PII redaction (server-side fence)

**Owner**: claude. **Depends on**: J1 MERGED (this brief wraps J1's response).
**Authored against**: origin/main `bf9be9b2` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J2-gateway-approvals-pii-redaction-server-side`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 2 of 18 — closes the server-side PII fence promised by E1-e.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Pattern set | Same 5 categories as the client layer: email / phone / Luhn-card / IBAN / NZ IRD | Plus LLM-based NER for names / addresses / free-text PII | `mission-enterprise-pii-llm-classifier` |
| RBAC source | Per-Principal `permissions.unredacted_*` fields (extends E0's auth bridge) | Same — already enterprise-shaped | n/a |
| Performance | Per-row regex pass; ~50µs per row for typical 200-char details; bounded | LLM classifier adds cost; v1.5 caches per-row classification | `mission-enterprise-pii-classification-cache` |
| Elevated-view grant honoring | Server reads grant table; expired grants ignored | Same | n/a |
| Audit emission | Every redacted-field read audit-logged with grant id (when applicable) | Same | n/a |

v1 is acceptable: the regex set is the same as the
client (so frontend defense-in-depth + server fence
mask the same categories). LLM-based NER is the
next-generation enhancement.

## Why

E1-e shipped client-side PII redaction as defense-
in-depth and explicitly named this server-side
brief as the mandatory companion. Per the
enterprise-grade-defaults memory: "PII / data
classification: server-side fence is the
architecture, client redaction is defense-in-depth."

This brief is the real fence. It runs INSIDE the
J1 endpoint's response path so the gateway never
emits raw PII over the wire to viewers lacking the
appropriate permission.

## Scope

- A `redact_approval_row` function in
  `argo-redact` (NEW crate) that takes an
  `ApprovalRow` + viewer `Principal` and returns
  the row with masked fields.
- Wrap J1's GET response with this function per-
  row before serialization.
- Honor temporary elevated-view grants (read from
  the existing grant table; v1 mock grants in
  client until J16 ships the server endpoint).
- Default fences: ALL viewers (including
  delegators) see sentinels for the 5 pattern
  categories unless their Principal has the
  `unredacted-<category>` permission.
- Server emits audit row when a viewer accesses a
  row with active redaction (so we know what was
  shown vs what was masked).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - NEW `crates/argo-redact/` crate.
  - `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs`
    — locked by J1 until merge; this brief takes
    next lock.

## Files you may touch

- `crates/argo-redact/Cargo.toml` — NEW. Standard
  crate manifest. Deps: `regex`, `argo-types`,
  `argo-identity`. ≤ 30 LOC.
- `crates/argo-redact/src/lib.rs` — NEW. Crate
  root. ≤ 80 LOC.
- `crates/argo-redact/src/patterns.rs` — NEW.
  The 5 regex patterns + Luhn validator
  (Rust port of E1-e's `web/lib/redaction/patterns.ts`).
  ≤ 200 LOC.
- `crates/argo-redact/src/redact.rs` — NEW.
  `redact_text(text, policy)` + `redact_approval_row(row, principal)`.
  ≤ 250 LOC.
- `crates/argo-redact/src/policy.rs` — NEW.
  `RedactionPolicy` resolution from `Principal`
  (reads grants + permissions). ≤ 180 LOC.
- `crates/argo-redact/src/grants.rs` — NEW.
  Elevated-view grant lookup (v1 mock; v1.5
  reads from database). ≤ 120 LOC.
- `crates/argo-redact/src/lib_tests.rs` — NEW.
  ≥ 14 unit tests (one per pattern positive,
  one negative; one per RBAC permission; plus
  grant expiry + audit emission). ≤ 400 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs`
  — narrowly scoped: wrap the items array with
  `redact_approval_row` before serialization.
  ≤ 420 LOC after (J1 baseline ~400).
- `Cargo.toml` (workspace root) — narrowly: 1
  member addition.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs`
  — locked (HTML inbox stays unredacted in v1;
  legacy consumers handle redaction themselves).
- `crates/argo-channels/**` — read-only.
- `crates/argo-identity/**` — read-only.
- Other Track-J handler files.

## Acceptance criteria

- [ ] NEW crate `argo-redact` compiles standalone.
- [ ] `redact_text("Contact me at sam@acmeco.com",
      RedactionPolicy::default())` returns
      `"Contact me at [EMAIL]"`.
- [ ] `redact_text(..., RedactionPolicy { email:
      Show, ... })` returns the raw text.
- [ ] Phone, Luhn-valid-card, IBAN, IRD all
      mask per-policy.
- [ ] Luhn validator rejects pattern-matching
      but Luhn-invalid sequences (no false
      positives on long order numbers).
- [ ] `redact_approval_row(row, principal)`:
      - Pulls principal's `permissions.unredacted_*`
        fields.
      - Pulls active grants (if any).
      - Combines: permission OR active grant →
        Show; else → Mask.
      - Redacts `detail`, `consequence_if_yes`,
        `consequence_if_no`, `reason` fields.
- [ ] Audit row emitted on each redacted-row
      read: `event_type = "approval.row.redacted"`,
      payload includes `actor_id`, `approval_id`,
      `categories_masked: [...]`, `grant_id?`.
- [ ] J1's GET response wraps each item via
      `redact_approval_row`.
- [ ] Production builds enforce the fence (no
      `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` analog
      on the server side — server is authoritative).
- [ ] 14 unit tests pass.
- [ ] `cargo build --workspace` clean.
- [ ] `cargo clippy --workspace --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J2-gateway-approvals-pii-redaction-server-side`
- PR title: `redact: server-side PII fence + wire into approvals JSON (J2)`

## Notes for the implementer

- **Rust port of E1-e's TypeScript patterns.**
  Use the same regex strings + Luhn algorithm.
  Side-by-side test fixtures across both
  layers ensure both fences mask identically.

- **`RedactionPolicy` shape:**
  ```rust
  pub struct RedactionPolicy {
      pub email: PolicyMode,
      pub phone: PolicyMode,
      pub card: PolicyMode,
      pub iban: PolicyMode,
      pub ird: PolicyMode,
  }

  pub enum PolicyMode { Mask, Show }

  impl Default for RedactionPolicy {
      fn default() -> Self {
          // strict-by-default
          Self {
              email: PolicyMode::Mask,
              phone: PolicyMode::Mask,
              card: PolicyMode::Mask,
              iban: PolicyMode::Mask,
              ird: PolicyMode::Mask,
          }
      }
  }
  ```

- **Permission → policy mapping.** Extend
  `Principal` (in `argo-identity`) with optional
  `permissions: HashMap<String, bool>` lookup.
  Per-category:
  - `permissions.unredacted_email = true` → Show
  - missing or false → Mask
  Similar for phone/card/iban/ird.

- **Grants lookup.** Pull active grants for the
  principal:
  ```sql
  SELECT category, expires_at
  FROM elevated_view_grant
  WHERE principal_id = $1
    AND expires_at > NOW();
  ```
  v1 mock returns empty (no grants yet). J16
  (gateway-elevated-view-request-endpoint) lands
  the table + endpoint that issues grants.

- **Sentinel format alignment.** `[EMAIL]`,
  `[PHONE]`, `[CARD]`, `[IBAN]`, `[IRD]` —
  byte-identical to E1-e's frontend sentinels.
  Ensures grep-ability + tests can assert across
  layers.

- **Performance.** Regex compilation is expensive;
  use `lazy_static!` for the compiled patterns
  (or `once_cell::sync::Lazy`). Per-row
  redaction stays < 100µs for typical detail
  lengths.

- **Audit row content.** Don't include the
  redacted strings themselves in the audit
  payload (that defeats the point). Just
  `categories_masked: ["email", "phone"]` so
  operators can see what was masked.

- **Don't introduce a new `unsafe` block.**
  Pure Rust; no FFI.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-pii-llm-classifier` —
  LLM-based NER for names / addresses / free-text.
- `mission-enterprise-pii-classification-cache`
  — per-row classification cache for v1.5
  performance.
