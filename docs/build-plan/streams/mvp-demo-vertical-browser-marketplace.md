# MVP demo prep — marketplace-seller vertical with BrowserExecutor

**Owner**: claude. **Depends on**: BrowserExecutor MERGED (#1247 — `executor.kind = "browser"`); existing `verticals/marketplace-seller/vertical.toml` SHIPPED (verified at `verticals/marketplace-seller/`).
**Authored against**: origin/main `5e1ef545` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mvp-demo-vertical-browser-marketplace`.

> SOURCE: gap surfaced by MVP-readiness audit (4 investigation agents, 2026-05-11). The v1 demo requires "at least one action executed via BrowserExecutor" but **no shipped vertical declares `executor.kind = "browser"`**. The demo script today still references HumanExecutor for the post-listing step. This brief authors a demo-targeted vertical that uses BrowserExecutor for the post-listing action, so the operator can record the v1 demo with a real autonomous browser execution segment.

## Scope

ADD a NEW vertical at `verticals/marketplace-seller-browser/` based on the existing `verticals/marketplace-seller/` template. Only structural difference: the `post_listing` action declares `executor = { kind = "browser", binding = "..." }` per the pattern at `crates/argo-cli/tests/browser_action_executor_smoke.rs:62`.

**Files in the new vertical**:
1. `verticals/marketplace-seller-browser/vertical.toml` — full TOML manifest:
   - Identity: id `marketplace-seller-browser`, name "Marketplace Seller (Browser)"
   - Inherits prompts/persona/memory from `marketplace-seller` (symlinks or copies — implementer chooses based on existing convention)
   - `[actions.post_listing]` with `executor = { kind = "browser", binding = '''{"target_url": "https://demo.skycot.dev/marketplace-mock", "steps": [...]}''' }`
   - Other actions keep their existing kinds
2. `verticals/marketplace-seller-browser/eval/` — copy from sibling
3. `verticals/marketplace-seller-browser/prompts/` — symlink or copy from sibling
4. `verticals/marketplace-seller-browser/skills/` — symlink or copy from sibling
5. `verticals/marketplace-seller-browser/inventory.json` — copy

The browser `binding` JSON contains a simple multi-step instruction (navigate → fill form → click submit) targeting a demo-safe URL. The brief is intentionally vague on the URL: the implementer either uses a localhost mock served by `browser_mock_mcp.py` or hardcodes a permissively-licensed demo target (e.g. example.com), at their discretion documented in the PR.

The vertical is **registered** by being present in the `verticals/` tree — no Rust code change needed. `argo vertical list` will pick it up automatically.

This is NOT in scope: changing the BrowserExecutor itself; rewriting the existing `marketplace-seller` vertical; authoring a Slack-app setup guide (separate brief); recording the actual demo (operator action).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `verticals/marketplace-seller-browser/` (NEW directory) — all files above. Reuse symlinks pointing at the sibling directory where possible to minimize duplication.

## Files you must NOT touch

- `verticals/marketplace-seller/` — sibling stays untouched
- `verticals/AUTHORING.md` / `AUTHORING_v3_ACTIONS.md` — docs upstream
- Any Rust source — manifests only
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `verticals/marketplace-seller-browser/vertical.toml` exists and parses (`argo vertical lint --vertical marketplace-seller-browser` clean)
- [ ] `argo vertical list` includes the new vertical id
- [ ] `argo vertical show marketplace-seller-browser` displays the manifest cleanly
- [ ] The `post_listing` action declares `executor.kind = "browser"`
- [ ] Other actions are byte-identical (or symlinks-equivalent) to the sibling marketplace-seller
- [ ] `argo work add --vertical marketplace-seller-browser --prompt "..."` succeeds
- [ ] `argo work serve --vertical marketplace-seller-browser --once --max-steps 5 --json` succeeds in mock mode
- [ ] No new Rust code; no migration; no test changes
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mvp-demo-vertical-browser-marketplace`
- PR title: `verticals: marketplace-seller-browser for v1 demo BrowserExecutor segment (mvp-demo-vertical-browser-marketplace)`

## Notes for the implementer

- The `binding` JSON content matters less than the fact that it parses. The point of this brief is "operator can pick a vertical that uses BrowserExecutor at all". Pick a plausible 3-step interaction (`navigate`, `wait_for_selector`, `click`) that the existing browser MCP fixture can handle.
- The TOML format quirk: `binding = '''{...}'''` (triple-single-quote) is the convention for embedded JSON-in-TOML — copy from `tests/browser_action_executor_smoke.rs:62` exactly.
- Symlinks vs copies: check what `verticals/recruiter/` does (it's listed but the contents are likely sparse) — match that style. If unclear, use full copies for safety.
- Don't break the existing `marketplace-seller`. Many smoke tests probably reference it.
