# SSO Setup (SAML + OIDC)

The gateway exposes additive SSO middleware for human operators
reaching `/app/*` and `/inbox`. Bearer-token authentication
(`ARGO_GATEWAY_BEARER_TOKEN`) continues to work unchanged. SSO is
the browser sign-in path; bearer auth remains the CLI and automation
path. Both can coexist. When `[sso].enabled = false`, the default, no
`/auth/*` routes are mounted.

Common prerequisites:

- `ARGO_CHANNEL_HMAC_SECRET` must be set. The SSO session cookie uses
  the same HMAC secret as channel approval tokens.
- Configure your IdP with the gateway public origin and pre-register
  the `redirect_uri` for OIDC or `acs_url` for SAML.
- After config changes, check the wire-up with:

```bash
curl -s "$ARGO_GATEWAY_URL/auth/status"
```

Expected shape:

```json
{
  "enabled": true,
  "session_ttl_seconds": 28800,
  "cookie_name": "argo_sso",
  "oidc_tenants": ["default"],
  "saml_tenants": []
}
```

## OIDC: Google Workspace

```toml
[sso]
enabled = true
session_ttl_seconds = 28800
cookie_name = "argo_sso"

[sso.oidc.default]
client_id = "1234567890-abc.apps.googleusercontent.com"
client_secret_env = "ARGO_GOOGLE_OIDC_SECRET"
authorization_endpoint = "https://accounts.google.com/o/oauth2/v2/auth"
token_endpoint = "https://oauth2.googleapis.com/token"
redirect_uri = "https://gw.example.com/auth/callback"
scopes = "openid email profile"
```

```bash
export ARGO_GOOGLE_OIDC_SECRET="GOCSPX-..."
```

## OIDC: Microsoft Entra ID

```toml
[sso.oidc.default]
client_id = "<application-id>"
client_secret_env = "ARGO_ENTRA_OIDC_SECRET"
authorization_endpoint = "https://login.microsoftonline.com/<tenant>/oauth2/v2.0/authorize"
token_endpoint = "https://login.microsoftonline.com/<tenant>/oauth2/v2.0/token"
redirect_uri = "https://gw.example.com/auth/callback"
scopes = "openid email profile"
```

## OIDC: Okta

```toml
[sso.oidc.default]
client_id = "<okta-client-id>"
client_secret_env = "ARGO_OKTA_OIDC_SECRET"
authorization_endpoint = "https://<okta-domain>.okta.com/oauth2/v1/authorize"
token_endpoint = "https://<okta-domain>.okta.com/oauth2/v1/token"
redirect_uri = "https://gw.example.com/auth/callback"
scopes = "openid email profile"
```

## SAML: SP-Initiated

```toml
[sso.saml.default]
idp_entity_id = "https://idp.example.com/saml"
idp_sso_url = "https://idp.example.com/sso"
sp_entity_id = "https://gw.example.com/saml"
acs_url = "https://gw.example.com/auth/saml/default/acs"
idp_cert_pem = """-----BEGIN CERTIFICATE-----
MII...
-----END CERTIFICATE-----"""
```

Login from a browser via `GET /auth/saml/default/login`. The gateway
redirects to the IdP, which POSTs the SAML Response back to the ACS
URL. v1 ships SP-initiated flow only. IdP-initiated SSO is out of
scope.

XML signature verification of the SAML Response is deferred to
`mission-sso-saml-xml-dsig-verify`; the v1 ACS handler parses the
response structurally and does not cryptographically verify it.

## Endpoints

| Route | Method | Purpose |
| --- | --- | --- |
| `/auth/login?tenant=<id>` | GET | OIDC kick-off; redirects to IdP |
| `/auth/callback` | GET | OIDC callback; sets session cookie |
| `/auth/logout` | GET | Clear session cookie |
| `/auth/saml/<tenant>/login` | GET | SAML kick-off; redirects to IdP |
| `/auth/saml/<tenant>/acs` | POST | SAML ACS; sets session cookie |
| `/auth/status` | GET | Read-only summary of `[sso]` config |

Session cookie attributes are `HttpOnly; Secure; SameSite=Lax`. The
cookie value is `<base64url(payload_json)>.<hex(hmac_sha256)>`.
Tamper detection is constant-time and returns `401` on mismatch.
