# Wave-3 Audit: Competitor Parity vs /compare Claims

**Date:** 2026-05-20
**Auditor:** Wave-3 audit agent #5
**Scope:** read-only assessment of `~/code/argo/web/app/compare/page.tsx`
**Source ref:** `web/app/compare/page.tsx` (479 LOC) — 13 dimensions, 5 columns

## 1. Per-row defensibility scoring (1-10)

For each row I cross-checked the SKYCOT claim against the codebase and rated how well it survives a 30-min sales-call grilling from a buyer who knows the competitor space.

| # | Group | Question | SKYCOT claim | Defensibility | Codebase evidence | Risk |
|---|-------|----------|--------------|---------------|-------------------|------|
| 1 | Teammates, not tools | Named, with personality + responsibilities | yes | **6/10** | `web/components/skycot/worker/WorkerPersonalityCard.tsx:32-60` PERSONALITIES is a **static map keyed by worker name** in the UI; backend has no `personality`/`trait`/`signature_behavior` field (`grep -rn "personality\|signature_behavior" crates/argo-types/src` returns 0 hits). The data shape comment says "matches the future API contract". | **HIGH** — pretty UI, no system-of-record. If buyer asks "where do these traits live?" we have to admit they're cosmetic. |
| 2 | Teammates, not tools | Show up in Slack like a colleague | yes | **8/10** | `crates/argo-channels/src/slack_inbound.rs`, `crates/argo-mcp-connector-slack/src/slack.rs`, `crates/argo-cli/tests/slack_approval_roundtrip_e2e.rs` — Slack inbound + outbound approval roundtrip has an e2e test. Channel manifest lists slack as a first-class channel (`crates/argo-channels/src/manifest.rs`: `order = ["slack", "email", "webinbox"]`). Lindy also "yes" is fair. | LOW |
| 3 | Teammates, not tools | Earn trust over time + get promoted | yes | **9/10** | `crates/argo-skill/src/trust.rs` 5-rung TrustGrade ladder, `crates/argo-skill/src/store_trust_grade.rs`, `web/components/skycot/TrustRatchetCard.tsx` ("after N consecutive approvals … offers one-click promotion"), `web/app/(app)/workers/[workerId]/trust/page.tsx`. **Real shipped surface, end-to-end.** | LOW |
| 4 | Safe by default | Ask for approval before doing risky things | yes | **9/10** | `crates/argo-store/src/approvals.rs`, `crates/argo-orchestrator/src/approval_routing.rs`, `crates/argo-cli/src/handlers/approval_*`, 5 shipped policy presets at `crates/argo-cli/assets/approval_policies/` (auto-approve-low-risk, deny-network-egress, read-only-developer, require-human-for-finance, strict-production), Slack approval roundtrip e2e test. | LOW |
| 5 | Safe by default | Every action audit-logged with full lineage | yes | **10/10** | `crates/argo-audit/src/sink_chain.rs` hash chain (parent_hash → self_hash), `crates/argo-audit/src/verify.rs` `argo audit verify` CLI walks the chain and recomputes hashes, `crates/argo-audit/src/filter_dims.rs` denormalized columns with drift detection. **This is the strongest factual claim on the page.** | LOW |
| 6 | Safe by default | Freeze a worker in one click | yes | **9/10** | `crates/argo-store/src/worker_freeze.rs` (table + tenant-scoped freeze/unfreeze), `crates/argo-agent-runner/src/turn_helpers_freeze.rs` (runtime enforcement before any provider work), `web/app/(app)/workers/[workerId]/FreezeButton.tsx` + `FreezeBanner.tsx`. **End-to-end: UI → store → runtime block.** | LOW |
| 7 | Plain-English editing | Edit by chatting in plain English | yes | **3/10** | `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:307` shows the comment "Stub: real wire-up POSTs to /api/workflows/propose". The prompt input renders, but the handler fires a `toast.loading()` and a comment says "Until the live wire (PR #1882 rebase pending) ships, surface visible feedback so the demo operator sees the action firing." **This is a UI-only demo stub.** Lindy ships this for real. | **CRITICAL** — competing against Lindy on a feature we haven't wired. |
| 8 | Plain-English editing | Visual step timeline with approval gates inline | yes | **9/10** | Same file uses `@xyflow/react` for full React-Flow visual canvas, drag-drop, pan/zoom, mini-map, approval-gate banners on nodes (`approvalGate?: { reason: string; channel: string }`). Two seed workflow templates ship with approval-gated steps. Zapier/n8n also "yes" is fair — they all have visual builders. | LOW |
| 9 | Enterprise-ready | Multi-tenant by construction | yes | **10/10** | `crates/argo-types/src/ids.rs` defines `TenantId` as core type, `CLAUDE.md` invariant #2 ("Multi-tenant by construction (tenant_id plumbed through every primitive)"), 109+ migrations all carry tenant_id, dedicated tenant-isolation tests across audit/skill/freeze. Most defensible architectural claim. | LOW |
| 10 | Enterprise-ready | Capability-based RBAC across every action | yes | **9/10** | `crates/argo-capability/src/lib.rs` "v0.2 invariant: capability-based security with first-class RBAC. Every tool dispatch checks declared capabilities. Default policies deny." Capability::ToolDispatch, CapabilityRole, dynamic grants, audit emission on capability decisions. | LOW |
| 11 | Enterprise-ready | Encryption-at-rest with operator-controlled keys | yes | **8/10** | `crates/argo-store/src/encryption.rs` (SQLCipher PRAGMA wiring), `crates/argo-store/src/encryption_key_source.rs` (KeySource enum), `crates/argo-store/src/kms/{aws,azure,gcp}.rs` (BYOK envelope encryption). Cargo features `sqlcipher` and `bundled-sqlcipher` exist. **Caveat:** the encryption module comments state "Stock builds therefore silently ignore PRAGMA key and write plaintext pages — the helper is the wire that becomes load-bearing once an operator swaps in a SQLCipher-linked SQLite driver." So this is real only if you build with the bundled-sqlcipher feature. Zapier "yes" is fair (they use AWS at-rest). | MEDIUM — only true when built with the cipher feature. |
| 12 | Deploy how you want | Single binary you can self-host | yes | **10/10** | `crates/argo-cli/Cargo.toml`: `[[bin]] name = "argo"`, single Rust workspace binary, `CLAUDE.md`: "Maintain single-binary architecture". No service split. | LOW |
| 13 | Deploy how you want | Air-gapped deploys for regulated environments | yes | **7/10** | Air-gap support is referenced in 9+ design docs and `docs/build-plan/streams/M0-enterprise-parity-track-overview.md` lists it as a track, `docs/build-plan/streams/M2-doc-airgapped-paddleocr.md` ships an air-gapped OCR. No infrastructure code blocks egress — it's "we don't require external services" rather than "we hardened against egress". n8n "partial" is generous; Crew.ai "partial" defensible (it's a Python lib). | MEDIUM — claim is "buildable air-gapped" not "shipped air-gap hardened mode". |

**Average defensibility: 7.85/10.** Weighted by row prominence (rows 5/6/9/10/12 are the structural wins, row 7 is the load-bearing risk), the page lands at roughly **8/10 defensibility** — strong overall, with one CRITICAL row that could blow up on a tough demo call.

## 2. Top 3 strongest factual wins (lead with these)

1. **Row 5 — Audit chain on every action (10/10).** Hash-chained `audit_record` table, `argo audit verify` CLI that walks the chain and reproves every row's `self_hash`, denormalized filter columns with drift detection on top. Zapier and n8n don't ship cryptographic chain verification; this is genuinely differentiated. **Lead the comparison page with this.**
2. **Row 9 — Multi-tenant by construction (10/10).** `TenantId` is a core type in `argo-types`; every store function takes tenant_id; cross-tenant negative tests exist in every subsystem (audit, skill, freeze). Crew.ai and Lindy genuinely cannot match this — Crew.ai is a Python lib with no tenancy story, Lindy is a SaaS product without per-tenant isolation guarantees.
3. **Row 12 — Single binary you can self-host (10/10).** `argo` binary, no service split. n8n also ships this (their answer is "yes" too). The combination of row 12 + row 13 (air-gap) + row 11 (BYOK) is the enterprise-deploy story that Zapier cannot touch.

Row 6 (one-click freeze) and row 3 (trust ratchet promotion) are close behind — both are end-to-end shipped surfaces with runtime enforcement.

## 3. Claims at risk of over-claiming

### CRITICAL: Row 7 — "Edit a workflow by chatting in plain English" (3/10)

**Evidence:** `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:307` — the prompt input is wired only to a `toast.loading()`. The code comment is explicit: *"Stub: real wire-up POSTs to /api/workflows/propose with the workflow + nl change. … Until the live wire (PR #1882 rebase pending) ships, surface visible feedback so the demo operator sees the action firing."* No `/api/workflows/propose` route exists in `web/app/api/workflows/`.

**Why it's risky:** Lindy genuinely ships this — natural-language assistant editing is their core differentiator. If a Diego-persona buyer (technical, knows Lindy) clicks the prompt bar and asks "can you show me a real change?", the demo collapses. The page claims **"yes" against Lindy's "yes"** on this row, which means we are publicly asserting parity on Lindy's strongest feature with a UI stub.

**Mitigation options:**
- Demote to `"partial"` until the gateway endpoint lands. Honest, defends against Lindy.
- Reframe row to "AI prompt diff suggestion" and de-emphasize the chat surface in the demo until PR #1882 ships.
- Ship the wire-up before the page lands in front of buyers (preferred).

### MEDIUM: Row 11 — "Encryption-at-rest with operator-controlled keys" (8/10)

**Evidence:** SQLCipher integration is real but **default builds ship plaintext** until the `bundled-sqlcipher` feature is enabled (`crates/argo-store/src/encryption.rs` docstring). BYOK to AWS/GCP/Azure KMS is scaffolded but not yet load-bearing in production.

**Why it's risky:** A buyer's security team will ask "show me the build that gets shipped" — if our public release artifact is the stock build, the answer is "no encryption at rest by default". The claim is true for the *enterprise* build flavor; should be footnoted or made explicit.

### LOW: Row 1 — "Named, with personality + responsibilities" (6/10)

The UI works; the backend doesn't yet store these fields. Cosmetic risk — a buyer who asks "where are these traits configured?" will find we don't have a personality-edit page yet. Not a credibility-destroyer unless the buyer is technical and pokes the data layer.

## 4. Top 3 rows to ADD to /compare

| Score | Proposed row | Group | Rationale + defensibility |
|-------|--------------|-------|---------------------------|
| **10/10** | "Cryptographically verifiable audit chain — re-verify every action months later?" | Safe by default | `argo audit verify` is unique in this space. Zapier/n8n: log-store. Lindy: SaaS audit log. Crew.ai: print statements. This is a row none of them can answer "yes" to. **STRONGEST POSSIBLE ADD.** |
| **9/10** | "Cross-app command palette — every worker, workflow, skill, page reachable from ⌘K?" | Plain-English editing | `web/components/skycot/CommandPalette.tsx` is a real shipped surface with category-grouped results, recents, quick actions. Lindy doesn't have this; Zapier has app-search but not cross-surface; n8n has nothing. Demo wow-factor. |
| **9/10** | "BYOK — your own KMS key (AWS/GCP/Azure) wraps the database?" | Enterprise-ready | `crates/argo-store/src/kms/{aws,azure,gcp}.rs` envelope-encryption pattern. Zapier: no (managed). n8n: partial (you manage your own DB). Crew.ai/Lindy: no. Enterprise deal-closer. |

Honorable mentions:
- **8/10 — "Default-deny RBAC. Every tool dispatch capability-checked."** Differentiator vs Zapier/n8n's "everyone in the workspace can run everything" model.
- **7/10 — "One-click freeze enforced at the runtime, not just the UI."** Already on the page (row 6) but worth emphasizing the runtime-enforced angle in the caption.
- **6/10 — "Hire by writing a job description in plain English (the /hire wizard)."** `web/app/(hire)/hire/page.tsx` ships. Brand-loud differentiator but harder to score competitors fairly on.
- **5/10 — "SAML + OIDC SSO via per-tenant config."** `crates/argo-config/src/sections_sso.rs` exists. Zapier and Lindy both have SSO on enterprise tiers; rare clean win.

## 5. Competitor moves to consider (no web-search performed)

- **n8n** has been pushing hard on AI nodes and an "AI Agent" node since 2024. Their *visual* approval gate story remains weaker than ours, but expect them to claim "AI-native" by mid-2026 — our "AI teammates" framing needs to keep distance.
- **Zapier** shipped "Zapier Central" (their assistant) — increases the risk that our row 7 "plain-English editing" claim against Zapier looks dated. We mark Zapier "no" for natural-language editing; if their Central is widely known, this could be wrong.
- **Crew.ai** is positioning as the dev framework, not a competitor in our enterprise lane — our "code-first, no operator surface" positioning blurb is still accurate.
- **Lindy** is the closest competitor on the UX axis and the biggest threat on row 7. Don't claim parity on natural-language editing until #1882 ships.

## 6. Verdict — would /compare survive a 30-min Diego-persona sales call?

**Yes, with two visible-risk caveats. ~B+ to A- depending on which rows get probed.**

**Survives because:**
- The audit chain, multi-tenancy, freeze button, capability RBAC, trust ratchet, and single-binary claims are all genuinely shipped end-to-end with code I can show on the call.
- 5 of 13 rows are 9/10 or 10/10 defensible — that's enough material to win a deal on enterprise-deploy + safety alone.
- The framing ("teammates not workflows") is well-positioned against Crew.ai (dev framework) and Zapier (workflow automation) — Diego will recognize the gap those competitors have.

**Survives only if Diego doesn't:**
- Click the "Add a step that emails the seller before posting" prompt in the workflow builder and expect a real proposed change (row 7 is a stub).
- Ask which release artifact ships with encryption-at-rest enabled by default (row 11 is build-flavor-dependent).
- Probe the personality cards on the worker page to find where the traits are configured (row 1 is UI-only).

**Recommendations before the next big sales demo:**
1. Demote row 7 to "partial" until the gateway endpoint lands, OR rush PR #1882 in.
2. Add a footnote on row 11 clarifying SQLCipher is the enterprise build flavor.
3. Add the 3 proposed new rows above — the cryptographic-audit-verify row alone is worth more than any of the soft rows.
4. Lead the demo with rows 5 (audit chain) + 9 (multi-tenancy) + 6 (freeze) + 12 (self-host) — the 4-row "enterprise-grade by construction" pitch is bulletproof.

---

**Read-only audit. No code changes made.**
