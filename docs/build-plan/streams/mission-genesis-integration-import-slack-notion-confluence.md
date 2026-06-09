# mission-genesis-integration-import-slack-notion-confluence

**Owner**: claude.
**Authored against**: origin/main `5b20282f7` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-integration-import-slack-notion-confluence`.

## Why (verified gap + verified scope-down, 2026-06-09)
DEC-034 WS3 Genesis-as-onboarding-studio: a stranger should be able to import context from their company wiki / Slack / Notion / Confluence so the new teammate boots already knowing the team's slang, product names, and escalation patterns. The original (2026-05-25) brief assumed a full per-connector **OAuth import** flow. That is NOT a small real slice today — see the scope-down below.

VERIFIED on current `origin/main`:
- **What's already shipped (re-use it):** `POST /api/v1/genesis/ingest-document` is live (`crates/argo-cli/src/handlers/gateway/routes_genesis_ingest.rs`, shipped #2728 / `644be2cb1`). It accepts a `"wiki_url"` / `"url"` source kind that fetches a page via the **SSRF-guarded** `argo_genesis::safe_fetch_text` (`routes_genesis_ingest_url.rs::fetch_wiki`) and ingests the page text into the worker's onboarding memory. The SPA already exposes a single generic URL-paste input — `web/components/skycot/genesis/DocumentUploader.tsx` line 414 placeholder `"…or paste a wiki / docs URL"` (`data-testid="genesis-url-input"`), sent as `format: "wiki_url"` (`web/lib/api/genesis-ingest.ts` line 125).
- **The remaining gap:** there is NO branded "import from Notion / Confluence" affordance — just one anonymous URL box with no guidance, and no help text telling a user they can paste a published Notion/Confluence/wiki page URL. A stranger has no signal that "import my wiki" is even possible.
- **Why the full OAuth path is DEFERRED (verified):** `argo-mcp-connector-slack` and `argo-mcp-connector-notion` ARE real `reqwest` HTTP clients (`slack.com/api`, `api.notion.com/v1`) per `eval-mcp-connector-real-vs-stub-inventory` — but they are **NOT consumed by genesis or the gateway** (`rg -l argo-mcp-connector-slack crates/argo-genesis crates/.../gateway` → no consumer), and there is **NO `/settings/connectors` page and NO OAuth connect-account UI** (audit finding F-01/F-02: "23 REAL connectors with no operator-facing surface … the answer to 'can the operator connect a Slack workspace?' is edit env vars and restart the gateway"). So Slack OAuth import cannot be the demo path until that surface lands. Confluence has no connector crate at all.

**Scope this brief to the smallest REAL slice: branded URL-paste import for wiki / Notion / Confluence published pages, on the EXISTING `wiki_url` ingest path. Slack-history + true OAuth import are DEFERRED (named below) to a follow-up that depends on the connectors-settings surface (F-01).**

## Grep-before-write
- `rg -n 'genesis-url-input|wiki_url|paste a wiki' web/components/skycot/genesis/DocumentUploader.tsx web/lib/api/genesis-ingest.ts` — the existing URL-paste + `wiki_url` client wiring to extend (do NOT add a parallel ingest path).
- `rg -n '"wiki_url"|"url"|fetch_wiki' crates/argo-cli/src/handlers/gateway/routes_genesis_ingest.rs crates/argo-cli/src/handlers/gateway/routes_genesis_ingest_url.rs` — the live server-side handler; this brief adds NO new endpoint, it reuses `wiki_url`.
- `rg -l 'IntegrationConnector|WikiImportCard|genesis-integrations' web/` — confirm the proposed names are free (verified free 2026-06-09; the old brief's `routes_genesis_integration.rs` + `integration_import.rs` do NOT exist and are NOT created here).
- `rg 'pub (struct|enum|fn) (fetch_wiki|safe_fetch_text|ingest_response)' crates/` — these already exist; this brief REUSES them and declares NO new Rust public surface.

## Fix (additive; consumer = the `/genesis` page via `DocumentUploader`)
1. **Brand the existing URL-paste** in `web/components/skycot/genesis/DocumentUploader.tsx`: above (or beside) the existing `genesis-url-input`, render a small row of source chips/labels — "Notion", "Confluence", "Wiki / Docs" — each a presentational hint that, when clicked, focuses the URL input and sets a source-specific placeholder + helper line (e.g. Notion → "Paste a published / shared Notion page URL"; Confluence → "Paste a Confluence page URL your account can view"). Keep it ONE input under the hood; the chips only adjust guidance copy. Extract the chip row into a NEW component `web/components/skycot/genesis/WikiImportSources.tsx` if `DocumentUploader.tsx` would cross 500 LOC.
2. **No new endpoint, no new Rust.** Every chip's "Add link" still POSTs `format: "wiki_url"` to the existing `/api/v1/genesis/ingest-document` — the SSRF-guarded `fetch_wiki` already handles arbitrary published page URLs. (Notion/Confluence published pages are plain fetchable HTML; that is the real, demoable slice.)
3. **Per-source helper + a one-line "private workspaces need a connector (coming soon)" note** so the UI is honest about the boundary — a stranger pasting a private Slack/Notion URL gets a clear "this needs the workspace connector, not yet available" message rather than a silent failure.
4. **DEFERRED (name it in the brief + a follow-up):** true OAuth import (Slack channel history, private Notion via integration token, Drive folders) is OUT — it depends on `web-settings-connectors-page-build` (audit F-01) landing an OAuth connect-account surface first. File the follow-up `mission-genesis-oauth-connector-import` as BLOCKED-on-F-01; do NOT build it here.

## Acceptance (truth-bounded)
1. `/genesis` shows source-labelled wiki-import chips (`data-testid="genesis-import-source-notion"` / `-confluence` / `-wiki`) that drive the existing `genesis-url-input` with source-specific placeholder/helper copy; the underlying POST is still `format:"wiki_url"` to the live endpoint (NO mock, NO new endpoint).
2. **vitest:** clicking a source chip updates the input placeholder/helper and focuses the input; "Add link" with a URL calls the existing `ingestUrl` path (mock the fetch) — assert the request body carries `format:"wiki_url"`. The pasted-content the camera would see is a REAL upstream fetch (the server path is unchanged) — no canned fallback.
3. Web test runner green for touched files; web lint clean. Touched files stay < 500 LOC (extract `WikiImportSources.tsx` if needed).
4. The "private workspaces need a connector (coming soon)" note is present so the deferred-OAuth boundary is honest (NO_THEATER, DEC-007).

## On ship (same commit)
Update the FEATURE_MATRIX.md Genesis multi-modal / onboarding-studio row to note Genesis intake now offers branded wiki/Notion/Confluence URL-import via the existing SSRF-guarded `wiki_url` ingest, and that OAuth/Slack-history import is DEFERRED on the connectors-settings surface (F-01). 🚨 DEC-043: do NOT regenerate or commit `feature-matrix.html` (gitignored) — `.md` row only.

## Effort
S-M — web-only, ~120-220 LOC (chip row component + placeholder/helper wiring in `DocumentUploader` + vitest). No new Rust, no new endpoint. ~2-3h. The original 500-900 LOC OAuth estimate is for the DEFERRED follow-up, not this slice.
