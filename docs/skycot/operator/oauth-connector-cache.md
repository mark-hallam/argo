# OAuth connector token cache — tenant-keyed migration

**Applies to:** `argo-mcp-connector-gcal`, `argo-mcp-connector-gmail`,
`argo-mcp-connector-salesforce` (released
`mission-mcp-connector-token-cache-tenant-keyed`).

## Why this changed

Prior to this release the three OAuth-bearing MCP connectors (Google
Calendar, Gmail, Salesforce) wrote their cached access tokens to a
**single, cwd-shared path** regardless of which tenant invoked them:

```
<cwd>/.argo/connectors/gcal/access_token.json
<cwd>/.argo/connectors/gmail/access_token.json
<cwd>/.argo/connectors/salesforce/token.json
```

In a multi-tenant gateway running with one working directory, tenant A's
token was readable by — and could be overwritten by — tenant B. This is
a P0 cross-tenant credential leak.

The cache path is now tenant-keyed:

```
<cwd>/.argo/connectors/<tenant_id>/gcal/access_token.json
<cwd>/.argo/connectors/<tenant_id>/gmail/access_token.json
<cwd>/.argo/connectors/<tenant_id>/salesforce/token.json
```

`<tenant_id>` is resolved per-subprocess at startup:

1. The `ARGO_MCP_TENANT_ID` env var set by the spawner
   (`McpConnectionPool` in `argo-agent-runner`) when launching the
   connector under a specific tenant context.
2. Falls back to the workspace-stable id produced by
   `argo_config::workspace_tenant_id(&cwd)` — i.e. either the explicit
   `[tenant] id = "..."` in `argo.toml`, or the `ws-<hex>` derived from
   the workspace path. Single-tenant deployments therefore still get a
   stable, isolated cache rather than reusing the legacy shared path.

## What happens on first read after upgrade

When any of the three connectors starts up after the upgrade and notices
a pre-existing legacy file at the shared path, it moves the file out of
the way before reading the (now-empty) tenant-keyed path:

```
<cwd>/.argo/connectors/gcal/access_token.json
  → <cwd>/.argo/connectors/legacy/gcal/access_token.json
```

A single `tracing::warn!` is emitted per (tenant, connector) noting the
move. The connector then refreshes a brand-new access token via its
normal OAuth refresh flow and writes it to the tenant-keyed path.

**Operator-visible effect:** the very next tool call after upgrade
triggers one extra OAuth refresh round-trip for each affected
connector. No outage; no manual re-authentication is required as long
as `ARGO_<CONNECTOR>_REFRESH_TOKEN` and the matching client id/secret
are still present in workspace credentials.

## Clearing the `legacy/` directory

Once you have confirmed the upgrade is sticking (every tenant has its
own working `<tenant_id>/<connector>/` cache file), the quarantined
`legacy/` tree is safe to delete:

```bash
rm -rf "$ARGO_WORKSPACE/.argo/connectors/legacy"
```

The migration is one-shot — the connectors do not re-create the legacy
path. Deleting it does not affect any active tenant.

If you discover after the fact that the legacy file held a token you
actually needed (e.g. to recover an in-flight OAuth state), you can copy
it back into the appropriate `<tenant_id>/` directory before the
quarantine sweep — the connector will pick it up on the next read.

## Source

- Brief: `docs/build-plan/streams/mission-mcp-connector-token-cache-tenant-keyed.md`
- Audit: PR #2097 evaluator round-10 (29-row connector matrix; 3 × P0
  cross-tenant leaks)
- Same root cause as #2080 §5 (round-9 credentials audit), but at the
  per-connector cache layer instead of the global `argo_config::cred`
  path.
