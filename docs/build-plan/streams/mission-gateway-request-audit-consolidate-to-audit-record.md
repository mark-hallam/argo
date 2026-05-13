# mission-gateway-request-audit-consolidate-to-audit-record — v1.x port: merge gateway_request_audit cols into audit_record.payload_json

**Owner**: claude. **Depends on**: #1564 MERGED (gateway_request_audit table shipped); audit-3 INTENT-DRIFT finding documented.
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-request-audit-consolidate-to-audit-record`.

> **v1.x CLEANUP 2026-05-14 by conductor (CEO-authorized follow-up to #1564 merge decision).** #1564 shipped a parallel `gateway_request_audit` table alongside the audit-chain's existing `kind='http.request'` rows. The 3 columns genuinely missing from audit_record were the merge justification: `source_ip`, `user_agent`, `body_sha256`. This brief ports those 3 cols onto `audit_record.payload_json` JSON keys + retires the parallel table.

## Scope

1. **Extend** `routes_request_audit.rs:73-100` payload_json with `source_ip` / `user_agent_truncated` / `body_sha256_hex` keys. The fields are already captured at request entry; just write them into payload_json on the audit-chain row. ≤30 LOC.

2. **Migrate `argo gateway audit list/show` CLI** to query `audit_record WHERE kind='http.request'` + extract the 3 new keys from `payload_json`. ≤80 LOC.

3. **Migration**: NEW migration that copies all extant `gateway_request_audit` rows into `audit_record` (or accepts they stay as legacy + new rows go to audit_record). Document the choice in PR body.

4. **Deprecate-and-delete** `gateway_request_audit` table: mark in v1.x with `// DEPRECATED: use audit_record kind='http.request'` comment + plan deletion in v1.2.

**Not in scope**: changing the audit-chain hash semantics; removing the gateway-side capture at boot.rs (just rewires the destination).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_request_audit.rs` — narrowly: payload_json extension. ≤30 LOC.
- `crates/argo-cli/src/handlers/gateway/gateway_audit_list.rs` (or wherever the CLI handler lives) — migrate query. ≤80 LOC.
- `crates/argo-store/src/migrations/<slot>__copy_gateway_request_audit_to_audit_record.sql` — one-shot copy.
- `crates/argo-store/src/migrations/registry.rs` — register.

## Files you must NOT touch

- `audit_record` schema (kind enum, payload validation) — locked.
- The audit-chain hash impl — locked.

## Acceptance criteria
- [ ] `audit_record kind='http.request'` rows carry `source_ip`/`user_agent_truncated`/`body_sha256_hex` in payload_json.
- [ ] `argo gateway audit list/show` returns identical output (operator-visible no-change).
- [ ] Legacy `gateway_request_audit` rows copied; new rows go to audit_record only.
- [ ] DEPRECATED comment on the legacy table.
- [ ] Workspace clippy clean. BUILD_STATUS row updated.

## Branch + PR

- Branch: `claude-mission-gateway-request-audit-consolidate-to-audit-record`
- PR title: `audit: consolidate gateway_request_audit into audit_record kind=http.request (mission-gateway-request-audit-consolidate-to-audit-record)`
