# Stream E1-e — Approvals inbox: PII redaction layer (defense-in-depth, client-side)

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED; E1-a-approvals-gateway-wireup MERGED.
**Authored against**: origin/main `9ab5a79c` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E1-e-approvals-pii-redaction`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E1.
**Operator review flag**: this brief proposes the **default PII patterns** that get masked. See "Open questions" — this is where the operator picks the default conservatism level before merge.

## Why

The auditor persona
([auditor.md §"To design for"](../../skycot/ux-research/auditor.md))
specifies that approval-card bodies render PII-masked unless
the viewer's role grants unredacted view. Without this, a
delegator's screen-share or a contractor with limited
permission can expose customer data.

This brief lands the client-side **defense-in-depth** layer:
PII patterns matched against card content are replaced with
sentinels at render time. The viewer's RBAC controls whether
to apply the mask or render raw.

**This brief is NOT the only line of defense.** Real PII
protection requires server-side redaction in the gateway —
data arriving at the client is already in browser memory.
The companion backend brief
(`mission-gateway-approvals-pii-redaction-server-side`, to
be authored) handles that. This client brief is one layer
of defense among several.

## Scope

- A `redactPII()` library function that takes a string and
  a `RedactionPolicy` and returns the redacted version
  with sentinels (`[EMAIL]`, `[PHONE]`, `[CARD]`, etc.).
- A `RedactionPolicy` type with fields per category:
  `email: "mask" | "show"`, `phone: ...`, etc.
- A `useRedactionPolicy()` hook that returns the current
  viewer's effective policy. v1 resolution:
  - If user's RBAC includes `unredacted-view` permission
    → all fields `"show"`.
  - Else → conservative defaults (see below).
  - If `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → all `"show"`
    (dev mode trusts the operator).
- A `<RedactedText>` component that wraps any text node and
  applies the policy via the hook.
- Wire into `ApprovalCard` to redact `detail`,
  `consequenceIfYes`, `consequenceIfNo`, `reason`.
- An audit emission when a viewer hits the
  "Request elevated view" CTA (the user is asking to see
  unredacted; the request itself is logged).

## Default conservative patterns (v1)

| Category | Default | Pattern |
|---|---|---|
| Email | mask | RFC 5322-ish: `[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}` |
| Phone (NZ + intl) | mask | `\+?[0-9 ()-]{7,}` (gates on min digit count = 7) |
| Credit card | mask | Luhn-validated 13-19 digit sequences |
| Bank IBAN | mask | `[A-Z]{2}[0-9]{2}[A-Z0-9]{4,}` |
| NZ IRD number | mask | `\b\d{2,3}-\d{3}-\d{3}\b` |
| Mailing address | **NOT masked by v1** | Out of scope; needs LLM detection |
| Names | **NOT masked by v1** | Need named-entity recognition; deferred |
| Order numbers | **NOT masked by v1** | Operator-configurable in v1.5 |
| Free-text PII | **NOT masked by v1** | Deferred to server-side LLM redaction |

The defaults aim for **high precision, low recall**: only
mask what is obviously sensitive and pattern-matchable.
False-positives on important data destroy trust faster than
false-negatives on edge cases. The operator can tune
later.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/components/skycot/ApprovalCard.tsx`
  (the wiring point)

## Files you may touch

- `web/lib/redaction/patterns.ts` — NEW. The regex
  patterns + Luhn validator. ≤ 150 LOC.
- `web/lib/redaction/redact.ts` — NEW. `redactPII(text,
  policy)` function. ≤ 200 LOC.
- `web/lib/redaction/redact.test.ts` — NEW. ≥ 12 tests
  covering each pattern's positive + negative cases +
  policy-respecting toggle. ≤ 300 LOC.
- `web/lib/hooks/use-redaction-policy.ts` — NEW. Resolves
  the viewer's effective policy from session + auth-bypass
  flag. ≤ 100 LOC.
- `web/components/skycot/RedactedText.tsx` — NEW. Wraps a
  string with the redacted output. Tooltip on a masked
  sentinel shows: `"Masked — your role doesn't include
  unredacted view"`. ≤ 100 LOC.
- `web/components/skycot/ApprovalCard.tsx` — narrowly
  scoped: wrap `detail`, `consequenceIfYes`,
  `consequenceIfNo`, `reason` text nodes in
  `<RedactedText>`. Keep all other structure unchanged.
  ≤ 400 LOC after changes (was ~350).
- `web/components/skycot/ApprovalCard.test.tsx` —
  narrowly scoped: add 3 tests:
  - card text renders raw when policy = all show.
  - card text renders sentinels when policy = mask all.
  - hover sentinel surfaces the tooltip.
  Existing 6 tests must still pass.
- `web/components/skycot/RequestElevatedView.tsx` — NEW.
  A small inline link `"Request elevated view"` shown
  ONLY when at least one redaction was applied to the
  visible cards. Tapping fires an audit-emitting POST
  to the gateway. ≤ 120 LOC.

## Files you must NOT touch

- `web/lib/api/approvals.ts` — locked.
- `web/lib/sample-approvals.ts` — locked. (Test
  fixtures with PII should live in
  `web/lib/redaction/redact.test.ts`.)
- Other E1 sibling deliverables.
- `crates/**` — server-side redaction is a separate
  brief.

## Acceptance criteria

- [ ] `redactPII("contact me at sam@acmeco.com", {
      email: "mask" })` returns
      `"contact me at [EMAIL]"`.
- [ ] `redactPII(..., { email: "show" })` returns the
      original.
- [ ] Phone numbers with `+64 21 ...` patterns are
      masked → `[PHONE]`.
- [ ] Credit-card Luhn-valid patterns are masked →
      `[CARD]`.
- [ ] False positives on common non-PII patterns are
      avoided (e.g. "12345" is not masked as a phone;
      `1-800-555-0199` IS masked).
- [ ] `<RedactedText>` respects the viewer's policy.
- [ ] In approval cards, the detail / consequence /
      reason fields are redacted per policy.
- [ ] The page surfaces a `<RequestElevatedView>` link
      ONLY when at least one redaction is active.
- [ ] Clicking that link fires an audit POST to the
      gateway (path:
      `/api/v1/audit/elevated-view-request`).
- [ ] `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` short-circuits
      to all-show.
- [ ] 12 redact unit tests + 3 ApprovalCard tests pass.
- [ ] Existing 6 ApprovalCard tests still pass.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 4KB First
      Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E1-e-approvals-pii-redaction`
- PR title: `web(approvals): client-side PII redaction layer (E1-e)`

## Notes for the implementer

- **Why client-side redaction at all if server is the
  real fence?** Defense in depth. The server-side brief
  may not land at v1 ship. Even when it does, client-side
  masking guards against scenarios where data is loaded
  by a higher-privileged session and viewed by a lower-
  privileged user later (e.g. session sharing, screen
  recording with delayed playback). It's belt-and-braces.

- **Sentinel format.** Use `[CATEGORY]` not `***` or
  `<redacted>`. Two reasons:
  - Operators can tell *what* was masked, not just *that*
    something was.
  - The Auditor persona can grep transcripts and
    distinguish "this masked email" from "this masked
    phone."
  Same shape across all categories: `[EMAIL]`,
  `[PHONE]`, `[CARD]`, `[IBAN]`, `[IRD]`.

- **Luhn validator for cards.** Implement a small
  function in `patterns.ts`. A pure-JS Luhn check is
  ~15 LOC. Reject sequences that pass the regex but fail
  Luhn — this kills false positives on long order
  numbers.

- **Phone number heuristic.** Min 7 digits, allow
  spaces / dashes / parens / leading `+`. Examples that
  must match: `+64 21 555 0123`, `(09) 555-0123`,
  `02125550123`. Examples that must NOT match: `1234`,
  `2026-05-11` (date), `$1,500.00` (currency).

- **Tooltip copy.** Strings: `"Masked — your role
  doesn't include unredacted view"`. If
  `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` the tooltip
  shouldn't appear because nothing is masked.

- **RequestElevatedView audit.** Single POST to the
  gateway. Body: `{ surface: "approvals-inbox",
  categories: ["email", "phone"], reason: "<optional
  free text the user types>" }`. v1 doesn't gate the
  unmask on the audit — the audit is just a record.
  v1.5 might add an admin-approval step.

- **What this brief does NOT do.**
  - Doesn't add server-side redaction. That's the
    sibling backend brief.
  - Doesn't add LLM-based PII detection (names,
    addresses, free-text). v1.5.
  - Doesn't add per-tenant configurable patterns. v1.5.
  - Doesn't apply to the audit timeline. E5-g handles
    that.

## Open questions for operator (review at E1-checkpoint)

`(needs operator judgment)`

1. **Default conservatism.** The v1 defaults mask only
   pattern-matchable PII (email, phone, card, IBAN, IRD).
   Names + addresses + free-text are NOT masked. Acceptable
   for v1, or does any vertical (e.g. healthcare-intake)
   need a stricter default?
2. **Sentinel format.** `[EMAIL]` vs `***@***.***` vs
   something else. Sticking with `[CATEGORY]` per the
   "tell auditors what was masked" argument; flag if a
   different shape preferred.
3. **Audit on elevated-view request.** v1 logs the
   request but doesn't gate the unmask. Should v1 also
   require admin approval before unmasking, or is
   logging enough?
4. **Where does the unredacted-view RBAC permission live
   on the role tree?** The brief assumes one named
   permission `unredacted-view`. Real RBAC may want
   per-category permissions (`unredacted-email`,
   `unredacted-phone`). Defer to v1.5?
5. **Should `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`
   short-circuit unmask in production builds?** Currently
   it does — bypass = dev mode = all-show. Confirm.

Plan to validate: ship, watch the elevated-view-request
audit rate for the first 30 days. If > 50% of viewers
request elevated view immediately, the defaults are too
aggressive.
