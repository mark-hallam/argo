# cu1h-dec050-adversarial-review-followups

**Owner**: claude or codex — security-hardening + perf; conductor review before merge.
**Depends on**: cu1d #2819, cu1f #2820, cu1g #2823 merged (this collects their non-blocking review findings).
**Authored against**: origin/main `ee5e9e79e` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh cu1h-dec050-adversarial-review-followups

## Source

A 9-skeptic adversarial review of the cu1d/cu1f/cu1g PRs (2026-06-11, Fable)
cleared them for merge (cross-tenant + exfiltration + correctness attacks all
FAILED) and surfaced one MEDIUM (fixed in cu1g `d6fef839b`) plus these
non-blocking findings, captured here so they are not lost.

## Items (severity ordered)

### 1. LOW — `workspace_for_tenant` cwd fallback is fail-open
`crates/argo-cli/src/handlers/worker_actions_query.rs` `workspace_for_tenant`:
an authenticated tenant with no provisioned workspace row falls through to the
demo/cwd workspace and could read/decide that workspace's flat held actions.
Mitigated today (provisioning always writes a workspace row —
`tenant_provision.rs:145-150`), so it is theoretical, but contrary to
fail-closed doctrine. **Fix**: distinguish a session-authenticated tenant
(return empty/None — its own empty workspace) from the genuinely anonymous /
demo path (cwd is legitimate). Likely needs the request's auth state threaded
in, or a `first_workspace_for_tenant`-or-empty (no cwd ensure) variant for the
authenticated branch.

### 2. LOW — decide/undo open a separate store handle
`routes_approvals_json::{decide_response,undo_response}` call
`open_workspace_memory_with_store()` (a fresh `SessionStore::open` keyed only by
`SKYCOT_DB_KEY_BASE64`) instead of the gateway's shared, already-open store. On
keyring-only deployments (no env key) a listed approval could 500 on decide.
**Fix**: thread the gateway's shared `Arc<SessionStore>` into decide/undo (the
dispatcher already has it for the list path).

### 3. LOW/perf (durable fix for the cu1g MEDIUM) — index `action_id`
`find_action` / `collect_latest_actions*` still scan per-vertical because
`action_id` lives in the JSON body, not a column. cu1g bounded the vertical set
(workspace `distinct_verticals`), but the durable fix is to persist `action_id`
as a `memory_entry` column (or a side `action_id → (workspace, vertical_id)`
index) so locate is a single O(1) indexed query. Additive migration; back-fill
on read. Also: have the 5 state transitions accept the already-located
`vertical_id`/`&LocatedAction` from decide so the hot path never re-locates.

### 4. ADVISORY — `web_extract` SSRF reach (from cu1d NetEgress)
`crates/argo-tool/src/tools/web/mod.rs` fetcher: `NetEgress{**}` lets a
model-controlled URL reach internal / link-local / cloud-metadata hosts
(169.254.169.254, etc.). **Fix**: a private-IP / metadata-endpoint blocklist in
the web_extract + web_search HTTP client (resolve + reject RFC1918/loopback/
link-local before connect). Independent of the capability grant.

### 5. COSMETIC — cu1f swallowed error detail
`crates/argo-vertical/src/loader.rs:155` `if let Ok((registry, _errors))`
drops the `load_dir` error the old path logged. Log `{e}` on the Err arm.

### 6. INFO — redundant tenant resolution in decide
`decide_response` resolves the tenant at the top AND again in
`finalize_decision`. Dedupe opportunistically.

## Acceptance / verifier

Per item: unit test for #1 (authenticated-no-workspace → no cross-read) + #4
(metadata IP rejected). #3 carries an additive-migration test + a
locate-is-one-query assertion. `cargo test -p argo-cli -p argo-memory -p
argo-tool` green; clippy.

## Size / priority

~200-350 LOC across items (splittable). **🟡 P2** — none block the DEC-050
demo; #4 (SSRF) is the highest-value security item, #3 the highest-value perf
item. Safe to land piecemeal.
