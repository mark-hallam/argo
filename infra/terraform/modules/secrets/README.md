# modules/secrets

Placeholder Secrets Manager entries for production env vars the gateway needs.

Provisions one `aws_secretsmanager_secret` per logical key, each with a sentinel
`secret_string = "REPLACE_ME"` and `lifecycle.ignore_changes = [secret_string]` so the
operator's real values (populated via `aws secretsmanager put-secret-value`) are not
overwritten on subsequent `terraform apply`.

Keys (logical name → env var the gateway reads):
| key | env var |
|---|---|
| `bearer_token` | `ARGO_GATEWAY_BEARER_TOKEN` |
| `workspace_root_key` | `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` (aka `ARGO_CREDENTIALS_MASTER_KEY`) |
| `auth_jwt_secret` | `SKYCOT_AUTH_JWT_SECRET` |
| `stripe_secret_key` | `STRIPE_SECRET_KEY` |
| `stripe_webhook_secret` | `STRIPE_WEBHOOK_SECRET` |
| `stripe_price_ids` | (operator-defined JSON; consumed by argo-billing tier loader) |

Population is the operator's step — see [`AWS_DEPLOY_RUNBOOK.md`](../../../../docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md).
WS5-3 (sprint-ws5-3-prod-secrets-bearer-token-bind-hardening) tightens KMS / rotation /
recovery-window policy.
