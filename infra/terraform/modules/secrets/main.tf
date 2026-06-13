# Production secrets for the SKYCOT gateway.
#
# Every secret here is created with a PLACEHOLDER value ("REPLACE_ME"). The
# operator populates the real value out-of-band via:
#
#   aws secretsmanager put-secret-value --secret-id <id> --secret-string '<value>'
#
# `lifecycle.ignore_changes = [secret_string]` means a subsequent
# `terraform apply` does NOT clobber the populated value.
#
# WS5-3 (sprint-ws5-3-prod-secrets-bearer-token-bind-hardening) will harden
# this further (KMS key, recovery window, rotation).

locals {
  # Each secret declares:
  #  - id    : the SecretsManager id segment (final ARN is .../skycot-prod/<id>)
  #  - desc  : description shown in the SecretsManager console
  secret_specs = {
    bearer_token = {
      id   = "argo-gateway-bearer-token"
      desc = "ARGO_GATEWAY_BEARER_TOKEN — non-loopback bind admin bearer. openssl rand -hex 32."
    }
    workspace_root_key = {
      id   = "skycot-workspace-root-key-base64"
      desc = "SKYCOT_WORKSPACE_ROOT_KEY_BASE64 / ARGO_CREDENTIALS_MASTER_KEY — credential master key. 32 raw bytes, base64-encoded."
    }
    auth_jwt_secret = {
      id   = "skycot-auth-jwt-secret"
      desc = "SKYCOT_AUTH_JWT_SECRET — HMAC secret for __Host-session JWTs. 32+ random bytes."
    }
    db_key = {
      id   = "skycot-db-key-base64"
      desc = "SKYCOT_DB_KEY_BASE64 — SQLCipher at-rest DB encryption key (32 raw bytes, base64). REQUIRED in production: the image must be built --features bundled-sqlcipher AND this set, or the gateway refuses to boot."
    }
    stripe_secret_key = {
      id   = "stripe-secret-key"
      desc = "STRIPE_SECRET_KEY — Stripe live/test secret key."
    }
    stripe_webhook_secret = {
      id   = "stripe-webhook-secret"
      desc = "STRIPE_WEBHOOK_SECRET — endpoint signing secret for /api/v1/billing/webhook."
    }
    stripe_price_ids = {
      id   = "stripe-price-ids"
      desc = "Operator record: JSON map { free, pro, enterprise } -> Stripe price ids matching v1-tiers.toml slugs. NOT read by the gateway (the SPA sends the price id in the checkout body via NEXT_PUBLIC_STRIPE_PRICE_*); kept here as the canonical price-id record."
    }
    smtp_password = {
      id   = "skycot-smtp-password"
      desc = "SKYCOT_SMTP_PASSWORD — the SES (or other SMTP) outbound-email password. Injected into the ECS task as a secret env var via module.ecs (lookup of `smtp_password` in secret_arns); NEVER set in tfvars. The SES SMTP username + host go in email_outbound (plaintext, non-secret)."
    }
  }
}

resource "aws_secretsmanager_secret" "this" {
  for_each = local.secret_specs

  name        = "${var.name_prefix}/${each.value.id}"
  description = each.value.desc

  # Encrypt with the customer-managed CMK (WS5-3 hardening). Scoped decrypt is
  # granted to the task-execution role in the iam module.
  kms_key_id = var.kms_key_arn

  # A fat-finger delete is recoverable for a week, but the name frees fast
  # enough for a re-apply (the AWS default is 30 days).
  recovery_window_in_days = 7
}

resource "aws_secretsmanager_secret_version" "placeholder" {
  for_each = aws_secretsmanager_secret.this

  secret_id     = each.value.id
  secret_string = "REPLACE_ME"

  lifecycle {
    # Operator populates the real value via put-secret-value. Subsequent
    # terraform applies must not overwrite it back to the placeholder.
    ignore_changes = [secret_string]
  }
}
