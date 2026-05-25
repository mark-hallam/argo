# mission-genesis-integration-import-slack-notion-confluence

**Owner**: claude or codex.
**Authored against**: origin/main `a85963b34` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-integration-import-slack-notion-confluence`.
**Depends on**: `mission-genesis-document-upload-and-memory-ingest` MERGED (re-uses the chunking + memory-write pipeline).

🔴 **P0 MISSION CLAIM 3 — Phase 2 Onboarding Studio — external integration import.**

Per memory [[genesis-is-ai-teammate-onboarding-studio]], onboarding a teammate includes giving them access to company wiki + Slack history so they pick up tone, context, and "who asks what". This brief leverages the existing MCP connector layer (29 connectors per `eval-mcp-connector-real-vs-stub-inventory`) — Slack, Notion, Confluence, Google Drive, Microsoft 365 — to import content as memory at hire time.

The investor moment: *"I connected our Slack workspace and Confluence — the worker now knows our team's slang, our product names, our escalation patterns from past tickets."*

## Scope

1. **SPA surface**: on `/genesis`, add a "Connect your tools" section below the document drop-zone. Cards for Slack / Notion / Confluence / Google Drive / Microsoft 365 / Linear / GitHub Wiki (subset matching real-vs-stub-MERGED connectors).
2. **OAuth flow**: clicking a card opens the existing MCP connector OAuth (Open Authorization — a standard delegation flow where the user authorizes Skycot to read their workspace without sharing the password). Tokens go to the tenant-scoped credential store per `mission-mcp-connector-token-cache-tenant-keyed` (#2103 MERGED).
3. **Source-scope selector**: after OAuth, operator picks WHICH channels / WHICH wiki spaces / WHICH folders to import — not the whole workspace by default. Defaults to "the spaces I'm an admin of" + a "preview-first" affordance.
4. **Import pipeline**: selected content streams through the same chunking + embedding pipeline as `mission-genesis-document-upload-and-memory-ingest`, into the worker's `onboarding_integrations` memory namespace (or extend the existing `onboarding_documents` namespace with a `source: { kind: "slack" | "notion" | ... , id, url }` discriminator).
5. **Per-source progress**: same 4-phase progress bar as document upload (connected / fetching / chunking / embedded). SSE-streamed through the parent Genesis progress strip.
6. **Cross-tenant**: integration tokens are tenant-scoped (already enforced by `mission-mcp-connector-token-cache-tenant-keyed`); imported content is tenant-scoped at the memory layer.

## Acceptance criteria

- [ ] At least 3 integrations live: Slack (channel history), Notion (page tree), Google Drive (folder).
- [ ] OAuth flow completes for each + token persists tenant-scoped.
- [ ] Import streams real content + the new worker's first-turn can cite specifics ("you said in #engineering on March 3rd...").
- [ ] Cross-tenant: tenant A's Slack import is NOT visible in tenant B's `audit/memory/list`.
- [ ] Failure modes: revoked OAuth → clear message; rate-limited upstream → backoff with operator-visible progress; partial-import-completed → operator can resume or hire-as-is.
- [ ] Function-side: imported content the camera SEES must be REAL upstream API responses; sandbox account in dev mode is fine but mock fallback is NOT acceptable for the demo.

## May-touch

- `web/components/skycot/genesis/IntegrationConnector.tsx` (NEW — per-integration card + OAuth trigger)
- `web/lib/api/genesis-integrations.ts` (NEW — list available connectors + initiate OAuth + select-source UI + status polling)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_integration.rs` (NEW — OAuth callback handler + source-list endpoint + ingest dispatch)
- `crates/argo-genesis/src/integration_import.rs` (NEW — per-source fetchers calling argo-mcp connectors + chunking)
- Re-use `crates/argo-genesis/src/onboarding_documents.rs` chunking + memory-write pipeline (from prior brief)

## Out of scope

- Continuous sync post-Hire (one-time import for v1.0; sync is a v1.1 brief)
- Per-document-level fine-grained permissions inside an imported workspace (rely on what OAuth grants)
- Custom-connector framework (use existing MCP connectors only)
- Email import (Gmail / Outlook) — flagged but separate brief if needed

## Effort

~500-900 LOC (3 integrations × ~150 LOC each + shared OAuth/UI + memory pipeline). ~6-10 hours. Splittable per-integration if needed.
