# SKYCOT Operator Runbook -- SCIM Provisioning

The gateway speaks SCIM 2.0 under `/scim/v2/Users` for automatic user
provisioning from Okta, Azure AD, Google Workspace bridges, JumpCloud,
and other enterprise IdPs that follow RFC 7644.

## Bootstrap

1. Generate a scoped bearer token:

   ```sh
   export ARGO_SCIM_BEARER_TOKEN=$(openssl rand -hex 32)
   ```

   This token is intentionally distinct from `ARGO_GATEWAY_BEARER_TOKEN`.
   If the IdP token leaks, the blast radius is provisioning only, not the
   full gateway surface. Until the env var is set, `/scim/v2/Users`
   requests return `503 Service Unavailable`.

2. Verify the endpoint:

   ```sh
   curl -sH "Authorization: Bearer $ARGO_SCIM_BEARER_TOKEN" \
     http://gateway.example.com/scim/v2/Users
   ```

3. Configure the IdP with base URL `https://gateway.example.com/scim/v2`
   and bearer token `ARGO_SCIM_BEARER_TOKEN`.

## IdP Notes

Okta: use HTTP Header authentication with header name `Authorization`
and value `Bearer <ARGO_SCIM_BEARER_TOKEN>`. Enable Push New Users,
Push Profile Updates, and Import New Users. Group sync is deferred.

Azure AD / Entra ID: set Tenant URL to the SCIM base URL and Secret
Token to the SCIM bearer token. Keep the default `customappsso -> User`
mapping and ensure `userName`, `active`, and `displayName` are checked.

Google Workspace: use a third-party SCIM bridge with the SCIM base URL
and bearer token. Disable group sync until the groups endpoint ships.

## Endpoint Reference

| Method | Path | Behavior |
| --- | --- | --- |
| `GET` | `/scim/v2/Users` | List users; supports `?filter=userName eq "x"`, `startIndex`, and `count`. |
| `POST` | `/scim/v2/Users` | Provision a user; returns 201 with the SCIM envelope or 409 on duplicate `userName`. |
| `GET` | `/scim/v2/Users/<id>` | Read a user; returns 404 on unknown or soft-deleted users. |
| `PATCH` | `/scim/v2/Users/<id>` | Partial update; accepts PatchOp envelopes and plain attributes. `active=false` deactivates. |
| `DELETE` | `/scim/v2/Users/<id>` | Soft-delete; returns 204 and writes a tombstone row for audit-chain lookup. |

Every SCIM request flows through the gateway's HTTP request audit path.
The `scim_provisioned_tombstones` table is the fast lookup for deleted
users when an audit row references a `userName` that no longer appears
in `scim_user`.
