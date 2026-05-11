# Stream E5-h — Audit timeline: cross-system jump links (per-connector deep-link generators)

**Owner**: claude.
**Depends on**: E5-c MERGED (drill-in surface).
**Authored against**: origin/main `fc62f44d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E5-h-audit-cross-system-jump-links`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E5 brief 8 of 8 (LAST in E5; LAST in Track E author phase).

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Connector deep-link registry | Hardcoded per-connector generator functions (Stripe / Slack / GitHub / Linear / Salesforce / HubSpot / Zendesk / Jira) | Each connector's manifest declares its deep-link template; operator can add new connectors without code changes | `mission-enterprise-connector-deep-link-manifest` |
| URL safety | Strict origin allowlist (only known SaaS origins are rendered as links) | Same; v1 ships the allowlist | n/a |
| Click telemetry | Not collected | Optional anonymous "operator clicked Stripe link" metric per tenant for product analytics | `mission-enterprise-deep-link-telemetry` |
| Authentication continuity | Link opens in new tab; user logs into target system separately | SSO with the target system carries through (e.g. Stripe SAML chain) | `mission-enterprise-deep-link-sso-continuity` |

v1 is acceptable: deep links are user-experience
plumbing — they save the operator a manual copy-
paste, and the auditor a "what was this Stripe ID
for?" round trip. No security surface.

## Why

E5-c's drill-in mentions cross-system jump links —
"View in Stripe" / "View in Slack" / "View in
GitHub" — as a UX affordance. This brief implements
the per-connector generators that turn an audit-row
reference into a deep link.

For enterprise: operators investigating a Mariah-
processed invoice click `act_sam_001` → drill-in →
see the Stripe Refund `re_3O...` → click "View in
Stripe" → land on the Stripe dashboard for that
refund in one motion. Without this, every audit
investigation requires manual copy-paste-search.

## Scope

- NEW connector deep-link registry with v1 generators
  for 8 SaaS systems.
- `<CrossSystemLink>` component that detects
  references in an audit row's body / output / verb
  and renders inline "View in <System>" links.
- Reference detection rules:
  - **Stripe**: matches `re_<id>` /
    `ch_<id>` / `cus_<id>` / `in_<id>` / `pi_<id>`
    → links to `https://dashboard.stripe.com/<type>/
    <id>`.
  - **Slack**: matches `slack://channel/<id>/<msg-id>`
    or `https://*.slack.com/archives/...` → opens as-
    is.
  - **GitHub**: matches `https://github.com/<owner>/
    <repo>/issues|pull/<n>` → opens as-is.
  - **Linear**: matches `https://app.linear.app/`
    OR `LIN-<n>` → links to `https://app.linear.app/
    issue/LIN-<n>`.
  - **Salesforce**: matches `https://*.lightning.force.com/`
    OR `SF-<id>` (when origin is configured).
  - **HubSpot**: matches `https://app.hubspot.com/`
    OR `hubspot-deal-<id>` → constructs URL.
  - **Zendesk**: matches `https://*.zendesk.com/agent/
    tickets/<n>` OR `ZD-<n>`.
  - **Jira**: matches `https://*.atlassian.net/`
    OR `<PROJECT>-<n>` (when project key matches).
- All generators check origin against an allowlist
  before emitting any external URL.
- Generators emit `<a target="_blank" rel="noopener
  noreferrer">` with a `[↗]` icon suffix.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/components/skycot/audit/
  ActionOutputCard.tsx` (extended; locked to this
  brief).

## Files you may touch

- `web/lib/cross-system/registry.ts` — NEW. Connector
  registry with per-system metadata + generator
  functions. ≤ 350 LOC.
- `web/lib/cross-system/generators/stripe.ts` — NEW.
  ≤ 100 LOC.
- `web/lib/cross-system/generators/slack.ts` — NEW.
  ≤ 80 LOC.
- `web/lib/cross-system/generators/github.ts` — NEW.
  ≤ 80 LOC.
- `web/lib/cross-system/generators/linear.ts` — NEW.
  ≤ 80 LOC.
- `web/lib/cross-system/generators/salesforce.ts` —
  NEW. ≤ 100 LOC.
- `web/lib/cross-system/generators/hubspot.ts` —
  NEW. ≤ 100 LOC.
- `web/lib/cross-system/generators/zendesk.ts` —
  NEW. ≤ 80 LOC.
- `web/lib/cross-system/generators/jira.ts` — NEW.
  ≤ 100 LOC.
- `web/lib/cross-system/detect.ts` — NEW. Pure
  function: takes a string, returns ordered list
  of `{ start, end, system, deepLink }` matches.
  ≤ 250 LOC.
- `web/lib/cross-system/types.ts` — NEW. ≤ 80 LOC.
- `web/lib/cross-system/detect.test.ts` — NEW. ≥ 16
  tests (2 per system: positive + negative).
  ≤ 350 LOC.
- `web/components/skycot/audit/CrossSystemLink.tsx`
  — NEW. Renders inline link with [↗] icon. ≤ 150
  LOC.
- `web/components/skycot/audit/ActionOutputCard.tsx`
  — narrowly scoped: wraps text rendering with the
  detect function + CrossSystemLink. ≤ 350 LOC
  after.

## Files you must NOT touch

- E1-E4 / earlier-E5 files except the narrow
  ActionOutputCard extension.
- `crates/**` — sibling enterprise-tier brief for
  the manifest-driven registry.

## Acceptance criteria

- [ ] Registry exports 8 generators for v1.
- [ ] Each generator implements:
      ```ts
      interface ConnectorGenerator {
        system: string;                    // "Stripe"
        icon: string;                      // lucide icon name
        match: (text: string) => Match[];  // returns ordered matches
        renderLink: (match: Match) => { url: string; label: string };
        verifyOrigin: (url: string) => boolean;
      }
      ```
- [ ] `detect(text)` returns all matches across all
      generators, sorted by start position. No
      overlapping matches; later matches yielded
      to earlier.
- [ ] ActionOutputCard text rendering wraps detected
      references with `<CrossSystemLink>`:
      ```
      "Refund processed for re_3O7abcXYZ..."
                       ↑
                       <a> "View in Stripe ↗"
      ```
- [ ] CrossSystemLink:
      - Renders raw text + small `↗` icon.
      - Click opens `target="_blank" rel="noopener
        noreferrer"`.
      - Hover: subtle underline.
      - Tooltip: `"Opens in ${system}"`.
- [ ] Strict allowlist enforced:
      - Stripe links must start with
        `https://dashboard.stripe.com/`.
      - Slack: `https://*.slack.com/archives/`.
      - GitHub: `https://github.com/<owner>/<repo>/...`.
      - Linear: `https://app.linear.app/`.
      - Etc.
      - Any URL not matching the allowlist for its
        claimed system is NOT rendered as a link
        (raw text only).
- [ ] No false positives: text that *looks like* a
      Stripe ID (e.g. `"re_test"` in a chat message)
      but isn't preceded by a Stripe context is NOT
      auto-linked. Conservative matching.
- [ ] 16 unit tests pass (2 per system: matches +
      negative non-match).
- [ ] Playwright e2e: drill into an action with a
      Stripe reference, click the link, assert
      new-tab opens (test asserts the URL via
      Playwright's expect-newpage).
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 6KB
      First Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E5-h-audit-cross-system-jump-links`
- PR title: `web(audit): cross-system jump links (E5-h)`

## Notes for the implementer

- **Stripe ID prefixes** (v1 supported):
  - `re_` (refund) → `/refunds/<id>`
  - `ch_` (charge) → `/payments/<id>` (legacy)
  - `cus_` (customer) → `/customers/<id>`
  - `in_` (invoice) → `/invoices/<id>`
  - `pi_` (payment intent) → `/payments/<id>`
  - `pm_` (payment method) → `/payment-methods/<id>`

- **Context heuristics.** To avoid false positives,
  require either:
  - The match is in a structured field (
    `output.body.stripe_refund_id`) → always link.
  - The match is in free-text AND preceded by a
    keyword like "Stripe", "refund", "payment",
    "charge" within 80 chars → link.
  - The match is in a URL → link.

  Otherwise: render as plain text.

- **Linear handling.** Linear has two patterns:
  - URL: `https://app.linear.app/skycot/issue/SKY-123/...`
  - Bare ID: `SKY-123` (requires team-prefix
    configuration).

  v1 ships URL detection only; bare-ID detection
  is gated by per-tenant Linear-team-prefix
  config (out of scope for v1).

- **Salesforce / HubSpot / Zendesk / Jira.** All
  require operator-configured origin to detect.
  v1 reads origin from a workspace setting
  (placeholder; setting itself is out of scope —
  reads from a hardcoded list in
  `lib/cross-system/registry.ts`).

- **Security review.** Every generator's regex
  must:
  - Use `^` / `$` anchors where possible.
  - Escape special characters.
  - Limit length (max 200 chars per ID).
  - Not allow URLs in the path that could redirect
    (no `//` in path component).

- **Performance.** Detection runs over every audit-
  output rendering. Cache results per audit-detail
  load (TanStack Query handles this). 30+ matches
  per row is the realistic upper bound — under
  10ms for the detect function on a typical row.

- **Open-source friendly.** Generators are simple,
  testable, contributable. Operators can fork and
  add their own (e.g. internal tools). v1.5
  upstreams the manifest pattern.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-connector-deep-link-manifest`
  — manifest-driven registry (replaces hardcoded
  generators).
- `mission-enterprise-deep-link-telemetry` —
  anonymous click metrics.
- `mission-enterprise-deep-link-sso-continuity` —
  SSO-aware deep links.

## Track E author phase COMPLETE after this commits

- E0 (foundation)
- E1-a..f (approvals inbox — 6 briefs)
- E2-a..i (hire flow — 9 briefs)
- E3-a..e (team page — 5 briefs)
- E4-a..i (worker profile — 9 briefs)
- E5-a..h (audit timeline — 8 briefs)
- **Total: 38 frontend implementation briefs**

Next: conductor advisory for E2-E5 (the rest of Track
E beyond the E0/E1 advisory already authored), then
Track F (backend mission claim 4 fill-ins).
