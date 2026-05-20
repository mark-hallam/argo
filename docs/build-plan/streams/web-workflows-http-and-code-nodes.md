# web-workflows-http-and-code-nodes — generic HTTP Request + Code (JS/Python) nodes

**Owner**: claude. **Sizing**: ~200-350 LOC.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflows-http-and-code-nodes`.
**Priority**: **P0** — n8n + UIpath both ship these as the universal escape hatch. Without them every integration we don't have a first-class skill for is a dead end. Power-user credibility hinges on this.

## Scope

Add two new step kinds to the visual builder at `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (plus `SkycotStepNode` + `stepsToFlow()` helpers and the step-encoder).

### Part A — HTTP Request node

1. New step kind `http_request` with config:
   - `method` — dropdown (GET / POST / PUT / PATCH / DELETE / HEAD / OPTIONS), default GET
   - `url` — text input (supports `{{var}}` template placeholders from upstream payload)
   - `headers` — key/value editor (multi-row, add/remove rows)
   - `body` — textarea, monospaced, JSON-aware syntax highlighting if cheap (otherwise plain monospace)
   - `auth` — selector: None / Bearer / Basic / Custom Header. Each shows the right secondary input(s).
   - `timeout_seconds` — number, default 30
2. Node renders with:
   - Header shows method + truncated URL ("GET api.stripe.com/v1/customers/...")
   - Single input + single output handle
   - Hover tooltip shows the full URL + auth mode
3. Inline test-call button on the node ("Try it") — POSTs the config to gateway `/api/v1/tools/http/try` (gateway brief if endpoint doesn't exist; otherwise show a "test endpoint not yet wired" tooltip) and shows status + truncated response body in a popover.

### Part B — Code (JS / Python) node

1. New step kind `code` with config:
   - `language` — selector: JavaScript (Node 20) / Python 3.12 (stub today; gateway executor brief separate)
   - `code` — monaco-editor textarea (or fall back to plain `<textarea class="font-mono">` if Monaco bundle cost is too high — pick the cheaper path that still feels editor-grade)
   - Read-only display of available variables ("Available: input, env, secrets")
2. Node renders with:
   - Header shows language icon + truncated first line of code
   - Single input + single output handle
   - **Sandbox warning banner** displayed inside the node config drawer in amber: "Code runs in an isolated sandbox. No filesystem access. 5s CPU budget. Network requires explicit allowlist."
3. Validation: empty `code` blocks save submission with an inline error.

### Shared

- Both nodes appear in the Add-step palette under a new "Tools" group (or extend the existing one if present).
- TypeScript types: extend `WorkflowStepKind` union — grep `web/lib/types` first to find canonical home.
- TOML encoder writes `kind: "http_request"` / `kind: "code"` with the relevant config keys.

## Acceptance criteria

- [ ] HTTP node has method (dropdown) / URL / headers (k/v editor) / body / auth / timeout config.
- [ ] HTTP node header summary shows method + truncated URL.
- [ ] HTTP node "Try it" button either calls the test endpoint or shows a friendly "test endpoint coming v1.x" tooltip.
- [ ] Code node has language selector + textarea + sandbox warning banner.
- [ ] Code node header summary shows language + first line.
- [ ] Both kinds appear in the Add-step palette.
- [ ] Both kinds round-trip through the TOML encoder without losing config.
- [ ] Existing workflows still render correctly.
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- The actual sandboxed Code executor (gateway concern — separate brief).
- HTTP secret-store integration (use plaintext field for v1; secret-ref UI is v1.x).
- Syntax-aware JSON body validation beyond "is it valid JSON on save".
