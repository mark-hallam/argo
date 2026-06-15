# modules/secrets

Placeholder Secrets Manager entries for production env vars the gateway needs.

Provisions one `aws_secretsmanager_secret` per logical key, each:
- encrypted with the customer-managed CMK from `modules/kms` (`kms_key_id = var.kms_key_arn`)
  — WS5-3 hardening; the AWS-managed default key is not used;
- with `recovery_window_in_days = 7` (a fat-finger delete is recoverable for a week, vs the
  AWS 30-day default that blocks name reuse);
- with a sentinel `secret_string = "REPLACE_ME"` and `lifecycle.ignore_changes = [secret_string]`
  so the operator's real values (populated via `aws secretsmanager put-secret-value`) are not
  overwritten on subsequent `terraform apply`.

Keys (logical name → env var the gateway reads):
| key | env var |
|---|---|
| `bearer_token` | `ARGO_GATEWAY_BEARER_TOKEN` |
| `workspace_root_key` | `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` (aka `ARGO_CREDENTIALS_MASTER_KEY`) |
| `auth_jwt_secret` | `SKYCOT_AUTH_JWT_SECRET` |
| `db_key` | `SKYCOT_DB_KEY_BASE64` (SQLCipher at-rest key — required in prod) |
| `stripe_secret_key` | `STRIPE_SECRET_KEY` |
| `stripe_webhook_secret` | `STRIPE_WEBHOOK_SECRET` |
| `stripe_price_ids` | operator record only — NOT read by the gateway (SPA sends the price id via `NEXT_PUBLIC_STRIPE_PRICE_*`) |
| `audit_export_secret` | `ARGO_AUDIT_EXPORT_SECRET` (HMAC for signed audit-export download URLs — required in prod, gateway refuses to boot without it) |
| `audit_screenshot_secret` | `ARGO_AUDIT_SCREENSHOT_SECRET` (HMAC for signed audit-screenshot URLs — required in prod) |

Population is the operator's step — see [`AWS_DEPLOY_RUNBOOK.md`](../../../../docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md)
and the canonical secret list at
[`deploy/secrets/skycot-production.env.example`](../../../../deploy/secrets/skycot-production.env.example).
CMK encryption + recovery window landed in WS5-3
(sprint-ws5-3-prod-secrets-bearer-token-bind-hardening). Secret-rotation automation
(Lambda) is a post-launch follow-up.
