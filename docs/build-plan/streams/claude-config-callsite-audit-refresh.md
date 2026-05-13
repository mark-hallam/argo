# claude-config-callsite-audit-refresh — refresh `CONFIG_CALLSITE_AUDIT.md` to current main state

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `adeb3faf` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-config-callsite-audit-refresh`.

> SPLIT 1 of 2 from the DEEP-DRIFT `claude-config-loader-migrate-callsites` (audit stale + scaffold extension forbidden but required). This sub-brief refreshes the audit document so the migration sub-brief has a current source-of-truth.

## Scope

`docs/skycot/CONFIG_CALLSITE_AUDIT.md` snapshot is stale: 21 files / 25 env-var keys vs the current `scripts/check-no-direct-env-reads.sh` output of 31 files / 39 keys. Refresh the audit by re-running the script and annotating each new row with a recommended `ArgoConfig` field mapping.

**New keys not in the original audit** (per the DEEP-DRIFT note):
- `ARGO_CSP_REPORT_ONLY`
- `ARGO_CU_SCREENSHOT_RETENTION_DAYS`
- `ARGO_DELIVERY_REPORTS_PATH`
- `ARGO_IP_RATE_{CAPACITY,LIMIT,REFILL}`
- `ARGO_MCP_HEALTH_CHECK`
- `ARGO_MCP_REGISTRY_BACKED`
- `ARGO_PII_{AUTOREDACT,DETECT,LOG_FINDINGS}`
- `ARGO_RATE_LIMIT_TENANT_PER_MIN`
- `ARGO_SHUTDOWN_DEADLINE_SECS`
- `ARGO_TRUST_XFF`

For each new key, recommend a target `sections_<topic>.rs` sibling field. For each existing row, re-verify the line number still resolves (line numbers drift with file edits).

**Not in scope**: actually migrating callsites (that's the next sub-brief `claude-config-loader-migrate-callsites-v2`); extending the scaffold (also that sub-brief); flipping CI.

## Files you may touch

- `docs/skycot/CONFIG_CALLSITE_AUDIT.md` — rewrite the table to reflect current main state. ≤300 LOC depending on layout.

## Files you must NOT touch

- Any `crates/**` — pure docs refresh.
- `scripts/check-no-direct-env-reads.sh` — running it is read-only.
- `.github/workflows/ci.yml`.

## Acceptance criteria

- [ ] `CONFIG_CALLSITE_AUDIT.md` table count matches `bash scripts/check-no-direct-env-reads.sh | wc -l` output ±0.
- [ ] Every row has a recommended `ArgoConfig` field (mostly `sections_<topic>::<field>` paths). Where the existing scaffold doesn't have an obvious home, note `NEW: <topic>_section` and pick the file (e.g. `sections_rate_limit.rs` for the `ARGO_IP_RATE_*` cluster).
- [ ] Document the recommendation that **~30 of 39 env vars need new ArgoConfig fields** — the audit's recommendation drives the next sub-brief's scope.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-config-callsite-audit-refresh`
- PR title: `docs: refresh CONFIG_CALLSITE_AUDIT.md to current main state (claude-config-callsite-audit-refresh)`

## Notes for the implementer

- Pure docs. ~30 min worker time.
- Group the new env vars into 3-5 cohesive clusters (rate-limit, PII, MCP, screenshot/CU, lifecycle) and recommend a `sections_<cluster>.rs` per cluster rather than 1 sibling per env var.
- This refresh is the input for `claude-config-loader-migrate-callsites-v2` — keep the recommendation column clean.
