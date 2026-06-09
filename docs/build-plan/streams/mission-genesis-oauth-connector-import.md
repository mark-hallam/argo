# mission-genesis-oauth-connector-import

**Owner**: claude.
**Authored against**: origin/main `8618c7ec7` on `2026-06-10`.
**Status**: BLOCKED on `web-settings-connectors-page-build` (audit F-01).
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-oauth-connector-import`.

## Why (deferred slice of mission-genesis-integration-import-slack-notion-confluence)
The shipped slice (#2769) brands the existing SSRF-guarded `wiki_url` ingest so a
stranger can paste a **published / public** wiki / Notion / Confluence page URL on
`/genesis`. The remaining gap is **true OAuth import of PRIVATE workspaces**: Slack
channel history, private Notion via integration token, Confluence Cloud via API
token, Google Drive folders, MS 365. This is DEFERRED, not dropped — the shipped UI
already tells the operator "private workspaces need a workspace connector (coming
soon)", and this brief is the honest follow-up that makes that true.

## Blocked-on (do NOT claim until satisfied)
- **Audit F-01 / F-02:** there is NO `/settings/connectors` page and NO OAuth
  connect-account UI. The 23 real MCP connector crates (`argo-mcp-connector-slack`,
  `argo-mcp-connector-notion`, …) have **no operator-facing surface** — connecting a
  workspace today means editing env vars and restarting the gateway. This brief
  depends on `web-settings-connectors-page-build` landing that OAuth connect-account
  surface first.
- **Confluence has no connector crate at all** — a new `argo-mcp-connector-confluence`
  (pattern: `crates/argo-mcp-connector-notion/src/*.rs`) is a prerequisite for the
  Confluence path.
- Genesis/gateway do NOT yet consume `argo-mcp-connector-slack` /
  `argo-mcp-connector-notion` (`rg -l argo-mcp-connector-slack crates/argo-genesis
  crates/argo-cli` → no consumer). A genesis-side ingest bridge from a connected
  workspace into the tenant-scoped onboarding namespace must be built here.

## Fix (when unblocked)
1. After a workspace is connected via the F-01 connectors surface, let the operator
   pick WHICH channels / spaces / folders to import on `/genesis`.
2. Pull the selected content through the connector crate (`reqwest` clients already
   exist for Slack/Notion) and ingest it into the SAME tenant-scoped onboarding
   namespace the `wiki_url` / document path uses — re-use `OnboardingDocument` +
   the chunk/embed pipeline; NO parallel ingest route.
3. First-turn citation: the hired teammate can cite specifics ("you said in
   #engineering on March 3rd…"), gradeable by the existing `/api/v1/genesis/quiz`.

## Effort
M-L — ~500-900 LOC (connectors-settings dependency + per-connector import bridge +
Confluence connector crate). Multi-crate; depends on F-01.
