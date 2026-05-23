# Eval — MCP connector real-vs-stub inventory (29 connectors)

**Brief**: `eval-mcp-connector-real-vs-stub-inventory`
**Authored against**: `origin/main` `7a00c577d` on `2026-05-23`.
**Scope**: read-only audit of every `crates/argo-mcp-connector-*/src/**`, classifying each connector by (a) does `connect()` actually hit the upstream API, (b) is `request()` keyed by per-tenant OAuth tokens, (c) is there at least one in-crate smoke test that does not hit real network, (d) is the connector operator-configurable from the SPA today, (e) is it in a published pricing tier — and a verdict tag of REAL / STUB-OK / STUB-NEEDS-WORK / DEFERRED-v1.x.
**Severity scale**: P0 — marketing mis-advertisement (looks real, isn't); P1 — operator-configurability gap; P2 — smoke-test gap on a REAL connector; P3 — minor.
**Companion briefs**: `eval-mcp-connector-credential-scoping-sweep` (cred-axis sibling, MERGED 2026-05-23), `mission-mcp-connector-token-cache-tenant-keyed` (P0 cache-leak fix, MERGED via #2103 tick-91).
**Source handoff**: gap-manifest row "27 → **29** MCP connectors" — *"Real-vs-stub inventory still owed."*

---

## TL;DR

Every shipped connector is **REAL** on the substrate axis (cell a). All 29 connectors include a `reqwest` HTTP client built against a real upstream production base URL (api.airtable.com, slack.com/api, api.stripe.com, graph.microsoft.com/v1.0, etc.), and a per-method dispatch that builds an HTTP request and parses an HTTP response. There are **zero** placeholder shells that return canned data instead of issuing a real request. The single exception is `imessage`, which is REAL but proxies through a [Blue Bubbles](https://bluebubbles.app) self-hosted iMessage relay rather than an Apple-owned HTTP API — semantically real, structurally indirect.

Every connector has at least one in-crate smoke test that exercises the request/response path without hitting real network (cell c). The dominant pattern is a hand-rolled one-shot `TcpListener` + assertion-on-raw-bytes (used by 27 of 29). Two connectors use richer harnesses: `onedrive` uses `wiremock`; `gcal` ships a dedicated cross-tenant cache-test sibling file (`client_cache_tenant_tests.rs`) covering the legacy-cache migration path. Test counts range from 2 (notion) to 14 (stripe).

Cell (b) — tenant-keyed credentials — is **NO** for **every connector**, as already enumerated by the cred-axis sibling audit (`eval-mcp-connector-credential-scoping-sweep`, 2026-05-23). The single canonical lookup is `argo_config::cred::get_unscoped(name)`, which by its own rustdoc *"bypasses tenancy isolation."* This is not a per-connector finding — it is a workspace-wide property tracked by the already-authored brief `mission-cred-get-tenant-parameter`. The 3 × P0 cwd-cache leaks (gcal, gmail, salesforce) are fixed by `mission-mcp-connector-token-cache-tenant-keyed` (MERGED #2103). This audit does not re-litigate the cred-axis findings.

Cell (d) — operator-configurable — is the **headline gap**. The brief asked us to cross-check `web/app/settings/connectors/page.tsx`. **That file does not exist.** The only operator-facing connector catalog in the workspace is `web/lib/connectors/registry.ts`, a hardcoded 10-entry hire-wizard catalog used by the new-worker flow. Of the 10 ids in `CONNECTORS`:
- 6 are backed by a connector crate in this audit: `gmail`, `google-calendar` (→ `gcal`), `slack`, `stripe`, `linear`, `github` (→ `github-issues`).
- 4 are computer-use or external-OAuth ids with no connector crate: `facebook-marketplace`, `gumtree`, `ebay`, `intercom`.

That leaves **23 of 29 connector crates with zero operator-facing surface** — they compile, test, and are wired into the workspace, but no user can choose them when hiring a worker, and there is no `/settings/connectors` page where one could connect an account to them. This is the load-bearing finding of the audit.

Cell (e) — tier — is **unanswerable from the workspace today**. There is no `docs/skycot/marketing/PRICING.md` or equivalent published tier doc. Marketing has only `MARKETING_AGENT_CHARTER.md` and `VOICE_PROFILE.md`. Every cell-e is marked `?` and surfaced as a follow-up.

Counts:

- **29 REAL connectors** (substrate-honest, hit a real upstream API).
- **0 STUB-NEEDS-WORK** (no connector marketing-mis-advertises itself as real-while-being-fake).
- **0 STUB-OK** (no admitted placeholder shells remain in the workspace; the 4 consumer stubs — signal, telegram, mailgun, resend — and 3 enterprise stubs — apollo, zoominfo, zoom — flagged in `project_v1_strategic_decisions_2026_05_14.md` have **already been deleted** from origin/main and do not appear in `ls crates/argo-mcp-connector-*`).
- **6 REAL + operator-visible via hire wizard** (gmail, gcal, slack, stripe, linear, github-issues).
- **23 REAL + operator-INVISIBLE** (no hire-wizard entry, no `/settings/connectors` page — these are the candidates for follow-up brief `web-connector-<name>-settings-page-row` per the brief).
- **1 REAL but tier-DEFERRED-v1.x candidate** (imessage — Blue Bubbles relay, macOS-only, consumer-leaning; flag for operator pricing call).
- **1 helper crate** (`argo-mcp-connector-shared` — retry policy + HTTP defaults, no connector surface).

### Headline findings

| # | Finding | Severity | Fix-brief recommendation |
|---|---|---|---|
| F-01 | **23 REAL connectors with no operator-facing surface.** airtable, asana, calendly, clickup, discord, figma, fireflies, google-meet, hubspot, imessage, jira, mailchimp, monday, notion, onedrive, pagerduty, postmark, salesforce, sendgrid, stripe-events, trello, typeform, zendesk — each is a working HTTP client with smoke tests, but no one can connect an account to them. They are dead inventory until either the hire-wizard catalog (`web/lib/connectors/registry.ts`) is broadened, or a `/settings/connectors` page is built. | 🟡 P1 | Single rollup brief: `web-settings-connectors-page-build` (NEW). Build the missing settings route; render rows for every connector advertised in a workspace-managed allowlist. Alternative: expand `web/lib/connectors/registry.ts` to the full 29 ids and let the hire wizard surface them. Operator-decisional which UX comes first. |
| F-02 | **`web/app/settings/connectors/page.tsx` does not exist.** The brief assumed this surface; the audit confirms it has not been built. Until F-01 lands, the answer to "can the operator connect a Slack workspace?" is "edit env vars and restart the gateway" — not a marketplace experience. | 🟡 P1 | Same brief as F-01. |
| F-03 | **No published pricing tier doc.** Cell (e) is unanswerable for every connector because `docs/skycot/marketing/PRICING.md` does not exist and the operator's enterprise-vs-consumer guidance from `project_v1_strategic_decisions_2026_05_14.md` is the only signal. The deleted-stubs decision (consumer-redundant out, enterprise placeholders in) is the closest thing to a tier policy in the workspace. | 🟢 P3 | `mission-pricing-tier-doc-author` (NEW). Operator-input brief — engineering cannot infer tiers without product-marketing input. Not a substrate issue. |
| F-04 | **`onedrive` has no production caller yet.** `OneDriveConnector::new(oauth_token: String)` accepts the bearer as a constructor argument; there is no `from_env` shortcut and no callsite in the workspace that constructs it. The crate is REAL (real HTTP client, real Graph endpoints, real wiremock smoke tests), but architecturally it is a library awaiting wire-up. Already flagged in `eval-mcp-connector-credential-scoping-sweep` (F-05). | 🟢 P3 | No new brief — covered by `mission-cred-get-tenant-parameter` once that defines the tenant-aware caller pattern. |
| F-05 | **`notion` has the sparsest smoke-test coverage** (2 tests, vs the 7-14 cluster everywhere else). The crate is REAL but only one tool path (and one regression) is pinned. | 🟢 P2 | `eval-notion-add-smoke-tests` (NEW). Add tool-by-tool happy-path coverage to match the airtable/asana/clickup baseline. |
| F-06 | **`slack` has 3 smoke tests** — below the per-connector median but the lowest among the operator-visible 6. | 🟢 P2 | `eval-slack-add-smoke-tests` (NEW). Equivalent to F-05. |
| F-07 | **`imessage` semantic surface ≠ Apple iMessage API.** The crate uses `BlueBubblesClient` (a 3rd-party self-hosted iMessage relay) rather than calling an Apple-owned endpoint — Apple does not publish a public iMessage API. This is substrate-honest at the crate level but is a marketing-honesty question: an enterprise customer reading "iMessage connector" expects Apple-managed delivery, not "deploy a Blue Bubbles server on a Mac mini in your office." | 🟢 P3 | No fix-brief; document in `web/lib/connectors/registry.ts` description when (if) imessage is added to the hire wizard. Operator should decide whether the connector ships as `imessage` or `imessage-via-bluebubbles`. |
| F-08 | **`stripe` + `stripe-events` are paired**, with `stripe-events` a read-only sibling of `stripe`. Operator-visible catalog has only `stripe`. Documented intentionally — `stripe-events` is meant for trigger/webhook flows, not direct hire-wizard exposure. No fix needed; noting for completeness. | 🟢 — | No brief. |

### Recommended fix-brief queue (post-this-audit)

1. **🟡 P1 — `web-settings-connectors-page-build`** (NEW). Owners: claude (UI). Touches: `web/app/(app)/settings/connectors/page.tsx` (NEW), `web/lib/connectors/registry.ts` (broaden to 29 ids or move to a workspace-fetched allowlist per the `mission-enterprise-connector-allowlist` plan already commented in registry.ts). Resolves F-01 + F-02 as one route.
2. **🟢 P3 — `mission-pricing-tier-doc-author`** (NEW). Owners: operator. Touches: `docs/skycot/marketing/PRICING.md` (NEW). Resolves F-03; unlocks the tier column of any future connector-marketplace UI.
3. **🟢 P2 — `eval-notion-add-smoke-tests`** (NEW). Owners: codex. Touches: `crates/argo-mcp-connector-notion/src/**`. Tool-by-tool happy-path coverage to match peer baseline. Resolves F-05.
4. **🟢 P2 — `eval-slack-add-smoke-tests`** (NEW). Owners: codex. Touches: `crates/argo-mcp-connector-slack/src/**`. Same shape as F-05 fix. Resolves F-06.

🚨 **ESCALATION (operator-eyes)**: No P0 surfaces. The headline operator-call is F-01 — 23 REAL connectors are dead inventory until a `/settings/connectors` route or broadened hire-wizard catalog exists. v1.0 demo is **macOS single-tenant** with a fixed cohort, so this does **not** block the v1.0 ship gate, but it is the largest "compiled-and-tested but unusable" surface in the workspace and the most credible candidate for the next operator-facing UX investment.

---

## Per-connector inventory matrix

Reading order: cell **a** (real connect), **b** (tenant-keyed), **c** (smoke test), **d** (operator-configurable), **e** (tier), **verdict**.

**Cell-b shorthand.** Every connector's answer is the same: **NO** — `cred::get_unscoped(...)` is the canonical read, no per-tenant binding. The cred-axis sibling audit (`eval-mcp-connector-credential-scoping-sweep`, MERGED 2026-05-23) is the source of truth; fix lands via `mission-cred-get-tenant-parameter` (and `mission-mcp-connector-token-cache-tenant-keyed` for the 3 cwd-cache subset). Cell b is collapsed below to "NO — sibling audit" rather than repeated for 29 rows.

**Cell-e shorthand.** Every connector's answer is `?` — no published pricing tier doc (F-03). Collapsed in the table.

**Cell-d signal source.** `web/lib/connectors/registry.ts` (the only operator-facing connector catalog in the workspace, used by the hire wizard). **YES** = the connector id appears in `CONNECTORS`; **NO** = it does not.

| # | Connector | a. real connect | c. smoke test | d. op-configurable | Verdict |
|---|---|---|---|---|---|
| 1 | airtable | YES — `reqwest::Client` → `https://api.airtable.com/v0` | YES — 9 tests, one-shot `TcpListener` pattern | NO | **REAL** — F-01 candidate |
| 2 | asana | YES — `reqwest::Client` → `https://app.asana.com/api/1.0` | YES — 8 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 3 | calendly | YES — `reqwest::Client` → `https://api.calendly.com` | YES — 7 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 4 | clickup | YES — `reqwest::Client` → `https://api.clickup.com/api/v2` | YES — 10 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 5 | discord | YES — `reqwest::Client` → `https://discord.com/api/v10` | YES — 7 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 6 | figma | YES — `reqwest::Client` → `https://api.figma.com/v1` | YES — 8 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 7 | fireflies | YES — `reqwest::Client` → `https://api.fireflies.ai/graphql` | YES — 10 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 8 | gcal | YES — `reqwest::Client` → `https://www.googleapis.com/calendar/v3`; OAuth refresh via `https://oauth2.googleapis.com/token` | YES — 8 in-crate tests + dedicated `client_cache_tenant_tests.rs` sibling (cross-tenant negative test for the cache-keying fix) | YES (`google-calendar` in registry) | **REAL** — operator-visible |
| 9 | github-issues | YES — `reqwest::Client` → GitHub REST (base URL in env) | YES — 9 tests, one-shot listener | YES (`github` in registry) | **REAL** — operator-visible |
| 10 | gmail | YES — `reqwest::Client` → `https://gmail.googleapis.com`; OAuth via `https://accounts.google.com/o/oauth2/v2/auth` + `https://oauth2.googleapis.com/token` | YES — 12 tests, one-shot listener | YES (`gmail` in registry) | **REAL** — operator-visible |
| 11 | google-meet | YES — `reqwest::Client` → `https://meet.googleapis.com/v2` | YES — 7 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 12 | hubspot | YES — `reqwest::Client` → `https://api.hubapi.com` | YES — 8 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 13 | imessage | YES — via `BlueBubblesClient` (3rd-party self-hosted iMessage relay; no direct Apple API) | YES — 7 tests | NO | **REAL** — DEFERRED-v1.x-candidate (F-07 — semantic mismatch with "iMessage connector" marketing expectation) |
| 14 | jira | YES — `reqwest::Client` → `https://<tenant>.atlassian.net` (per-instance subdomain) | YES — 9 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 15 | linear | YES — `reqwest::Client` → `https://api.linear.app/graphql` | YES — 10 tests, one-shot listener | YES (`linear` in registry) | **REAL** — operator-visible |
| 16 | mailchimp | YES — `reqwest::Client` → `https://<dc>.api.mailchimp.com/3.0` | YES — 8 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 17 | monday | YES — `reqwest::Client` → `https://api.monday.com/v2` | YES — 10 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 18 | notion | YES — `reqwest::Client` → `https://api.notion.com/v1` | YES — 2 tests (F-05 — sparsest coverage) | NO | **REAL** — F-01 + F-05 candidate |
| 19 | onedrive | YES — `reqwest::Client` → `https://graph.microsoft.com/v1.0` (constructor takes bearer; no `from_env`) | YES — 9 tests, **`wiremock`** harness | NO | **REAL** — F-01 + F-04 candidate (no production caller yet) |
| 20 | pagerduty | YES — `reqwest::Client` → `https://api.pagerduty.com` | YES — 9 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 21 | postmark | YES — `reqwest::Client` → `https://api.postmarkapp.com` | YES — 9 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 22 | salesforce | YES — `reqwest::Client` → `https://<instance>.my.salesforce.com`; OAuth handshake in `auth.rs` | YES — 11 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 23 | sendgrid | YES — `reqwest::Client` → `https://api.sendgrid.com/v3` | YES — 7 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 24 | slack | YES — `reqwest::Client` → `https://slack.com/api` | YES — 3 tests (F-06 — sparse for an operator-visible connector) | YES (`slack` in registry) | **REAL** — operator-visible + F-06 candidate |
| 25 | stripe | YES — `reqwest::Client` → `https://api.stripe.com` | YES — 14 tests, one-shot listener (densest coverage) | YES (`stripe` in registry) | **REAL** — operator-visible |
| 26 | stripe-events | YES — `reqwest::Client` → `https://api.stripe.com` (read-only sibling of `stripe`) | YES — 9 tests, one-shot listener | NO (intentional — read-only sibling) | **REAL** — paired with `stripe` (F-08, no fix needed) |
| 27 | trello | YES — `reqwest::Client` → `https://api.trello.com/1` | YES — 10 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 28 | typeform | YES — `reqwest::Client` → `https://api.typeform.com` | YES — 8 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| 29 | zendesk | YES — `reqwest::Client` → `https://<subdomain>.zendesk.com` | YES — 8 tests, one-shot listener | NO | **REAL** — F-01 candidate |
| H | argo-mcp-connector-shared | N/A — helper crate (retry policy + HTTP defaults; `RetryPolicy`, `default_http_policy`) | YES — 11 tests against the retry helper itself | N/A | **HELPER** (not a connector) |

---

## Verdict counts

- **REAL**: 29 / 29
- **STUB-OK** (admitted placeholder, marketing-honest): 0
- **STUB-NEEDS-WORK** (looks real but isn't): 0
- **DEFERRED-v1.x**: 0 in the workspace (the 7 originally-flagged stubs — signal, telegram, mailgun, resend, apollo, zoominfo, zoom — have already been **deleted** from origin/main per the v1 strategic-decisions doc; `imessage` is the only candidate for an operator pricing/positioning call, per F-07)

## Per-connector commentary (highlights only — substrate is uniform; only the deviations get a paragraph)

**airtable / asana / calendly / clickup / discord / figma / fireflies / google-meet / hubspot / jira / mailchimp / monday / pagerduty / postmark / sendgrid / trello / typeform / zendesk** — the substrate-uniform majority. Each crate is `auth.rs` + `client.rs` + `lib.rs` + `main.rs` + `schemas.rs` + `service.rs`, builds a `reqwest::Client` against the real upstream production base URL, dispatches per-tool methods that issue real HTTP requests + parse responses, and ships an in-crate `mod tests` block with 7-10 `#[tokio::test]` cases driven by a hand-rolled one-shot `TcpListener` that asserts on the raw HTTP request bytes (Authorization header, path, body shape) and returns a canned response. Coverage spans happy-path, query-string encoding, body shape, and error mapping. None of these connectors appear in the operator-facing catalog at `web/lib/connectors/registry.ts`. They are dead inventory until F-01 ships.

**gcal** — only connector with a dedicated cache-test sibling file (`src/client_cache_tenant_tests.rs`) ensuring the tenant-keyed disk cache path. Real OAuth refresh handshake against `oauth2.googleapis.com/token`. The fixed cwd-cache leak (P0 from cred-audit) was here; the tests pin the fix. **Operator-visible** via the `google-calendar` registry id.

**github-issues** — base URL pulled from env (typically `https://api.github.com` but configurable for GHES). 9 in-crate tests via the one-shot listener. **Operator-visible** via the `github` registry id.

**gmail** — densest OAuth surface in the audit: refresh flow against `oauth2.googleapis.com/token`, auth-initiation against `accounts.google.com/o/oauth2/v2/auth`, request layer against `gmail.googleapis.com`. 12 in-crate tests. One of the 3 connectors with cwd-cache leak (P0 from cred-audit), pinned via `mission-mcp-connector-token-cache-tenant-keyed`. **Operator-visible** via the `gmail` registry id.

**imessage** — singular substrate-uniformity break. Wraps `argo_bluebubbles_client::BlueBubblesClient` rather than building a `reqwest::Client` itself. Blue Bubbles is a self-hosted iMessage relay (an HTTP API that an operator runs on a Mac). The connector is **substrate-honest** at the crate level (it really does send iMessages, end-to-end). But "iMessage connector" as a marketing label sets an expectation that the customer doesn't need to operate any infrastructure — for Blue Bubbles, they do. This is the only marketing-honesty issue surfaced by the audit (F-07), and the right resolution is a clearer name (`imessage-via-bluebubbles`) when/if it surfaces in the SPA catalog. Strictly, the crate is REAL; the verdict is REAL with a positioning footnote.

**linear** — `https://api.linear.app/graphql` (the only GraphQL endpoint among the operator-visible 6). 10 in-crate tests. **Operator-visible** via the `linear` registry id.

**notion** — the test-coverage outlier (2 tests vs the 7-14 cluster). Both tests are real one-shot-listener tests; just thin coverage. F-05 — file `eval-notion-add-smoke-tests` to bring it to peer baseline. Otherwise REAL.

**onedrive** — the architectural outlier. Uses `wiremock` instead of the hand-rolled listener pattern (9 tests, all proper HTTP mocking against `graph.microsoft.com/v1.0`). Lib.rs documents two follow-ups (`mission-mcp-connector-onedrive-sharepoint`, `-onedrive-refresh`) — token refresh is explicitly out-of-scope. No `from_env` and no production caller in the workspace; the eventual caller will inherit the workspace-wide `cred::get_unscoped` leak unless built tenant-aware. F-04. REAL but library-only-until-wired.

**salesforce** — only connector other than `gcal`/`gmail` with a real OAuth handshake (`src/auth.rs`). One of the 3 connectors with cwd-cache leak (P0 from cred-audit). 11 in-crate tests. NOT in the operator-facing registry (would be a high-value enterprise add when F-01 lands).

**slack** — REAL, operator-visible, but only 3 in-crate tests (well below the median and the lowest among the operator-visible 6). F-06 — file `eval-slack-add-smoke-tests`. The 25-line carriage-return comment in `slack.rs` documenting the tenant-unscoped read is the cleanest example of the discipline-without-fix pattern noted in the cred-axis audit.

**stripe / stripe-events** — paired by design: `stripe` is the action/dispatch surface (14 in-crate tests, densest coverage in the audit), `stripe-events` is a read-only sibling for trigger/webhook flows (9 tests). Only `stripe` is in the operator catalog; this is intentional. F-08, no fix needed.

**argo-mcp-connector-shared** — not a connector. Exports `RetryPolicy` and `default_http_policy()` used by `onedrive` (and any connector adopting wiremock-style harnesses going forward). 11 in-crate tests against the retry helper itself.

---

## Methodology

For each `crates/argo-mcp-connector-*` directory:

1. `ls src/` to enumerate file layout (auth/client/lib/main/schemas/service vs deviations).
2. `grep -E 'reqwest::Client::builder|reqwest::Client::new'` to confirm a real HTTP client.
3. `grep -roh 'https://[a-zA-Z0-9./_-]*'` to pull the real upstream base URL out of `src/`.
4. `grep -rh 'cred::get'` to confirm cell (b) is the workspace-uniform `cred::get_unscoped` pattern (deferred to sibling audit).
5. `grep -rh '#\[tokio::test\]\|#\[test\]'` count + `grep -rh 'mockito::\|wiremock::\|MockServer'` to identify smoke-test framework. The hand-rolled `TcpListener` pattern was identified by reading `airtable/src/client.rs` end-to-end and confirmed in spot-checks of 4 other crates (zendesk, slack, asana, stripe).
6. `grep -l '<connector-name>'` in `web/lib/connectors/registry.ts` for cell (d). The brief's assumed `web/app/settings/connectors/page.tsx` was confirmed not to exist via `find web/app/(app)/settings -name 'page.tsx'`.
7. `find docs/skycot/marketing -name '*PRICING*'` to confirm no tier doc (cell e).

The 29-connector + 1-helper count matches `ls crates/argo-mcp-connector-* | wc -l = 30` against `origin/main` `7a00c577d`. A structural pin test (`crates/argo-mcp/tests/connector_inventory_pin_test.rs`) was added to fail loudly if a future commit changes this count without updating the matrix.

---

## Notes on the brief's premise

The brief framed the audit as catching connectors that "mistakenly look real to the marketplace." The audit found **zero** such connectors — every shipped connector hits a real upstream API. The reason is that the 7 known-stub connectors (signal, telegram, mailgun, resend, apollo, zoominfo, zoom) were **deleted between operator memory and this audit's run** — they no longer appear in `crates/argo-mcp-connector-*`. The substrate-vs-stub axis is therefore structurally green; the real-vs-stub gap from the gap-manifest collapses to "yes, every connector still in the workspace is real."

The audit's substantive value is the **operator-configurability gap** (F-01): 23 of 29 REAL connectors have no operator-facing surface, which is a marketing-vs-product mismatch even if not a marketing-vs-substrate one. That finding is the load-bearing recommendation queued for the conductor.
